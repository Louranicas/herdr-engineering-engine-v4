//! The attempts ledger (`attempts`, migration `m005_attempts`; V4-94, V4-95 item 3). One row
//! per `Dispatch`, whose lifecycle columns are a function of the task's events (opened by
//! `Dispatch`, closed by `Settle` or `Recover(R07|R08)` inside `apply_in`'s one transaction,
//! through [`on_event`]) plus the runtime facts the events do not hold (receipt, permit, model,
//! head, workspace, lease, pid + start ticks), written only through the named `Store` verbs
//! here: [`Store::attempt_started`], [`Store::attempt_pid`], [`Store::attempt_cleanup_settled`].
//! No verb here writes `events` or `tasks`.
//!
//! State map §2b: attempt `state` {running, settled, unknown}, `effect` {none, pending,
//! committed, unknown}, `cleanup` {none, pending, settled, unknown}. IA-1 (settled → running)
//! is the trigger `attempts_no_reopen`; IA-3 (a second `Dispatch` on a running attempt) is
//! `UNIQUE (task_id, generation)` plus `transition`, which gives `Running` no `Dispatch` edge.
//! Pre-migration histories are never backfilled: an events-open attempt with no row is
//! `R08Reason::AcknowledgementUnrecorded` in `recovery::decide`.
//!
//! # Schema (`m005_attempts`)
//!
//! ```text
//! attempts(id PK 'a-<task>-<generation>', task_id FK, generation >= 1, dispatch_seq FK events,
//!          receipt_id, permit_id, model, head_sha (40 hex), workspace, started_ms,
//!          deadline_ms, clock_epoch, pid, pid_start_ticks,
//!          state IN (running, settled, unknown), effect IN (none, pending, committed, unknown),
//!          cleanup IN (none, pending, settled, unknown), closed_seq FK events,
//!          outcome IN (ready, not_ready, unsettled, r07, r08, stopped, abandoned),
//!          UNIQUE (task_id, generation)) STRICT
//! attempts_open: partial index on task_id WHERE state = 'running'
//! attempts_no_delete   BEFORE DELETE                     → 'attempts are append-only'
//! attempts_no_rewrite  BEFORE UPDATE OF task_id, generation, dispatch_seq, started_ms
//!                                                        → 'attempt identity is immutable'
//! attempts_no_reopen   BEFORE UPDATE WHEN OLD.state != 'running' AND NEW.state = 'running'
//!                                                        → 'IA-1: a closed attempt never reopens'
//! meta attempts_from_seq = max(events.seq) when m005 applied (pre-ledger dispatches)
//! ```
//!
//! # Event hook (inside `apply_in`, after the events INSERT)
//!
//! | Event | Row |
//! |---|---|
//! | `Dispatch` | INSERT `a-<task>-<generation>`: running / none / none, `dispatch_seq` = the event's seq |
//! | `Settle(Ready)` / `Settle(NotReady)` | the running row → settled, outcome `ready` / `not_ready`, cleanup pending, `closed_seq` = seq |
//! | `Settle(Unsettled)` / `Recover(R07)` / `Recover(R08)` | the running row → unknown, effect unknown, cleanup pending, outcome `unsettled` / `r07` / `r08`, `closed_seq` = seq |
//! | `Stop` / `Resolve(Abandon)` | the running row, if any → unknown, effect unknown, cleanup pending, outcome `stopped` / `abandoned`, `closed_seq` = seq; none is a no-op (both are legal with no attempt in flight) |
//! | `Resolve(Quarantine)` and anything else | no-op (a quarantined attempt may still `Settle`) |
//!
//! A `Settle` / `Recover` that finds no running row is [`StoreError::AttemptMissing`]: the
//! whole transaction rolls back and the event is not written (fail closed). The one exception
//! is a pre-ledger attempt: no row exists for `(task, generation)` and its `Dispatch` seq is at
//! or below `meta.attempts_from_seq`; its close writes the event and no row.
//!
//! `attempt_started` writes the start facts once (an identical repeat is a no-op, other facts
//! are [`StoreError::AttemptAcknowledged`]) and takes only an absolute, normal workspace
//! ([`StoreError::WorkspaceNotNormal`]); S2 refuses a workspace equal to, inside or containing
//! another unsettled attempt's ([`StoreError::WorkspaceLeased`]).
//!
//! # Reconcile rows this table enables (`recovery.rs`)
//!
//! R03 (a row that contradicts the task's events is a `Finding::Contradictory` and withholds
//! `recovery_complete`), R08 reason classes (acknowledged = `receipt_id` written), R09
//! (workspace + lease read back per row), R06/R07 (pid + start ticks persisted here, read by
//! `probe.rs`), R11 (`cleanup` settled only by readback, through `attempt_cleanup_settled`).

