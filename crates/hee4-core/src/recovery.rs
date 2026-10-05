//! Startup reconcile, R01–R14 (`gates/features/crash-restart.md`, reconcile table).
//!
//! The shape is v3's (`migrated/v3-b5367bc/src/recovery.rs`): a pure policy over durable facts
//! and caller-supplied observations (`reconcile` `:868-933`; claim checks `:874-900`; terminal
//! history `:955-1027`; running custody `:1028-1108`; settled/boundary `:1147-1272`). The policy
//! reads no clock and no process table (EX-04) and writes nothing; [`reconcile`] feeds each decided
//! event to [`Store::apply`], the one writer; [`crate::probe`] is the only IO. Only R07 and R08
//! have a task edge in `hee4_contracts::transition`; every other rule leaves the task unchanged
//! and is reported.
//!
//! # The attempts rows (`src/store/attempts.rs`)
//!
//! | Durable facts | Observation | Rule | Event | After |
//! |---|---|---|---|---|
//! | the latest `attempts` row contradicts the events: closed with no / a wrong `closed_seq`, an outcome that disagrees with the closing event, or running while a close follows its `dispatch_seq` | any | R03 | — (`Finding::Contradictory`; a contradiction cannot be transitioned away) | unchanged; `complete=false`, the engine does not listen, the operator restores |
//! | running, attempt open, row with pid | `LiveSameIdentity` (probe: the process start ticks equal the row's, its state is not `Z`/`X`, and the row's lease `clock_epoch`, when present, is this boot's `boot_id`; otherwise `Absent`) | R06 | — (observe-only, no reaper) | unchanged |
//! | running, attempt open | `Absent` | R08 + reason: row unacknowledged → `DispatchUnacknowledged`; acknowledged → `AcknowledgedWorkerLost`; no row (pre-migration history) → `AcknowledgementUnrecorded` | `Recover(R08)` | effect_unknown |
//! | running, attempt open | `PidReused` / `Unreadable` / `Unobserved` (a row with no pid probes `Unobserved`) | R07 + the same acknowledgement class | `Recover(R07)` | effect_unknown |
//! | verifying, nothing sealed: no `Decide` after the latest `Dispatch`, receipts ≤ `Decide`s | any | R12 (no verdict exists to copy) | `Resolve(Quarantine(EffectUnknownPermanent{R12}))` | blocked |
//! | verifying, receipts > `Decide`s (a legacy settle sealed, then stopped) | any | R12 + [`SealedStep::DecideFromSeal`] ([`Row::sealed`]) | `Decide(<the latest sealed receipt's verdict, copied>)`, then `Accept` on a `Pass`, one transaction ([`Store::finish_sealed`]) | the phase the verdict names: accepted / repair_pending / failed / effect_unknown |
//! | verifying, a `Decide(Pass)` after the latest `Dispatch`, no `Accept` | any | R12 + [`SealedStep::AcceptAfterPass`] | `Accept` | accepted |
//! | terminal phase | latest row still `running` | R03 (a `Stop` or `Resolve(Abandon)` closes the row `stopped` / `abandoned`; a running row under a terminal phase contradicts) | — | unchanged; `complete=false` |
//! | R07/R08 over an open row, or R11 over a closed row with cleanup pending | `Writable{bytes}` workspace | R09 attached beside the rule: no lease → `NotLeasedWritable`; lease, no clock → `ClockUnavailable`; other clock epoch → `LeaseClockNotComparable`; same epoch, past deadline → `LeaseExpiredWritable`; same epoch, live → nothing | none for the attachment | unchanged; cleanup stays pending |
//! | ledger `epoch`, `event_high_water`, `restored_from` · a cursor | — | R13 ([`cursor`], not a task rule) | — | `PriorEpochOfRestore` / `EpochChanged` / `FutureSequence` carry `R13CursorEpoch`; `SnapshotOnly` carries none; never a replay authorisation |

use std::collections::BTreeMap;
use std::path::PathBuf;

