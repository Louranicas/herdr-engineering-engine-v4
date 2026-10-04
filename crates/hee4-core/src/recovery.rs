//! Startup reconcile, R01–R14 (`gates/features/crash-restart.md`, reconcile table).
//!
//! The shape is v3's (`migrated/v3-b5367bc/src/recovery.rs`): a pure policy over durable facts
//! and caller-supplied observations (`reconcile` `:868-933`; claim checks `:874-900`; terminal
//! history `:955-1027`; running custody `:1028-1108`; settled/boundary `:1147-1272`). The policy
//! reads no clock and no `/proc` (EX-04) and writes nothing; [`reconcile`] feeds each decided
//! event to [`Store::apply`], the one writer. Only R07 and R08 have a task edge in
//! `hee4_contracts::transition`; every other rule leaves the task unchanged and is reported.

use std::collections::BTreeMap;

use hee4_contracts::{Event, Phase, RecoveryRule, TaskId};

use crate::store::{Store, StoreError};

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

/// What the caller observed at startup. The policy consults nothing else.
#[derive(Debug, Clone, Default)]
pub struct Observations {
    /// Custody per task.
    pub process: BTreeMap<TaskId, ProcessCustody>,
    /// Custody for a task not in `process`.
    pub default_process: ProcessCustody,
    /// Claims per task.
    pub claims: BTreeMap<TaskId, Claim>,
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Facts {
    /// The replayed phase.
    pub phase: Phase,
    /// Count of `Dispatch` events.
    pub generation: u64,
    /// A `Dispatch` with no later `Settle` or `Recover(R07|R08)`: an attempt was in flight.
    pub attempt_open: bool,
}

impl Facts {
    /// Derive the facts from a history whose replay gave `phase`.
    #[must_use]
    pub fn from_history(phase: Phase, events: &[Event]) -> Self {
        let mut generation = 0;
        let mut attempt_open = false;
        for e in events {
            match e {
                Event::Dispatch => {
                    generation += 1;
                    attempt_open = true;
                }
                Event::Settle(_)
                | Event::Recover(RecoveryRule::R07ProcessNotOurs | RecoveryRule::R08WorkerAbsent) =>
                {
                    attempt_open = false;
                }
                _ => {}
            }
        }
        Self {
            phase,
            generation,
            attempt_open,
        }
    }
}

/// The policy's answer for one task: which rule fired and the event (if any) to apply.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Decision {
    /// The rule; `None` when the task holds no interrupted attempt (nothing to reconcile).
    pub rule: Option<RecoveryRule>,
    /// The event for `Store::apply`; `None` leaves the task unchanged.
    pub event: Option<Event>,
}

const fn keep(rule: RecoveryRule) -> Decision {
    Decision {
        rule: Some(rule),
        event: None,
    }
}

/// The pure policy. Same inputs, same decision: a second pass converges (make it idempotent).
#[must_use]
pub fn decide(
    ledger_epoch: &str,
    facts: Facts,
    custody: ProcessCustody,
    claim: Option<&Claim>,
) -> Decision {
    use RecoveryRule as R;
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
    match facts.phase {
        Phase::Accepted => keep(R::R04AcceptanceStands),
        Phase::Cancelled => keep(R::R05CancellationStands),
        // v3 `history` returns None for these; the settled attempt goes to cleanup (`:1147`).
        Phase::Failed | Phase::Abandoned | Phase::RepairPending => keep(R::R11CleanupReadback),
        Phase::Verifying => keep(R::R12VerificationBoundary),
        Phase::EffectUnknown { .. } => keep(R::R10EffectAmbiguity),
        Phase::Running | Phase::CancellationRequested if facts.attempt_open => match custody {
            ProcessCustody::LiveSameIdentity => keep(R::R06LiveOwnedChild),
            ProcessCustody::Absent => Decision {
                rule: Some(R::R08WorkerAbsent),
                event: Some(Event::Recover(R::R08WorkerAbsent)),
            },
            ProcessCustody::PidReused | ProcessCustody::Unreadable | ProcessCustody::Unobserved => {
                Decision {
                    rule: Some(R::R07ProcessNotOurs),
                    event: Some(Event::Recover(R::R07ProcessNotOurs)),
                }
            }
        },
        // `running` without an open attempt cannot come out of `transition`.
        Phase::Running => keep(R::R14UnexpectedState),
        Phase::Admitted | Phase::CancellationRequested | Phase::Blocked { .. } => Decision {
            rule: None,
            event: None,
        },
    }
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
        let (history, phase) = match store.history(&task).and_then(|h| {
            let p = store.phase(&task)?;
            Ok((h, p))
        }) {
            Ok((h, Some(p))) => (h, p),
            Ok((_, None)) => {
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
        let facts = Facts::from_history(phase, &history);
        let cache_ok = store.cached(&task)?.is_some_and(|c| {
            c.phase == phase.as_str()
                && c.cancel == phase.cancel_requested()
                && c.generation == facts.generation
        });
        if !cache_ok {
            report.findings.push(Finding::CacheMismatch {
                task_id: task.clone(),
            });
        }
        let decision = decide(
            &epoch,
            facts,
            observed.custody(&task),
            observed.claims.get(&task),
        );
        if decision.rule == Some(RecoveryRule::R14UnexpectedState) {
            report.findings.push(Finding::Unexpected {
                task_id: task.clone(),
            });
        }
        let after = match decision.event {
            None => phase,
            Some(event) => match store.apply(&task, event) {
                Ok(after) => {
                    report.applied += 1;
                    after
                }
                Err(StoreError::Refused(r)) => {
                    report.findings.push(Finding::Refused {
                        task_id: task.clone(),
                        detail: r.to_string(),
                    });
                    phase
                }
                Err(e) => return Err(e),
            },
        };
        report.rows.push(Row {
            task_id: task,
            rule: decision.rule,
            before: phase,
            after,
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
            let d = decide("e", facts(phase, open), custody, None);
            assert_eq!(
                (d.rule, d.event.is_some()),
                (rule, moves),
                "{phase:?} {custody:?}"
            );
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
        let d = decide("e", facts(Phase::Running, true), C::Absent, Some(&stale));
        assert_eq!(d, keep(R::R01StaleObservationEpoch));
        let old_gen = Claim {
            epoch: "e".into(),
            task_generation: 0,
        };
        let d = decide("e", facts(Phase::Running, true), C::Absent, Some(&old_gen));
        assert_eq!(d, keep(R::R02StaleGeneration));
    }
}