use std::fmt;
use std::path::PathBuf;
use std::str::FromStr;

use hee4_contracts::{
    Event, GitSha, ReceiptId, RecoveryRule, Refusal, Resolution, Settlement, TaskId,
};
use rusqlite::{Connection, OptionalExtension, Transaction, params};

use super::migrations::Migration;
use super::{Store, StoreError, corrupt, load_events_with_seq, meta_get, meta_set};

/// The `m005_attempts` DDL, applied once by `Store::open`.
pub(crate) const SCHEMA: &str = "
CREATE TABLE attempts(
  id TEXT PRIMARY KEY NOT NULL,
  task_id TEXT NOT NULL REFERENCES tasks(id),
  generation INTEGER NOT NULL CHECK (generation >= 1),
  dispatch_seq INTEGER NOT NULL REFERENCES events(seq),
  receipt_id TEXT,
  permit_id INTEGER,
  model TEXT,
  head_sha TEXT CHECK (head_sha IS NULL OR length(head_sha) = 40),
  workspace TEXT,
  started_ms INTEGER NOT NULL,
  deadline_ms INTEGER,
  clock_epoch TEXT,
  pid INTEGER,
  pid_start_ticks INTEGER,
  state TEXT NOT NULL CHECK (state IN ('running','settled','unknown')),
  effect TEXT NOT NULL CHECK (effect IN ('none','pending','committed','unknown')),
  cleanup TEXT NOT NULL CHECK (cleanup IN ('none','pending','settled','unknown')),
  closed_seq INTEGER REFERENCES events(seq),
  outcome TEXT CHECK (outcome IS NULL OR outcome IN ('ready','not_ready','unsettled','r07','r08',
    'stopped','abandoned')),
  UNIQUE (task_id, generation)
) STRICT;
CREATE INDEX attempts_open ON attempts(task_id) WHERE state = 'running';
CREATE TRIGGER attempts_no_delete BEFORE DELETE ON attempts
  BEGIN SELECT RAISE(ABORT, 'attempts are append-only'); END;
CREATE TRIGGER attempts_no_rewrite BEFORE UPDATE OF task_id, generation, dispatch_seq, started_ms
  ON attempts
  BEGIN SELECT RAISE(ABORT, 'attempt identity is immutable'); END;
CREATE TRIGGER attempts_no_reopen BEFORE UPDATE ON attempts
  WHEN OLD.state != 'running' AND NEW.state = 'running'
  BEGIN SELECT RAISE(ABORT, 'IA-1: a closed attempt never reopens'); END;
";

/// The one `MIGRATIONS` entry of this family.
pub(crate) const MIGRATION: Migration = Migration {
    name: "m005_attempts",
    apply: migrate,
};

/// `meta` key: the highest `events.seq` when `m005_attempts` was applied. A `Dispatch` at or
/// below it predates the attempts ledger and has no row (never backfilled).
pub(crate) const FROM_SEQ_KEY: &str = "attempts_from_seq";

fn migrate(tx: &Transaction<'_>) -> Result<(), StoreError> {
    tx.execute_batch(SCHEMA)?;
    let high: i64 = tx.query_row("SELECT coalesce(max(seq), 0) FROM events", [], |r| r.get(0))?;
    meta_set(tx, FROM_SEQ_KEY, &high.to_string())?;
    Ok(())
}

const ROW_COLUMNS: &str = "id, task_id, generation, dispatch_seq, receipt_id, permit_id, model,
       head_sha, workspace, started_ms, deadline_ms, clock_epoch, pid, pid_start_ticks, state,
       effect, cleanup, closed_seq, outcome";

/// Why a string is not an [`AttemptId`].
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AttemptIdFault {
    /// The text does not start with `a-`.
    #[error("an attempt id starts with `a-`")]
    MissingPrefix,
    /// No `-<generation>` tail.
    #[error("an attempt id ends with `-<generation>`")]
    MissingGeneration,
    /// The generation is not a positive integer.
    #[error("generation {0:?} is not a positive integer")]
    Generation(String),
    /// The task part is not a `TaskId`.
    #[error("task part: {0}")]
    Task(Refusal),
}

/// An attempt's id, `a-<task>-<generation>`: one per `Dispatch` of a task.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AttemptId {
    task: TaskId,
    generation: u64,
}

impl AttemptId {
    /// The id of `task`'s attempt number `generation` (1 for the first `Dispatch`).
    #[must_use]
    pub fn new(task: &TaskId, generation: u64) -> Self {
        Self {
            task: task.clone(),
            generation,
        }
    }