use hee4_contracts::{
    Event, Phase, QuarantineReason, RecoveryRule, Resolution, Settlement, TaskId,
};

use crate::store::{
    AttemptId, AttemptOutcome, AttemptRow, AttemptState, Cleanup, CursorVerdict, Lease, Store,
    StoreError,
};
pub use crate::store::{SealedFinish, SealedStep, sealed_step};

/// A worker's physical custody as the caller observed it (v3 `ProcessCustody`, `:520-532`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ProcessCustody {
    /// The same process identity is alive and ours (R06: re-attach, never redispatch).
    LiveSameIdentity,
    /// The pid is alive but a different process (R07, permanent).
    PidReused,
    /// No such process: the `kill -KILL` case (R08).
    Absent,
    /// Could not look (R07, transient).
    Unreadable,
    /// No process identity was observed; never means absent (R07, transient).
    #[default]
    Unobserved,
}

/// A claim an observation carries about which ledger it was taken against (R01, R02).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Claim {
    /// The ledger epoch the observer saw.
    pub epoch: String,
    /// The task generation (count of `Dispatch`) the observer saw.
    pub task_generation: u64,
}

/// What the probe read back of an attempt's workspace (R09).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WorkspaceReadback {
    /// The path does not exist.
    Absent,
    /// A directory is there; `bytes` is the walked size, capped at the caller's bound.
    Writable {
        /// Bytes found before the walk stopped.
        bytes: u64,
    },
    /// Could not look.
    Unreadable,
    /// Nothing was read.
    #[default]
    Unobserved,
}

/// The receiver clock the probe read, in which a lease's `deadline_ms` may be compared.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Clock {
    /// Now, ms since the Unix epoch.
    pub now_ms: i64,
    /// This boot's clock epoch (`boot_id`).
    pub clock_epoch: String,
}

/// What the caller observed at startup. The policy consults nothing else.
#[derive(Debug, Clone, Default)]
pub struct Observations {
    /// Custody per task.
    pub process: BTreeMap<TaskId, ProcessCustody>,
    /// Custody for a task not in `process`.
    pub default_process: ProcessCustody,
    /// Claims per task.
    pub claims: BTreeMap<TaskId, Claim>,
    /// Workspace readback per attempt (absent: `Unobserved`).
    pub workspace: BTreeMap<AttemptId, WorkspaceReadback>,
    /// The receiver clock, if readable.
    pub clock: Option<Clock>,
}

impl Observations {
    /// Every worker observed absent (the state after the whole engine was killed with `SIGKILL`).
    #[must_use]
    pub fn worker_absent() -> Self {
        Self {
            default_process: ProcessCustody::Absent,
            ..Self::default()
        }
    }

    fn custody(&self, task: &TaskId) -> ProcessCustody {
        self.process
            .get(task)
            .copied()
            .unwrap_or(self.default_process)
    }
}

/// The durable facts the policy reads for one task, derived from its events.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Facts {
    /// The replayed phase.
    pub phase: Phase,
    /// Count of `Dispatch` events.
    pub generation: u64,
    /// A `Dispatch` with no later close (`Settle`, `Recover(R07|R08)`, `Stop`,
    /// `Resolve(Abandon)`): an attempt was in flight.
    pub attempt_open: bool,
    /// Every closing event (`Settle(_)`, `Recover(R07|R08)`, `Stop`, `Resolve(Abandon)`) with
    /// its `events.seq`, for R03.
    pub closes: Vec<(i64, Event)>,
    /// No `Decide` follows the latest `Dispatch` and the chain holds no receipt beyond the
    /// `Decide` count: the verification of the latest attempt was never sealed, so there is no
    /// verdict to re-issue (the dispatcher seals one receipt, then applies one `Decide`).
    pub verdict_unsealed: bool,
}