    /// The task.
    #[must_use]
    pub fn task_id(&self) -> &TaskId {
        &self.task
    }

    /// The generation (count of `Dispatch` events up to and including this attempt's).
    #[must_use]
    pub fn generation(&self) -> u64 {
        self.generation
    }
}

impl fmt::Display for AttemptId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "a-{}-{}", self.task, self.generation)
    }
}

impl FromStr for AttemptId {
    type Err = AttemptIdFault;
    fn from_str(s: &str) -> Result<Self, AttemptIdFault> {
        let body = s.strip_prefix("a-").ok_or(AttemptIdFault::MissingPrefix)?;
        let (task, generation) = body
            .rsplit_once('-')
            .ok_or(AttemptIdFault::MissingGeneration)?;
        let generation: u64 = generation
            .parse()
            .ok()
            .filter(|g| *g >= 1)
            .ok_or_else(|| AttemptIdFault::Generation(generation.to_owned()))?;
        let task = task.parse().map_err(AttemptIdFault::Task)?;
        Ok(Self { task, generation })
    }
}

macro_rules! spelled {
    ($(#[$doc:meta])* $name:ident { $($(#[$vdoc:meta])* $variant:ident = $text:literal),+ $(,)? }) => {
        $(#[$doc])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum $name {
            $($(#[$vdoc])* $variant,)+
        }

        impl $name {
            /// The column spelling.
            #[must_use]
            pub const fn as_str(self) -> &'static str {
                match self {
                    $(Self::$variant => $text,)+
                }
            }

            fn from_column(text: &str) -> Option<Self> {
                match text {
                    $($text => Some(Self::$variant),)+
                    _ => None,
                }
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(self.as_str())
            }
        }
    };
}

spelled!(
    /// `attempts.state` (State map §2b).
    AttemptState {
        /// Opened by `Dispatch`, not yet closed.
        Running = "running",
        /// Closed by `Settle(Ready | NotReady)`.
        Settled = "settled",
        /// Closed by `Settle(Unsettled)`, `Recover(R07 | R08)`, `Stop` or `Resolve(Abandon)`:
        /// the effect is unknown.
        Unknown = "unknown",
    }
);

spelled!(
    /// `attempts.effect`: what the attempt did to the world.
    Effect {
        /// Nothing committed.
        None = "none",
        /// An effect is in flight.
        Pending = "pending",
        /// Committed.
        Committed = "committed",
        /// Cannot be known (the attempt closed unknown).
        Unknown = "unknown",
    }
);

spelled!(
    /// `attempts.cleanup`: the workspace's disposal.
    Cleanup {
        /// Nothing to clean yet (the attempt is running).
        None = "none",
        /// Closed; the workspace awaits R11's readback.
        Pending = "pending",
        /// Settled by readback (`attempt_cleanup_settled`), never by elapsed time.
        Settled = "settled",
        /// Unknown.
        Unknown = "unknown",
    }
);

spelled!(
    /// `attempts.outcome`: which event closed the row.
    AttemptOutcome {
        /// `Settle(Ready)`.
        Ready = "ready",
        /// `Settle(NotReady)`.
        NotReady = "not_ready",
        /// `Settle(Unsettled)`.
        Unsettled = "unsettled",
        /// `Recover(R07ProcessNotOurs)`.
        R07 = "r07",
        /// `Recover(R08WorkerAbsent)`.
        R08 = "r08",
        /// `Stop` (`CancellationRequested` → `Cancelled`) over a running attempt.
        Stopped = "stopped",
        /// `Resolve(Abandon)` over a running attempt.
        Abandoned = "abandoned",
    }
);

/// A workspace lease: `deadline_ms` is meaningful only in the receiver clock epoch that issued
/// it (R09: a lease is compared only in its own epoch; expiry alone never licenses reuse).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lease {
    /// The deadline in the issuing clock, ms.
    pub deadline_ms: i64,
    /// The issuing clock's epoch (the host's `boot_id`).
    pub clock_epoch: String,
}

/// The runtime facts the dispatcher records once the attempt is acknowledged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttemptStart {
    /// The receipt the attempt will seal.
    pub receipt_id: ReceiptId,
    /// The permit it was dispatched under.
    pub permit_id: u64,
    /// The model it runs on.
    pub model: String,
    /// The commit it builds on.
    pub head_sha: GitSha,
    /// Its fresh workspace (S2: never another live attempt's).
    pub workspace: PathBuf,
    /// Its workspace lease, if one was issued.
    pub lease: Option<Lease>,
}

/// One `attempts` row as read back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttemptRow {
    /// `a-<task>-<generation>`.
    pub id: AttemptId,
    /// The task.
    pub task_id: TaskId,
    /// Count of `Dispatch` events up to and including this attempt's.
    pub generation: u64,
    /// The seq of the `Dispatch` event that opened the row.
    pub dispatch_seq: i64,
    /// The acknowledgement facts; `None` until `attempt_started`.
    pub started: Option<AttemptStart>,
    /// The `Dispatch` event's `ts` (ms).
    pub started_ms: i64,
    /// The last recorded `(pid, start_ticks)` of the worker.
    pub pid: Option<(u32, u64)>,
    /// Lifecycle state.
    pub state: AttemptState,
    /// Effect state.
    pub effect: Effect,
    /// Cleanup state.
    pub cleanup: Cleanup,
    /// The seq of the closing event, once closed.
    pub closed_seq: Option<i64>,
    /// Which event closed the row, once closed.
    pub outcome: Option<AttemptOutcome>,
}

impl AttemptRow {
    /// `attempt_started` was recorded: the worker acknowledged the dispatch.
    #[must_use]
    pub fn acknowledged(&self) -> bool {
        self.started.is_some()
    }
}

/// A row as SQLite hands it over, before its ids, enums and paths are parsed.
struct RawRow {
    id: String,
    task_id: String,
    generation: i64,
    dispatch_seq: i64,
    receipt_id: Option<String>,
    permit_id: Option<i64>,
    model: Option<String>,
    head_sha: Option<String>,
    workspace: Option<String>,
    started_ms: i64,
    deadline_ms: Option<i64>,
    clock_epoch: Option<String>,
    pid: Option<i64>,
    pid_start_ticks: Option<i64>,
    state: String,
    effect: String,
    cleanup: String,
    closed_seq: Option<i64>,
    outcome: Option<String>,
}

impl RawRow {
    fn from_row(r: &rusqlite::Row<'_>) -> Result<Self, rusqlite::Error> {
        Ok(Self {
            id: r.get(0)?,
            task_id: r.get(1)?,
            generation: r.get(2)?,
            dispatch_seq: r.get(3)?,
            receipt_id: r.get(4)?,
            permit_id: r.get(5)?,
            model: r.get(6)?,
            head_sha: r.get(7)?,
            workspace: r.get(8)?,
            started_ms: r.get(9)?,
            deadline_ms: r.get(10)?,
            clock_epoch: r.get(11)?,
            pid: r.get(12)?,
            pid_start_ticks: r.get(13)?,
            state: r.get(14)?,
            effect: r.get(15)?,
            cleanup: r.get(16)?,
            closed_seq: r.get(17)?,
            outcome: r.get(18)?,
        })
    }

    fn parse(self) -> Result<AttemptRow, StoreError> {
        let task_id: TaskId = self.task_id.parse().map_err(|e| StoreError::Corrupt {
            task: self.task_id.clone(),
            detail: format!("attempts.task_id: {e}"),
        })?;
        let bad = |detail: String| corrupt(&task_id, detail);
        let id: AttemptId = self
            .id
            .parse()
            .map_err(|e| bad(format!("attempts.id {:?}: {e}", self.id)))?;
        let generation = u64::try_from(self.generation)
            .map_err(|_| bad(format!("attempts.generation {}", self.generation)))?;
        let started = match (
            self.receipt_id,
            self.permit_id,
            self.model,
            self.head_sha,
            self.workspace,
        ) {
            (None, None, None, None, None) => None,
            (Some(receipt), Some(permit), Some(model), Some(head), Some(workspace)) => {
                let lease = match (self.deadline_ms, self.clock_epoch) {
                    (None, None) => None,
                    (Some(deadline_ms), Some(clock_epoch)) => Some(Lease {
                        deadline_ms,
                        clock_epoch,
                    }),
                    _ => return Err(bad("attempts lease is half written".into())),
                };
                Some(AttemptStart {
                    receipt_id: receipt
                        .parse()
                        .map_err(|e| bad(format!("attempts.receipt_id: {e}")))?,
                    permit_id: u64::try_from(permit)
                        .map_err(|_| bad(format!("attempts.permit_id {permit}")))?,
                    model,
                    head_sha: head
                        .parse()
                        .map_err(|e| bad(format!("attempts.head_sha: {e}")))?,
                    workspace: PathBuf::from(workspace),
                    lease,
                })
            }
            _ => return Err(bad("attempts start facts are half written".into())),
        };
        let pid = match (self.pid, self.pid_start_ticks) {
            (None, None) => None,
            (Some(pid), Some(ticks)) => Some((
                u32::try_from(pid).map_err(|_| bad(format!("attempts.pid {pid}")))?,
                u64::try_from(ticks)
                    .map_err(|_| bad(format!("attempts.pid_start_ticks {ticks}")))?,
            )),
            _ => return Err(bad("attempts pid identity is half written".into())),
        };
        let state = AttemptState::from_column(&self.state)
            .ok_or_else(|| bad(format!("attempts.state {:?}", self.state)))?;
        let effect = Effect::from_column(&self.effect)
            .ok_or_else(|| bad(format!("attempts.effect {:?}", self.effect)))?;
        let cleanup = Cleanup::from_column(&self.cleanup)
            .ok_or_else(|| bad(format!("attempts.cleanup {:?}", self.cleanup)))?;
        let outcome = self
            .outcome
            .map(|o| {
                AttemptOutcome::from_column(&o)
                    .ok_or_else(|| bad(format!("attempts.outcome {o:?}")))
            })
            .transpose()?;
        Ok(AttemptRow {
            id,
            task_id,
            generation,
            dispatch_seq: self.dispatch_seq,
            started,
            started_ms: self.started_ms,
            pid,
            state,
            effect,
            cleanup,
            closed_seq: self.closed_seq,
            outcome,
        })
    }
}