impl Facts {
    /// Derive the facts from a history (with seqs) whose replay gave `phase` and the count of
    /// receipts its chain holds.
    #[must_use]
    pub fn from_history(phase: Phase, events: &[(i64, Event)], receipts: u64) -> Self {
        let mut generation = 0;
        let mut attempt_open = false;
        let mut closes = Vec::new();
        let mut decides = 0_u64;
        let mut decided_since_dispatch = false;
        for (seq, e) in events {
            match e {
                Event::Dispatch => {
                    generation += 1;
                    attempt_open = true;
                    decided_since_dispatch = false;
                }
                Event::Decide(_) => {
                    decides += 1;
                    decided_since_dispatch = true;
                }
                Event::Settle(_)
                | Event::Stop
                | Event::Resolve(Resolution::Abandon(_))
                | Event::Recover(RecoveryRule::R07ProcessNotOurs | RecoveryRule::R08WorkerAbsent) =>
                {
                    attempt_open = false;
                    closes.push((*seq, *e));
                }
                _ => {}
            }
        }
        Self {
            phase,
            generation,
            attempt_open,
            closes,
            verdict_unsealed: !decided_since_dispatch && receipts <= decides,
        }
    }
}

/// The latest `attempts` row of a task, as the policy reads it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttemptFacts {
    /// The attempt.
    pub id: AttemptId,
    /// Its generation.
    pub generation: u64,
    /// The seq of its `Dispatch` event.
    pub dispatch_seq: i64,
    /// Lifecycle state.
    pub state: AttemptState,
    /// `attempt_started` was recorded.
    pub acknowledged: bool,
    /// The last recorded worker identity.
    pub pid: Option<(u32, u64)>,
    /// The workspace, once acknowledged.
    pub workspace: Option<PathBuf>,
    /// The lease, if one was issued.
    pub lease: Option<Lease>,
    /// Cleanup state.
    pub cleanup: Cleanup,
    /// The closing event's seq, once closed.
    pub closed_seq: Option<i64>,
    /// The closing outcome, once closed.
    pub outcome: Option<AttemptOutcome>,
}

impl AttemptFacts {
    /// The facts of a row.
    #[must_use]
    pub fn from_row(row: &AttemptRow) -> Self {
        Self {
            id: row.id.clone(),
            generation: row.generation,
            dispatch_seq: row.dispatch_seq,
            state: row.state,
            acknowledged: row.acknowledged(),
            pid: row.pid,
            workspace: row.started.as_ref().map(|s| s.workspace.clone()),
            lease: row.started.as_ref().and_then(|s| s.lease.clone()),
            cleanup: row.cleanup,
            closed_seq: row.closed_seq,
            outcome: row.outcome,
        }
    }
}

/// The acknowledgement class of the lost attempt, carried by R08 and by R07 (named for R08,
/// the brief's spelling).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum R08Reason {
    /// A row with no `attempt_started`: the worker never acknowledged.
    DispatchUnacknowledged,
    /// An acknowledged row: the worker was seen, then lost.
    AcknowledgedWorkerLost,
    /// No row at all: a history older than the attempts ledger.
    AcknowledgementUnrecorded,
}

/// R09: why a writable workspace is not reused (attached beside the rule, never instead).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkspaceReuseRefused {
    /// Writable with no lease.
    NotLeasedWritable,
    /// Leased, but no receiver clock to compare in.
    ClockUnavailable,
    /// The lease was issued in another clock epoch: not comparable, expired or not.
    LeaseClockNotComparable,
    /// Same clock epoch, past the deadline: expiry alone never licenses reuse.
    LeaseExpiredWritable,
}

/// The policy's answer for one task: which rule fired and the event (if any) to apply.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Decision {
    /// The rule; `None` when the task holds no interrupted attempt (nothing to reconcile).
    pub rule: Option<RecoveryRule>,
    /// The event for `Store::apply`; `None` leaves the task unchanged.
    pub event: Option<Event>,
    /// R07/R08's acknowledgement class.
    pub reason: Option<R08Reason>,
    /// R09's attachment.
    pub workspace: Option<WorkspaceReuseRefused>,
    /// R03: the attempt row and the events disagree; no event is ever applied.
    pub contradiction: bool,
}