fn rows(
    conn: &Connection,
    where_order: &str,
    args: impl rusqlite::Params,
) -> Result<Vec<AttemptRow>, StoreError> {
    let sql = format!("SELECT {ROW_COLUMNS} FROM attempts {where_order}");
    let mut stmt = conn.prepare(&sql)?;
    let raw = stmt
        .query_map(args, RawRow::from_row)?
        .collect::<Result<Vec<_>, _>>()?;
    raw.into_iter().map(RawRow::parse).collect()
}

/// The row's state, `None` when the id is unknown.
fn state_of(conn: &Connection, id: &AttemptId) -> Result<Option<AttemptState>, StoreError> {
    let text: Option<String> = conn
        .query_row(
            "SELECT state FROM attempts WHERE id = ?1",
            [id.to_string()],
            |r| r.get(0),
        )
        .optional()?;
    text.map(|t| {
        AttemptState::from_column(&t)
            .ok_or_else(|| corrupt(id.task_id(), format!("attempts.state {t:?}")))
    })
    .transpose()
}

/// The workspace as the ledger stores it: UTF-8, absolute, no `.`/`..`/empty component and no
/// trailing `/`, so that equal directories are equal strings and overlap is a prefix test.
fn normal_workspace(path: &std::path::Path) -> Result<&str, StoreError> {
    let refuse = || StoreError::WorkspaceNotNormal {
        workspace: path.to_path_buf(),
    };
    let text = path.to_str().ok_or_else(refuse)?;
    let body = text.strip_prefix('/').ok_or_else(refuse)?;
    if body.is_empty()
        || body
            .split('/')
            .any(|c| c.is_empty() || c == "." || c == "..")
    {
        return Err(refuse());
    }
    Ok(text)
}

/// Refuse unless the row exists and is running.
fn require_running(conn: &Connection, id: &AttemptId) -> Result<(), StoreError> {
    match state_of(conn, id)? {
        None => Err(StoreError::AttemptMissing {
            task: id.task_id().clone(),
            generation: id.generation(),
        }),
        Some(AttemptState::Running) => Ok(()),
        Some(state) => Err(StoreError::AttemptClosed {
            id: id.clone(),
            state,
        }),
    }
}

/// The attempts half of `apply_in`, run after the events INSERT with `seq` = that event's
/// seq, `generation` = the task's count of `Dispatch` events including `event`, `ts` = the
/// event's `ts`. `Dispatch` opens the row; `Settle` and `Recover(R07|R08)` close the running
/// one; every other event is a no-op.
///
/// # Errors
/// [`StoreError::AttemptMissing`] when a close finds no running row (the caller's transaction
/// rolls back); SQLite's constraint error for a second row of one generation (IA-3).
pub(crate) fn on_event(
    tx: &Transaction<'_>,
    task: &TaskId,
    event: Event,
    seq: i64,
    generation: i64,
    ts: i64,
) -> Result<(), StoreError> {
    let generation =
        u64::try_from(generation).map_err(|_| corrupt(task, "generation is negative"))?;
    let (state, effect, outcome) = match event {
        Event::Dispatch => {
            tx.execute(
                "INSERT INTO attempts(id, task_id, generation, dispatch_seq, started_ms, state,
                   effect, cleanup)
                 VALUES (?1, ?2, ?3, ?4, ?5, 'running', 'none', 'none')",
                params![
                    AttemptId::new(task, generation).to_string(),
                    task.as_str(),
                    i64::try_from(generation).map_err(|_| corrupt(task, "generation overflow"))?,
                    seq,
                    ts
                ],
            )?;
            return Ok(());
        }
        Event::Settle(Settlement::Ready) => (AttemptState::Settled, None, AttemptOutcome::Ready),
        Event::Settle(Settlement::NotReady) => {
            (AttemptState::Settled, None, AttemptOutcome::NotReady)
        }
        Event::Settle(Settlement::Unsettled) => (
            AttemptState::Unknown,
            Some(Effect::Unknown),
            AttemptOutcome::Unsettled,
        ),
        Event::Recover(RecoveryRule::R07ProcessNotOurs) => (
            AttemptState::Unknown,
            Some(Effect::Unknown),
            AttemptOutcome::R07,
        ),
        Event::Recover(RecoveryRule::R08WorkerAbsent) => (
            AttemptState::Unknown,
            Some(Effect::Unknown),
            AttemptOutcome::R08,
        ),
        Event::Stop => (
            AttemptState::Unknown,
            Some(Effect::Unknown),
            AttemptOutcome::Stopped,
        ),
        Event::Resolve(Resolution::Abandon(_)) => (
            AttemptState::Unknown,
            Some(Effect::Unknown),
            AttemptOutcome::Abandoned,
        ),
        // `Resolve(Quarantine)` leaves the row running: `transition` keeps `(Blocked, Settle)`,
        // so the quarantined attempt may still settle, and a later `Resolve(Abandon)` closes it.
        _ => return Ok(()),
    };
    let changed = tx.execute(
        "UPDATE attempts
         SET state = ?1, effect = coalesce(?2, effect), cleanup = 'pending', outcome = ?3,
             closed_seq = ?4
         WHERE task_id = ?5 AND state = 'running'",
        params![
            state.as_str(),
            effect.map(Effect::as_str),
            outcome.as_str(),
            seq,
            task.as_str()
        ],
    )?;
    if changed > 0 {
        return Ok(());
    }
    // `Stop` and `Resolve(Abandon)` are legal with no attempt in flight (from `Admitted`,
    // `Verifying`, `RepairPending`, `EffectUnknown`, `Blocked`, or after a `Settle`): closing
    // nothing is correct for them.
    if matches!(outcome, AttemptOutcome::Stopped | AttemptOutcome::Abandoned) {
        return Ok(());
    }
    if pre_ledger_attempt(tx, task, generation)? {
        return Ok(());
    }
    Err(StoreError::AttemptMissing {
        task: task.clone(),
        generation,
    })
}

/// The attempt `(task, generation)` was dispatched before `m005_attempts` was applied: no row
/// exists for it and its `Dispatch` seq is at or below `attempts_from_seq`. Its close writes the
/// event and no row (`AcknowledgementUnrecorded`); every other row-less close fails closed.
fn pre_ledger_attempt(
    tx: &Transaction<'_>,
    task: &TaskId,
    generation: u64,
) -> Result<bool, StoreError> {
    let Some(from_seq) = meta_get(tx, FROM_SEQ_KEY)?.and_then(|v| v.parse::<i64>().ok()) else {
        return Ok(false);
    };
    let has_row: bool = tx.query_row(
        "SELECT EXISTS(SELECT 1 FROM attempts WHERE task_id = ?1 AND generation = ?2)",
        params![
            task.as_str(),
            i64::try_from(generation).map_err(|_| corrupt(task, "generation overflow"))?
        ],
        |r| r.get(0),
    )?;
    if has_row {
        return Ok(false);
    }
    let dispatch_seq = load_events_with_seq(tx, task)?
        .into_iter()
        .filter(|(_, e)| *e == Event::Dispatch)
        .map(|(seq, _)| seq)
        .next_back();
    Ok(dispatch_seq.is_some_and(|seq| seq <= from_seq))
}