const fn keep(rule: RecoveryRule) -> Decision {
    Decision {
        rule: Some(rule),
        event: None,
        reason: None,
        workspace: None,
        contradiction: false,
    }
}

const fn apply(rule: RecoveryRule) -> Decision {
    Decision {
        rule: Some(rule),
        event: Some(Event::Recover(rule)),
        reason: None,
        workspace: None,
        contradiction: false,
    }
}

const NOTHING: Decision = Decision {
    rule: None,
    event: None,
    reason: None,
    workspace: None,
    contradiction: false,
};

/// The outcome a closing event writes on the row.
const fn outcome_of(event: Event) -> Option<AttemptOutcome> {
    match event {
        Event::Settle(Settlement::Ready) => Some(AttemptOutcome::Ready),
        Event::Settle(Settlement::NotReady) => Some(AttemptOutcome::NotReady),
        Event::Settle(Settlement::Unsettled) => Some(AttemptOutcome::Unsettled),
        Event::Recover(RecoveryRule::R07ProcessNotOurs) => Some(AttemptOutcome::R07),
        Event::Recover(RecoveryRule::R08WorkerAbsent) => Some(AttemptOutcome::R08),
        Event::Stop => Some(AttemptOutcome::Stopped),
        Event::Resolve(Resolution::Abandon(_)) => Some(AttemptOutcome::Abandoned),
        _ => None,
    }
}

/// R03: does the row contradict the events? (a) closed with no `closed_seq`, or one that is
/// not a closing event after its `dispatch_seq`; (b) an outcome that disagrees with that
/// event; (c) running while a closing event follows its `dispatch_seq`, or while the task is
/// terminal. (`Blocked` with a running row is legal: `Resolve(Quarantine)` does not close the
/// attempt, and `(Blocked, Settle)` is an edge.)
fn contradicts(attempt: &AttemptFacts, facts: &Facts) -> bool {
    let after_dispatch = |seq: i64| seq > attempt.dispatch_seq;
    if attempt.state == AttemptState::Running {
        return facts.phase.is_terminal()
            || facts.closes.iter().any(|(seq, _)| after_dispatch(*seq));
    }
    let Some(closed_seq) = attempt.closed_seq.filter(|s| after_dispatch(*s)) else {
        return true;
    };
    let Some((_, event)) = facts.closes.iter().find(|(seq, _)| *seq == closed_seq) else {
        return true;
    };
    attempt.outcome.is_none() || attempt.outcome != outcome_of(*event)
}

/// R09 over a writable workspace.
fn lease_refusal(lease: Option<&Lease>, clock: Option<&Clock>) -> Option<WorkspaceReuseRefused> {
    let Some(lease) = lease else {
        return Some(WorkspaceReuseRefused::NotLeasedWritable);
    };
    let Some(clock) = clock else {
        return Some(WorkspaceReuseRefused::ClockUnavailable);
    };
    if clock.clock_epoch != lease.clock_epoch {
        return Some(WorkspaceReuseRefused::LeaseClockNotComparable);
    }
    (clock.now_ms > lease.deadline_ms).then_some(WorkspaceReuseRefused::LeaseExpiredWritable)
}