impl Store {
    /// Record the acknowledgement facts of a running attempt: receipt, permit, model, head,
    /// workspace and lease, written once. The workspace must be an absolute, normal path that
    /// neither equals, contains nor lies inside the workspace of another attempt whose cleanup
    /// is not settled (S2: a fresh workspace per attempt). An identical repeat is a no-op.
    ///
    /// # Errors
    /// [`StoreError::WorkspaceNotNormal`] for a relative path or one with `.`, `..`, an empty
    /// component or a trailing `/`; [`StoreError::AttemptMissing`] for an unknown id;
    /// [`StoreError::AttemptClosed`] when the row is not running (IA-3);
    /// [`StoreError::AttemptAcknowledged`] when other start facts are already recorded;
    /// [`StoreError::WorkspaceLeased`] when another unsettled attempt's workspace overlaps.
    pub fn attempt_started(&self, id: &AttemptId, start: &AttemptStart) -> Result<(), StoreError> {
        let workspace = normal_workspace(&start.workspace)?;
        let permit = i64::try_from(start.permit_id).map_err(|_| {
            corrupt(
                id.task_id(),
                format!("permit_id {} overflows", start.permit_id),
            )
        })?;
        let tx = self.begin()?;
        require_running(&tx, id)?;
        if let Some(recorded) = rows(&tx, "WHERE id = ?1", [id.to_string()])?
            .pop()
            .and_then(|r| r.started)
        {
            if recorded == *start {
                return Ok(());
            }
            return Err(StoreError::AttemptAcknowledged { id: id.clone() });
        }
        let holder: Option<String> = tx
            .query_row(
                "SELECT id FROM attempts
                 WHERE id != ?2 AND cleanup != 'settled' AND workspace IS NOT NULL
                   AND (workspace = ?1
                        OR substr(?1, 1, length(workspace) + 1) = workspace || '/'
                        OR substr(workspace, 1, length(?1) + 1) = ?1 || '/')
                 LIMIT 1",
                params![workspace, id.to_string()],
                |r| r.get(0),
            )
            .optional()?;
        if let Some(other) = holder {
            let other: AttemptId = other
                .parse()
                .map_err(|e| corrupt(id.task_id(), format!("attempts.id {other:?}: {e}")))?;
            return Err(StoreError::WorkspaceLeased {
                workspace: start.workspace.clone(),
                other,
            });
        }
        tx.execute(
            "UPDATE attempts
             SET receipt_id = ?1, permit_id = ?2, model = ?3, head_sha = ?4, workspace = ?5,
                 deadline_ms = ?6, clock_epoch = ?7
             WHERE id = ?8 AND state = 'running' AND receipt_id IS NULL",
            params![
                start.receipt_id.as_str(),
                permit,
                start.model,
                start.head_sha.as_str(),
                workspace,
                start.lease.as_ref().map(|l| l.deadline_ms),
                start.lease.as_ref().map(|l| l.clock_epoch.as_str()),
                id.to_string()
            ],
        )?;
        tx.commit()?;
        Ok(())
    }

    /// Record the worker's process identity. The row keeps the last pid written: an earlier
    /// pid is a reaped child.
    ///
    /// # Errors
    /// [`StoreError::AttemptMissing`]; [`StoreError::AttemptClosed`].
    pub fn attempt_pid(
        &self,
        id: &AttemptId,
        pid: u32,
        start_ticks: u64,
    ) -> Result<(), StoreError> {
        let ticks = i64::try_from(start_ticks)
            .map_err(|_| corrupt(id.task_id(), format!("start_ticks {start_ticks} overflows")))?;
        let tx = self.begin()?;
        require_running(&tx, id)?;
        tx.execute(
            "UPDATE attempts SET pid = ?1, pid_start_ticks = ?2 WHERE id = ?3 AND state = 'running'",
            params![i64::from(pid), ticks, id.to_string()],
        )?;
        tx.commit()?;
        Ok(())
    }

    /// R11: the workspace was read back clean; settle the row's cleanup. Only a readback
    /// settles cleanup, never elapsed time.
    ///
    /// # Errors
    /// [`StoreError::AttemptMissing`]; [`StoreError::AttemptOpen`] while the row is running.
    pub fn attempt_cleanup_settled(&self, id: &AttemptId) -> Result<(), StoreError> {
        let tx = self.begin()?;
        match state_of(&tx, id)? {
            None => {
                return Err(StoreError::AttemptMissing {
                    task: id.task_id().clone(),
                    generation: id.generation(),
                });
            }
            Some(AttemptState::Running) => {
                return Err(StoreError::AttemptOpen { id: id.clone() });
            }
            Some(_) => {}
        }
        tx.execute(
            "UPDATE attempts SET cleanup = 'settled' WHERE id = ?1",
            [id.to_string()],
        )?;
        tx.commit()?;
        Ok(())
    }

    /// The row with `id`, if any.
    ///
    /// # Errors
    /// SQLite errors; [`StoreError::Corrupt`] for an unparsable row.
    pub fn attempt(&self, id: &AttemptId) -> Result<Option<AttemptRow>, StoreError> {
        Ok(rows(&self.conn, "WHERE id = ?1", [id.to_string()])?.pop())
    }

    /// Every attempt of `task`, generation order.
    ///
    /// # Errors
    /// SQLite errors; [`StoreError::Corrupt`] for an unparsable row.
    pub fn attempts(&self, task: &TaskId) -> Result<Vec<AttemptRow>, StoreError> {
        rows(
            &self.conn,
            "WHERE task_id = ?1 ORDER BY generation",
            [task.as_str()],
        )
    }

    /// Every running row, by task id then generation.
    ///
    /// # Errors
    /// SQLite errors; [`StoreError::Corrupt`] for an unparsable row.
    pub fn open_attempts(&self) -> Result<Vec<AttemptRow>, StoreError> {
        rows(
            &self.conn,
            "WHERE state = 'running' ORDER BY task_id, generation",
            [],
        )
    }

    /// `task`'s highest-generation row, if any.
    ///
    /// # Errors
    /// SQLite errors; [`StoreError::Corrupt`] for an unparsable row.
    pub fn latest_attempt(&self, task: &TaskId) -> Result<Option<AttemptRow>, StoreError> {
        Ok(rows(
            &self.conn,
            "WHERE task_id = ?1 ORDER BY generation DESC LIMIT 1",
            [task.as_str()],
        )?
        .pop())
    }
}

#[cfg(test)]
mod tests {
    //! A real foundation-era file (opened with `m001`..`m004` only, never a DROP simulation)
    //! holding attempts dispatched before the attempts ledger existed.

    use super::super::migrations::MIGRATIONS;
    use super::*;
    use crate::codec;
    use crate::recovery::{Observations, R08Reason, reconcile};
    use hee4_contracts::Phase;

    type R = Result<(), Box<dyn std::error::Error>>;

    /// Write `Admit` + `Dispatch` for `task` as a binary without the attempts hook did: the
    /// `tasks` row and two `events` rows, no `attempts` row.
    fn plant_running(conn: &Connection, task: &str) -> R {
        conn.execute(
            "INSERT INTO tasks(id, phase, cancel, generation, updated_ts) VALUES (?1, 'running', 0, 1, 1)",
            [task],
        )?;
        for event in [Event::Admit, Event::Dispatch] {
            conn.execute(
                "INSERT INTO events(task_id, event_json, ts) VALUES (?1, ?2, 1)",
                params![task, codec::encode(event)?],
            )?;
        }
        Ok(())
    }

    #[test]
    fn a_pre_ledger_open_attempt_recovers_and_settles() -> R {
        let dir = std::env::temp_dir().join(format!("hee4-core-attempts-{}", std::process::id()));
        std::fs::create_dir_all(&dir)?;
        let path = dir.join("pre-ledger.sqlite");
        for suffix in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{}{suffix}", path.display()));
        }
        {
            let old = Store::open_with(&path, &MIGRATIONS[..4])?;
            assert_eq!(old.schema_version()?, 4);
            assert!(meta_get(&old.conn, "migration:m005_attempts")?.is_none());
            plant_running(&old.conn, "lost")?;
            plant_running(&old.conn, "settles")?;
        }
        let store = Store::open(&path)?;
        assert_eq!(
            meta_get(&store.conn, FROM_SEQ_KEY)?.as_deref(),
            Some("4"),
            "the high-water seq at m005"
        );
        let lost: TaskId = "lost".parse()?;
        let settles: TaskId = "settles".parse()?;
        assert_eq!(store.attempts(&lost)?, Vec::new(), "never backfilled");

        // A Settle of a pre-ledger attempt is accepted: the event is written, no row.
        assert_eq!(
            store.apply(&settles, Event::Settle(Settlement::Ready))?,
            Phase::Verifying
        );
        assert_eq!(store.attempts(&settles)?, Vec::new());

        let report = reconcile(&store, &Observations::worker_absent())?;
        let row = report
            .rows
            .iter()
            .find(|r| r.task_id == lost)
            .ok_or("lost row")?;
        assert_eq!(
            (row.rule, row.reason, row.after),
            (
                Some(RecoveryRule::R08WorkerAbsent),
                Some(R08Reason::AcknowledgementUnrecorded),
                Phase::EffectUnknown { cancel: false }
            )
        );
        assert_eq!(
            (report.applied, report.complete),
            (1, true),
            "{:?}",
            report.findings
        );
        assert!(store.recovery_complete()?);
        assert_eq!(store.attempts(&lost)?, Vec::new());

        // A post-ledger Dispatch with no row (planted past the hook) still fails closed.
        plant_running(&store.conn, "planted")?;
        let planted: TaskId = "planted".parse()?;
        let before = store.event_count()?;
        let err = store.apply(&planted, Event::Settle(Settlement::Ready));
        assert!(
            matches!(&err, Err(StoreError::AttemptMissing { task, generation: 1 }) if *task == planted),
            "{err:?}"
        );
        assert_eq!(store.event_count()?, before);

        // And a fresh Dispatch after the migration opens a row as usual.
        let fresh: TaskId = "fresh".parse()?;
        store.apply(&fresh, Event::Admit)?;
        store.apply(&fresh, Event::Dispatch)?;
        assert_eq!(store.attempts(&fresh)?.len(), 1);
        Ok(())
    }
}