/// The pure policy. Same inputs, same decision: a second pass converges (make it idempotent).
/// Reads no clock and no process table: `clock` and `workspace` are what the caller observed.
#[must_use]
pub fn decide(
    ledger_epoch: &str,
    facts: &Facts,
    attempt: Option<&AttemptFacts>,
    custody: ProcessCustody,
    workspace: WorkspaceReadback,
    clock: Option<&Clock>,
    claim: Option<&Claim>,
) -> Decision {
    use RecoveryRule as R;
    if let Some(a) = attempt
        && contradicts(a, facts)
    {
        return Decision {
            contradiction: true,
            ..keep(R::R03CommitOrdering)
        };
    }
    if !facts.phase.is_terminal()
        && let Some(claim) = claim
    {
        if claim.epoch != ledger_epoch {
            return keep(R::R01StaleObservationEpoch);
        }
        if claim.task_generation != facts.generation {
            return keep(R::R02StaleGeneration);
        }
    }
    let mut decision = match facts.phase {
        Phase::Accepted => keep(R::R04AcceptanceStands),
        Phase::Cancelled => keep(R::R05CancellationStands),
        // v3 `history` returns None for these; the settled attempt goes to cleanup (`:1147`).
        Phase::Failed | Phase::Abandoned | Phase::RepairPending => keep(R::R11CleanupReadback),
        // R12 with nothing sealed: no receipt and no `Decide` for the latest attempt, so no
        // verdict exists to copy. The task is quarantined by name; the operator's
        // `Resolve(Abandon)` is the exit from `blocked`.
        Phase::Verifying if facts.verdict_unsealed => Decision {
            event: Some(Event::Resolve(Resolution::Quarantine(
                QuarantineReason::EffectUnknownPermanent {
                    rule: R::R12VerificationBoundary,
                },
            ))),
            ..keep(R::R12VerificationBoundary)
        },
        // R12 with a seal: `reconcile` finishes it through `Store::finish_sealed`, which copies
        // the sealed verdict (never a judgement here); the policy itself applies nothing.
        Phase::Verifying => keep(R::R12VerificationBoundary),
        Phase::EffectUnknown { .. } => keep(R::R10EffectAmbiguity),
        Phase::Running | Phase::CancellationRequested if facts.attempt_open => {
            let reason = Some(match attempt {
                None => R08Reason::AcknowledgementUnrecorded,
                Some(a) if a.acknowledged => R08Reason::AcknowledgedWorkerLost,
                Some(_) => R08Reason::DispatchUnacknowledged,
            });
            match custody {
                ProcessCustody::LiveSameIdentity => keep(R::R06LiveOwnedChild),
                ProcessCustody::Absent => Decision {
                    reason,
                    ..apply(R::R08WorkerAbsent)
                },
                ProcessCustody::PidReused
                | ProcessCustody::Unreadable
                | ProcessCustody::Unobserved => Decision {
                    reason,
                    ..apply(R::R07ProcessNotOurs)
                },
            }
        }
        // `running` without an open attempt cannot come out of `transition`.
        Phase::Running => keep(R::R14UnexpectedState),
        Phase::Admitted | Phase::CancellationRequested | Phase::Blocked { .. } => NOTHING,
    };
    if let (Some(a), WorkspaceReadback::Writable { .. }) = (attempt, workspace) {
        let worker_gone = matches!(
            decision.rule,
            Some(R::R07ProcessNotOurs | R::R08WorkerAbsent)
        ) && a.state == AttemptState::Running;
        let cleanup_due = decision.rule == Some(R::R11CleanupReadback)
            && a.state != AttemptState::Running
            && a.cleanup == Cleanup::Pending;
        if worker_gone || cleanup_due {
            decision.workspace = lease_refusal(a.lease.as_ref(), clock);
        }
    }
    decision
}

/// R13 over a subscriber cursor: the foundation's verdict plus the rule it carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CursorDecision {
    /// `Store::cursor_check`'s verdict.
    pub verdict: CursorVerdict,
    /// `R13CursorEpoch` for every verdict but `SnapshotOnly`.
    pub rule: Option<RecoveryRule>,
}

/// R13: not a task rule. Reads `Store::cursor_check` and names the rule; never a replay
/// authorisation.
///
/// # Errors
/// SQLite errors.
pub fn cursor(store: &Store, epoch: &str, seq: u64) -> Result<CursorDecision, StoreError> {
    let verdict = store.cursor_check(epoch, seq)?;
    let rule = match verdict {
        CursorVerdict::PriorEpochOfRestore
        | CursorVerdict::EpochChanged
        | CursorVerdict::FutureSequence => Some(RecoveryRule::R13CursorEpoch),
        CursorVerdict::SnapshotOnly => None,
    };
    Ok(CursorDecision { verdict, rule })
}

/// One task's reconcile outcome.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    /// The task.
    pub task_id: TaskId,
    /// The rule that fired (`None`: nothing in flight).
    pub rule: Option<RecoveryRule>,
    /// Phase before the pass.
    pub before: Phase,
    /// Phase after the pass.
    pub after: Phase,
    /// R07/R08's acknowledgement class.
    pub reason: Option<R08Reason>,
    /// R09's attachment.
    pub workspace: Option<WorkspaceReuseRefused>,
    /// R12's convergence, when this pass finished a sealed `verifying` task
    /// ([`SealedStep::DecideFromSeal`] or [`SealedStep::AcceptAfterPass`]).
    pub sealed: Option<SealedStep>,
}

/// A reason `complete` is withheld.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Finding {
    /// The task's history is unreadable or does not replay (R14, EX-05).
    Unreadable {
        /// The task.
        task_id: String,
        /// What failed.
        detail: String,
    },
    /// The `tasks` cache disagrees with the replay of `events` (R14).
    CacheMismatch {
        /// The task.
        task_id: TaskId,
    },
    /// The policy chose R14 for this task.
    Unexpected {
        /// The task.
        task_id: TaskId,
    },
    /// `transition` refused the policy's event (a policy/whitelist disagreement).
    Refused {
        /// The task.
        task_id: TaskId,
        /// The refusal text.
        detail: String,
    },
    /// The task's latest `attempts` row contradicts its events (R03); no event is applied and
    /// the operator restores.
    Contradictory {
        /// The task.
        task_id: TaskId,
        /// The row.
        attempt_id: AttemptId,
        /// `R03CommitOrdering`.
        rule: RecoveryRule,
    },
}

/// The result of [`reconcile`].
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RecoveryReport {
    /// One row per readable task.
    pub rows: Vec<Row>,
    /// Why `complete` is false, if it is.
    pub findings: Vec<Finding>,
    /// Events applied this pass.
    pub applied: usize,
    /// No findings: `recovery_complete` was set and dispatch may start.
    pub complete: bool,
}

/// The `tasks` cache row agrees with the replay (R14's cache check).
fn cache_matches(store: &Store, task: &TaskId, facts: &Facts) -> Result<bool, StoreError> {
    Ok(store.cached(task)?.is_some_and(|c| {
        c.phase == facts.phase.as_str()
            && c.cancel == facts.phase.cancel_requested()
            && c.generation == facts.generation
    }))
}

/// Write `decision` for `task` through the store's doors: its event via [`Store::apply`], or,
/// for R12 over a sealed verification, [`Store::finish_sealed`] (the sealed verdict copied, and
/// `Accept` on a `Pass`, in one transaction). A refusal or an unreadable row is a [`Finding`];
/// returns the phase after and R12's convergence step, if one fired.
fn carry_out(
    store: &Store,
    task: &TaskId,
    phase: Phase,
    decision: Decision,
    report: &mut RecoveryReport,
) -> Result<(Phase, Option<SealedStep>), StoreError> {
    let result = match decision.event {
        Some(event) => store.apply(task, event).map(|after| {
            report.applied += 1;
            (after, None)
        }),
        None if decision.rule == Some(RecoveryRule::R12VerificationBoundary)
            && !decision.contradiction =>
        {
            store.finish_sealed(task).map(|finish| {
                finish.map_or((phase, None), |f| {
                    report.applied += f.events;
                    (f.phase, Some(f.step))
                })
            })
        }
        None => Ok((phase, None)),
    };
    match result {
        Ok(done) => Ok(done),
        Err(StoreError::Refused(r)) => {
            report.findings.push(Finding::Refused {
                task_id: task.clone(),
                detail: r.to_string(),
            });
            Ok((phase, None))
        }
        Err(StoreError::Corrupt { task, detail }) if decision.event.is_none() => {
            report.findings.push(Finding::Unreadable {
                task_id: task,
                detail,
            });
            Ok((phase, None))
        }
        Err(e) => Err(e),
    }
}

/// Apply R01–R14 to every task through [`Store::apply`], then set `recovery_complete` to
/// whether the pass had no findings. Idempotent: a second pass applies nothing.
///
/// # Errors
/// SQLite errors only; every per-task problem is a [`Finding`].
pub fn reconcile(store: &Store, observed: &Observations) -> Result<RecoveryReport, StoreError> {
    store.set_recovery_complete(false)?;
    let epoch = store.epoch()?;
    let mut report = RecoveryReport::default();
    let ids = match store.task_ids() {
        Ok(ids) => ids,
        Err(StoreError::Corrupt { task, detail }) => {
            report.findings.push(Finding::Unreadable {
                task_id: task,
                detail,
            });
            return Ok(report);
        }
        Err(e) => return Err(e),
    };
    for task in ids {
        let (history, phase, latest) = match store.history_with_seq(&task).and_then(|h| {
            let p = store.phase(&task)?;
            let a = store.latest_attempt(&task)?;
            Ok((h, p, a))
        }) {
            Ok((h, Some(p), a)) => (h, p, a),
            Ok((_, None, _)) => {
                report.findings.push(Finding::Unreadable {
                    task_id: task.to_string(),
                    detail: "tasks row with no events".into(),
                });
                continue;
            }
            Err(StoreError::Corrupt { task, detail }) => {
                report.findings.push(Finding::Unreadable {
                    task_id: task,
                    detail,
                });
                continue;
            }
            Err(e) => return Err(e),
        };
        let facts = Facts::from_history(phase, &history, store.receipt_count(&task)?);
        let attempt = latest.as_ref().map(AttemptFacts::from_row);
        if !cache_matches(store, &task, &facts)? {
            report.findings.push(Finding::CacheMismatch {
                task_id: task.clone(),
            });
        }
        let workspace = attempt
            .as_ref()
            .and_then(|a| observed.workspace.get(&a.id).copied())
            .unwrap_or_default();
        let decision = decide(
            &epoch,
            &facts,
            attempt.as_ref(),
            observed.custody(&task),
            workspace,
            observed.clock.as_ref(),
            observed.claims.get(&task),
        );
        if decision.rule == Some(RecoveryRule::R14UnexpectedState) {
            report.findings.push(Finding::Unexpected {
                task_id: task.clone(),
            });
        }
        if let (true, Some(a), Some(rule)) = (decision.contradiction, &attempt, decision.rule) {
            report.findings.push(Finding::Contradictory {
                task_id: task.clone(),
                attempt_id: a.id.clone(),
                rule,
            });
        }
        let (after, sealed) = carry_out(store, &task, phase, decision, &mut report)?;
        report.rows.push(Row {
            task_id: task,
            rule: decision.rule,
            before: phase,
            after,
            reason: decision.reason,
            workspace: decision.workspace,
            sealed,
        });
    }
    report.complete = report.findings.is_empty();
    store.set_recovery_complete(report.complete)?;
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn facts(phase: Phase, attempt_open: bool) -> Facts {
        Facts {
            phase,
            generation: 1,
            attempt_open,
            closes: Vec::new(),
            verdict_unsealed: false,
        }
    }

    fn plain(
        ledger_epoch: &str,
        facts: &Facts,
        custody: ProcessCustody,
        claim: Option<&Claim>,
    ) -> Decision {
        decide(
            ledger_epoch,
            facts,
            None,
            custody,
            WorkspaceReadback::Unobserved,
            None,
            claim,
        )
    }

    /// R12 quarantines only a verification nothing sealed: a receipt beyond the `Decide` count
    /// (sealed, not yet decided) or a `Decide` after the latest `Dispatch` keeps the task.
    #[test]
    fn r12_quarantines_only_an_unsealed_verification() {
        use hee4_contracts::Verdict;
        let ready = Event::Settle(Settlement::Ready);
        let settled = [(1, Event::Admit), (2, Event::Dispatch), (3, ready)];
        let mut passed = settled.to_vec();
        passed.push((4, Event::Decide(Verdict::Pass)));
        let mut repaired = settled.to_vec();
        repaired.extend([
            (4, Event::Decide(Verdict::Fail)),
            (5, Event::Dispatch),
            (6, ready),
        ]);
        let quarantine = Some(Event::Resolve(Resolution::Quarantine(
            QuarantineReason::EffectUnknownPermanent {
                rule: RecoveryRule::R12VerificationBoundary,
            },
        )));
        for (history, receipts, event) in [
            (&settled[..], 0, quarantine),
            (&settled[..], 1, None),
            (&passed[..], 1, None),
            (&passed[..], 0, None),
            (&repaired[..], 1, quarantine),
            (&repaired[..], 2, None),
        ] {
            let facts = Facts::from_history(Phase::Verifying, history, receipts);
            let d = plain("e", &facts, ProcessCustody::Absent, None);
            assert_eq!(
                (d.rule, d.event),
                (Some(RecoveryRule::R12VerificationBoundary), event),
                "{history:?} receipts={receipts}"
            );
        }
    }

    #[test]
    fn policy_table() {
        use ProcessCustody as C;
        use RecoveryRule as R;
        let cases = [
            (
                Phase::Running,
                true,
                C::Absent,
                Some(R::R08WorkerAbsent),
                true,
            ),
            (
                Phase::Running,
                true,
                C::PidReused,
                Some(R::R07ProcessNotOurs),
                true,
            ),
            (
                Phase::Running,
                true,
                C::Unobserved,
                Some(R::R07ProcessNotOurs),
                true,
            ),
            (
                Phase::Running,
                true,
                C::LiveSameIdentity,
                Some(R::R06LiveOwnedChild),
                false,
            ),
            (
                Phase::CancellationRequested,
                true,
                C::Absent,
                Some(R::R08WorkerAbsent),
                true,
            ),
            (Phase::CancellationRequested, false, C::Absent, None, false),
            (
                Phase::Accepted,
                false,
                C::Absent,
                Some(R::R04AcceptanceStands),
                false,
            ),
            (
                Phase::Cancelled,
                false,
                C::Absent,
                Some(R::R05CancellationStands),
                false,
            ),
            (
                Phase::Verifying,
                false,
                C::Absent,
                Some(R::R12VerificationBoundary),
                false,
            ),
            (
                Phase::EffectUnknown { cancel: false },
                false,
                C::Absent,
                Some(R::R10EffectAmbiguity),
                false,
            ),
            (
                Phase::RepairPending,
                false,
                C::Absent,
                Some(R::R11CleanupReadback),
                false,
            ),
            (
                Phase::Running,
                false,
                C::Absent,
                Some(R::R14UnexpectedState),
                false,
            ),
            (Phase::Admitted, false, C::Absent, None, false),
        ];
        for (phase, open, custody, rule, moves) in cases {
            let d = plain("e", &facts(phase, open), custody, None);
            assert_eq!(
                (d.rule, d.event.is_some()),
                (rule, moves),
                "{phase:?} {custody:?}"
            );
            assert!(!d.contradiction);
        }
    }

    #[test]
    fn stale_claims_are_refused_before_custody() {
        use ProcessCustody as C;
        use RecoveryRule as R;
        let stale = Claim {
            epoch: "other".into(),
            task_generation: 1,
        };
        let d = plain("e", &facts(Phase::Running, true), C::Absent, Some(&stale));
        assert_eq!(d, keep(R::R01StaleObservationEpoch));
        let old_gen = Claim {
            epoch: "e".into(),
            task_generation: 0,
        };
        let d = plain("e", &facts(Phase::Running, true), C::Absent, Some(&old_gen));
        assert_eq!(d, keep(R::R02StaleGeneration));
    }
}
