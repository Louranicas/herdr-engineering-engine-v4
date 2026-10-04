//! The ledger. The one file in this crate that writes SQL (`FLOW.md`, enforced by
//! `tests/one_door.rs`).
//!
//! `events` is the source of truth; `tasks` is a cache of `TaskState::replay(events)` written in
//! the same transaction as the event that changed it. [`Store::apply`] is the only state writer:
//! a refused transition rolls back and writes nothing. Every commit runs under
//! `journal_mode=WAL` + `synchronous=FULL`, so the WAL is fsynced before `apply`, `admit`,
//! `record_observation` or `append_receipt` returns (the ack point, task.submit "fsync before
//! ack").

use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use hee4_contracts::{
    ChainBreak, Event, Observation, ObservationId, Phase, Receipt, Refusal, Sha256Hex, TaskId,
    TaskState, transition,
};
use rusqlite::{Connection, OptionalExtension, Transaction, TransactionBehavior, params};
use serde_json::Value;

use crate::codec;

/// The schema version [`Store::open`] migrates to.
pub const SCHEMA_VERSION: i64 = 2;

const SCHEMA_V1: &str = "
CREATE TABLE tasks(
  id TEXT PRIMARY KEY NOT NULL,
  phase TEXT NOT NULL CHECK (phase IN ('admitted','running','verifying','repair_pending',
    'cancellation_requested','blocked','effect_unknown','accepted','failed','cancelled',
    'abandoned')),
  cancel INTEGER NOT NULL CHECK (cancel IN (0, 1)),
  generation INTEGER NOT NULL CHECK (generation >= 0),
  updated_ts INTEGER NOT NULL
) STRICT;
CREATE TABLE events(
  seq INTEGER PRIMARY KEY AUTOINCREMENT,
  task_id TEXT NOT NULL REFERENCES tasks(id),
  event_json TEXT NOT NULL,
  ts INTEGER NOT NULL
) STRICT;
CREATE INDEX events_by_task ON events(task_id, seq);
CREATE TRIGGER events_no_update BEFORE UPDATE ON events
  BEGIN SELECT RAISE(ABORT, 'events are append-only'); END;
CREATE TRIGGER events_no_delete BEFORE DELETE ON events
  BEGIN SELECT RAISE(ABORT, 'events are append-only'); END;
CREATE TABLE operations(
  principal TEXT NOT NULL,
  action TEXT NOT NULL,
  version INTEGER NOT NULL,
  idem_key TEXT NOT NULL,
  request_sha256 TEXT NOT NULL CHECK (length(request_sha256) = 64),
  task_id TEXT NOT NULL REFERENCES tasks(id),
  result_json TEXT NOT NULL,
  ts INTEGER NOT NULL,
  UNIQUE (principal, action, version, idem_key)
) STRICT;
CREATE TABLE observations(
  id TEXT PRIMARY KEY NOT NULL,
  task_id TEXT NOT NULL REFERENCES tasks(id),
  json TEXT NOT NULL,
  ts INTEGER NOT NULL
) STRICT;
CREATE TABLE receipts(
  seq INTEGER PRIMARY KEY AUTOINCREMENT,
  id TEXT NOT NULL UNIQUE,
  task_id TEXT NOT NULL REFERENCES tasks(id),
  json TEXT NOT NULL,
  hash_prev TEXT NOT NULL CHECK (length(hash_prev) = 64),
  hash_self TEXT NOT NULL UNIQUE CHECK (length(hash_self) = 64),
  ts INTEGER NOT NULL,
  UNIQUE (task_id, hash_prev)
) STRICT;
CREATE INDEX receipts_by_task ON receipts(task_id, seq);
CREATE TRIGGER receipts_no_update BEFORE UPDATE ON receipts
  BEGIN SELECT RAISE(ABORT, 'receipts are append-only'); END;
CREATE TRIGGER receipts_no_delete BEFORE DELETE ON receipts
  BEGIN SELECT RAISE(ABORT, 'receipts are append-only'); END;
CREATE TABLE meta(
  key TEXT PRIMARY KEY NOT NULL,
  value TEXT NOT NULL
) STRICT;
";

/// v2: `apply_in` records every time it overwrote a `tasks.phase` that disagreed with replay.
const SCHEMA_V2: &str = "
CREATE TABLE cache_heals(
  seq INTEGER PRIMARY KEY AUTOINCREMENT,
  task_id TEXT NOT NULL REFERENCES tasks(id),
  cached_phase TEXT NOT NULL,
  replayed_phase TEXT NOT NULL,
  ts INTEGER NOT NULL
) STRICT;
";

/// Why a store call wrote nothing.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum StoreError {
    /// SQLite failed; the transaction rolled back.
    #[error("sqlite: {0}")]
    Sqlite(#[from] rusqlite::Error),
    /// JSON (de)serialisation failed.
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
    /// `transition` refused the event; nothing was written.
    #[error("transition refused: {0}")]
    Refused(Refusal),
    /// A ledger row cannot be read back as a legal history (EX-05: parse, never repair).
    #[error("ledger row for task {task} unreadable: {detail}")]
    Corrupt {
        /// The task whose history failed.
        task: String,
        /// What failed.
        detail: String,
    },
    /// Same operation key, different request bytes (task.submit `conflict`).
    #[error("operation {0:?} already recorded with other request bytes")]
    Conflict(OperationKey),
    /// Same observation id, different body.
    #[error("observation {0} already recorded with another body")]
    ObservationConflict(ObservationId),
    /// The receipt does not extend its task's chain.
    #[error("receipt chain: {0}")]
    Chain(ChainBreak),
    /// The receipt cites an observation not ledgered for its task (STACK-MAP §3.5).
    #[error("receipt cites unledgered observation {0}")]
    UnledgeredObservation(ObservationId),
    /// `Dispatch` before this process's `recovery::reconcile` completed.
    #[error("dispatch refused: recovery_complete is false")]
    RecoveryIncomplete,
    /// The file's schema is newer than this binary.
    #[error("schema version {0} is not known to this binary (max {SCHEMA_VERSION})")]
    UnknownSchema(i64),
}

/// The idempotency key of a mutating operation: `(principal, action, version, idem_key)`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OperationKey {
    /// Who asked.
    pub principal: String,
    /// The action, e.g. `task.submit`.
    pub action: String,
    /// The action's contract version.
    pub version: u32,
    /// The caller's idempotency key.
    pub idem_key: String,
}

/// What [`Store::admit`] answered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Admission {
    /// The admitted task (the stored one on a replay).
    pub task_id: TaskId,
    /// `true` when the key was already recorded with the same request bytes.
    pub replayed: bool,
    /// The stored result.
    pub result: Value,
}

/// The `tasks` cache row, as stored (for recovery's cache check).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CachedRow {
    /// `Phase::as_str` spelling.
    pub phase: String,
    /// Cancellation requested.
    pub cancel: bool,
    /// Number of `Dispatch` events.
    pub generation: u64,
}

/// One time `apply` found `tasks.phase` divergent from replay and rewrote it (replay wins).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CacheHeal {
    /// The task whose cache row diverged.
    pub task_id: TaskId,
    /// What `tasks.phase` said before the heal.
    pub cached_phase: String,
    /// What replay of the events said (and what the cache now says).
    pub replayed_phase: String,
}

/// The ledger.
#[derive(Debug)]
pub struct Store {
    conn: Connection,
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_millis()).unwrap_or(i64::MAX))
}

fn corrupt(task: &TaskId, detail: impl Into<String>) -> StoreError {
    StoreError::Corrupt {
        task: task.to_string(),
        detail: detail.into(),
    }
}

fn load_events(conn: &Connection, task: &TaskId) -> Result<Vec<Event>, StoreError> {
    let mut stmt =
        conn.prepare_cached("SELECT seq, event_json FROM events WHERE task_id = ?1 ORDER BY seq")?;
    let rows = stmt.query_map([task.as_str()], |r| {
        Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?))
    })?;
    let mut events = Vec::new();
    for row in rows {
        let (seq, text) = row?;
        let event = codec::decode(&text)
            .ok_or_else(|| corrupt(task, format!("events.seq={seq} is not an Event: {text}")))?;
        events.push(event);
    }
    Ok(events)
}

fn replay(task: &TaskId, events: &[Event]) -> Result<Option<TaskState>, StoreError> {
    if events.is_empty() {
        return Ok(None);
    }
    TaskState::replay(events.iter().copied())
        .map(Some)
        .map_err(|r| corrupt(task, format!("replay refused: {r}")))
}

fn meta_get(conn: &Connection, key: &str) -> Result<Option<String>, rusqlite::Error> {
    conn.query_row("SELECT value FROM meta WHERE key = ?1", [key], |r| r.get(0))
        .optional()
}

fn meta_set(conn: &Connection, key: &str, value: &str) -> Result<(), rusqlite::Error> {
    conn.execute(
        "INSERT INTO meta(key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        [key, value],
    )?;
    Ok(())
}

/// The one state writer: replay, `transition`, append the event, update the cache. Runs inside
/// the caller's transaction; the caller commits.
fn apply_in(tx: &Transaction<'_>, task: &TaskId, event: Event) -> Result<Phase, StoreError> {
    if event == Event::Dispatch && meta_get(tx, "recovery_complete")?.as_deref() != Some("1") {
        return Err(StoreError::RecoveryIncomplete);
    }
    let history = load_events(tx, task)?;
    let before = replay(task, &history)?;
    let next = transition(before, event).map_err(StoreError::Refused)?;
    let phase = next.phase();
    let ts = now_ms();
    if let Some(prior) = before.map(TaskState::phase) {
        let cached: Option<String> = tx
            .query_row(
                "SELECT phase FROM tasks WHERE id = ?1",
                [task.as_str()],
                |r| r.get(0),
            )
            .optional()?;
        if let Some(cached) = cached.filter(|c| c != prior.as_str()) {
            tx.execute(
                "INSERT INTO cache_heals(task_id, cached_phase, replayed_phase, ts)
                 VALUES (?1, ?2, ?3, ?4)",
                params![task.as_str(), cached, prior.as_str(), ts],
            )?;
        }
    }
    let generation = history
        .iter()
        .chain(std::iter::once(&event))
        .filter(|e| **e == Event::Dispatch)
        .count();
    let generation = i64::try_from(generation).map_err(|_| corrupt(task, "generation overflow"))?;
    tx.execute(
        "INSERT INTO tasks(id, phase, cancel, generation, updated_ts) VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT(id) DO UPDATE SET phase = excluded.phase, cancel = excluded.cancel,
           generation = excluded.generation, updated_ts = excluded.updated_ts",
        params![
            task.as_str(),
            phase.as_str(),
            phase.cancel_requested(),
            generation,
            ts
        ],
    )?;
    tx.execute(
        "INSERT INTO events(task_id, event_json, ts) VALUES (?1, ?2, ?3)",
        params![task.as_str(), codec::encode(event)?, ts],
    )?;
    Ok(phase)
}

impl Store {
    /// Open (creating or migrating) the ledger at `path`: WAL, `synchronous=FULL`, foreign keys
    /// on. Resets `recovery_complete` to false: every process reconciles before it dispatches.
    ///
    /// # Errors
    /// SQLite errors, or [`StoreError::UnknownSchema`] for a newer file.
    pub fn open(path: &Path) -> Result<Self, StoreError> {
        let mut conn = Connection::open(path)?;
        let mode: String = conn.query_row("PRAGMA journal_mode = WAL", [], |r| r.get(0))?;
        if !mode.eq_ignore_ascii_case("wal") {
            return Err(StoreError::Corrupt {
                task: String::new(),
                detail: format!("journal_mode is {mode}, not wal"),
            });
        }
        conn.pragma_update(None, "synchronous", "FULL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        conn.busy_timeout(std::time::Duration::from_secs(5))?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Exclusive)?;
        let version: i64 = tx.query_row("PRAGMA user_version", [], |r| r.get(0))?;
        match version {
            0 => {
                tx.execute_batch(SCHEMA_V1)?;
                tx.execute_batch(SCHEMA_V2)?;
                tx.pragma_update(None, "user_version", SCHEMA_VERSION)?;
                meta_set(&tx, "epoch", &format!("{:x}", now_ms()))?;
            }
            1 => {
                tx.execute_batch(SCHEMA_V2)?;
                tx.pragma_update(None, "user_version", SCHEMA_VERSION)?;
            }
            SCHEMA_VERSION => {}
            other => return Err(StoreError::UnknownSchema(other)),
        }
        meta_set(&tx, "recovery_complete", "0")?;
        tx.commit()?;
        Ok(Self { conn })
    }

    fn begin(&self) -> Result<Transaction<'_>, rusqlite::Error> {
        Transaction::new_unchecked(&self.conn, TransactionBehavior::Immediate)
    }

    /// Apply `event` to `task`: replay its events, call `transition`, append the event and update
    /// the `tasks` cache in one transaction, commit (fsync), return the new phase. A refused
    /// event writes nothing.
    ///
    /// # Errors
    /// [`StoreError::Refused`] from `transition`; [`StoreError::RecoveryIncomplete`] for a
    /// `Dispatch` before reconcile; [`StoreError::Corrupt`] for an unreadable history.
    pub fn apply(&self, task: &TaskId, event: Event) -> Result<Phase, StoreError> {
        let tx = self.begin()?;
        let phase = apply_in(&tx, task, event)?;
        tx.commit()?;
        Ok(phase)
    }

    /// The idempotency door. Same key and same request digest: the stored result, `replayed`.
    /// Same key, other digest: [`StoreError::Conflict`]. New key: `Admit` `task`, compute the
    /// result with `f`, record the operation, all in one transaction, then commit (fsync).
    ///
    /// # Errors
    /// [`StoreError::Conflict`]; [`StoreError::Refused`] if `task` already exists.
    pub fn admit<F>(
        &self,
        task: &TaskId,
        op: &OperationKey,
        request: &[u8],
        f: F,
    ) -> Result<Admission, StoreError>
    where
        F: FnOnce(&TaskId, Phase) -> Value,
    {
        let sha = Sha256Hex::digest(request).to_string();
        let tx = self.begin()?;
        let stored: Option<(String, String, String)> = tx
            .query_row(
                "SELECT request_sha256, task_id, result_json FROM operations
                 WHERE principal = ?1 AND action = ?2 AND version = ?3 AND idem_key = ?4",
                params![op.principal, op.action, op.version, op.idem_key],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .optional()?;
        if let Some((stored_sha, stored_task, result)) = stored {
            if stored_sha != sha {
                return Err(StoreError::Conflict(op.clone()));
            }
            let task_id = stored_task
                .parse()
                .map_err(|e| corrupt(task, format!("operations.task_id: {e}")))?;
            return Ok(Admission {
                task_id,
                replayed: true,
                result: serde_json::from_str(&result)?,
            });
        }
        let phase = apply_in(&tx, task, Event::Admit)?;
        let result = f(task, phase);
        tx.execute(
            "INSERT INTO operations(principal, action, version, idem_key, request_sha256, task_id,
               result_json, ts) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                op.principal,
                op.action,
                op.version,
                op.idem_key,
                sha,
                task.as_str(),
                serde_json::to_string(&result)?,
                now_ms()
            ],
        )?;
        tx.commit()?;
        Ok(Admission {
            task_id: task.clone(),
            replayed: false,
            result,
        })
    }

    /// Ledger an observation for `task` (before any verdict cites it). Re-recording the same id
    /// with the same body is a no-op.
    ///
    /// # Errors
    /// [`StoreError::ObservationConflict`] for the same id with another body; SQLite's foreign
    /// key error for an unknown task.
    pub fn record_observation(
        &self,
        task: &TaskId,
        id: &ObservationId,
        observation: &Observation,
    ) -> Result<(), StoreError> {
        let json = serde_json::to_string(observation)?;
        let tx = self.begin()?;
        let existing: Option<(String, String)> = tx
            .query_row(
                "SELECT task_id, json FROM observations WHERE id = ?1",
                [id.as_str()],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
        match existing {
            Some((t, j)) if t == task.as_str() && j == json => return Ok(()),
            Some(_) => return Err(StoreError::ObservationConflict(id.clone())),
            None => {}
        }
        tx.execute(
            "INSERT INTO observations(id, task_id, json, ts) VALUES (?1, ?2, ?3, ?4)",
            params![id.as_str(), task.as_str(), json, now_ms()],
        )?;
        tx.commit()?;
        Ok(())
    }

    /// Append a sealed receipt to its task's chain. The chain with the new receipt must pass
    /// [`Receipt::verify_chain`] (so `hash_prev` is the last `hash_self` or `GENESIS`), and every
    /// observation it cites must be ledgered for the same task.
    ///
    /// # Errors
    /// [`StoreError::Chain`], [`StoreError::UnledgeredObservation`].
    pub fn append_receipt(&self, receipt: &Receipt) -> Result<(), StoreError> {
        let task = receipt.task_id();
        let tx = self.begin()?;
        let mut chain = Vec::new();
        {
            let mut stmt =
                tx.prepare_cached("SELECT json FROM receipts WHERE task_id = ?1 ORDER BY seq")?;
            for json in stmt.query_map([task.as_str()], |r| r.get::<_, String>(0))? {
                chain.push(serde_json::from_str::<Receipt>(&json?)?);
            }
        }
        chain.push(receipt.clone());
        Receipt::verify_chain(&chain).map_err(StoreError::Chain)?;
        for obs in receipt.observed() {
            let owner: Option<String> = tx
                .query_row(
                    "SELECT task_id FROM observations WHERE id = ?1",
                    [obs.as_str()],
                    |r| r.get(0),
                )
                .optional()?;
            if owner.as_deref() != Some(task.as_str()) {
                return Err(StoreError::UnledgeredObservation(obs.clone()));
            }
        }
        tx.execute(
            "INSERT INTO receipts(id, task_id, json, hash_prev, hash_self, ts)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                receipt.id().as_str(),
                task.as_str(),
                serde_json::to_string(receipt)?,
                receipt.hash_prev().to_string(),
                receipt.hash_self().to_string(),
                now_ms()
            ],
        )?;
        tx.commit()?;
        Ok(())
    }

    /// The last `hash_self` of `task`'s chain, or `GENESIS`.
    ///
    /// # Errors
    /// SQLite errors; [`StoreError::Corrupt`] for an unparsable hash.
    pub fn chain_head(&self, task: &TaskId) -> Result<Sha256Hex, StoreError> {
        let last: Option<String> = self
            .conn
            .query_row(
                "SELECT hash_self FROM receipts WHERE task_id = ?1 ORDER BY seq DESC LIMIT 1",
                [task.as_str()],
                |r| r.get(0),
            )
            .optional()?;
        last.map_or(Ok(Sha256Hex::GENESIS), |h| {
            h.parse()
                .map_err(|e| corrupt(task, format!("hash_self: {e}")))
        })
    }

    /// `task`'s decoded event history, oldest first.
    ///
    /// # Errors
    /// [`StoreError::Corrupt`] for a row that is not an `Event`.
    pub fn history(&self, task: &TaskId) -> Result<Vec<Event>, StoreError> {
        load_events(&self.conn, task)
    }

    /// `task`'s phase by replay of its events (the source of truth), `None` if never admitted.
    ///
    /// # Errors
    /// [`StoreError::Corrupt`] for an unreadable or unreplayable history.
    pub fn phase(&self, task: &TaskId) -> Result<Option<Phase>, StoreError> {
        Ok(replay(task, &self.history(task)?)?.map(TaskState::phase))
    }

    /// `task`'s `tasks` cache row.
    ///
    /// # Errors
    /// SQLite errors.
    pub fn cached(&self, task: &TaskId) -> Result<Option<CachedRow>, StoreError> {
        Ok(self
            .conn
            .query_row(
                "SELECT phase, cancel, generation FROM tasks WHERE id = ?1",
                [task.as_str()],
                |r| {
                    Ok(CachedRow {
                        phase: r.get(0)?,
                        cancel: r.get(1)?,
                        generation: u64::try_from(r.get::<_, i64>(2)?).unwrap_or(u64::MAX),
                    })
                },
            )
            .optional()?)
    }

    /// Every cache heal `apply` has recorded, oldest first.
    ///
    /// # Errors
    /// SQLite errors, or [`StoreError::Corrupt`] for an unparsable id.
    pub fn cache_heals(&self) -> Result<Vec<CacheHeal>, StoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT task_id, cached_phase, replayed_phase FROM cache_heals ORDER BY seq",
        )?;
        let rows = stmt
            .query_map([], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        rows.into_iter()
            .map(|(id, cached_phase, replayed_phase)| {
                let task_id = id.parse().map_err(|e| StoreError::Corrupt {
                    task: id.clone(),
                    detail: format!("cache_heals.task_id: {e}"),
                })?;
                Ok(CacheHeal {
                    task_id,
                    cached_phase,
                    replayed_phase,
                })
            })
            .collect()
    }

    /// Every task id in the ledger, in id order. An id that does not parse is returned as the
    /// error, never skipped.
    ///
    /// # Errors
    /// [`StoreError::Corrupt`] for an unparsable id.
    pub fn task_ids(&self) -> Result<Vec<TaskId>, StoreError> {
        let mut stmt = self.conn.prepare("SELECT id FROM tasks ORDER BY id")?;
        let ids = stmt.query_map([], |r| r.get::<_, String>(0))?;
        let mut out = Vec::new();
        for id in ids {
            let id = id?;
            out.push(id.parse().map_err(|e| StoreError::Corrupt {
                task: id.clone(),
                detail: format!("tasks.id: {e}"),
            })?);
        }
        Ok(out)
    }

    /// Total rows in `events`.
    ///
    /// # Errors
    /// SQLite errors.
    pub fn event_count(&self) -> Result<u64, StoreError> {
        let n: i64 = self
            .conn
            .query_row("SELECT count(*) FROM events", [], |r| r.get(0))?;
        Ok(u64::try_from(n).unwrap_or(0))
    }

    /// The ledger epoch, set when the file was created.
    ///
    /// # Errors
    /// SQLite errors.
    pub fn epoch(&self) -> Result<String, StoreError> {
        Ok(meta_get(&self.conn, "epoch")?.unwrap_or_default())
    }

    /// Whether this process's reconcile completed. K6 reads it before binding or dispatching;
    /// `apply(_, Dispatch)` refuses while it is false.
    ///
    /// # Errors
    /// SQLite errors.
    pub fn recovery_complete(&self) -> Result<bool, StoreError> {
        Ok(meta_get(&self.conn, "recovery_complete")?.as_deref() == Some("1"))
    }

    pub(crate) fn set_recovery_complete(&self, complete: bool) -> Result<(), StoreError> {
        let tx = self.begin()?;
        meta_set(&tx, "recovery_complete", if complete { "1" } else { "0" })?;
        tx.commit()?;
        Ok(())
    }

    /// `PRAGMA integrity_check`, first row (`ok` when sound).
    ///
    /// # Errors
    /// SQLite errors.
    pub fn integrity_check(&self) -> Result<String, StoreError> {
        Ok(self
            .conn
            .query_row("PRAGMA integrity_check", [], |r| r.get(0))?)
    }

    /// The connection's `(journal_mode, synchronous, foreign_keys)` as SQLite reports them.
    ///
    /// # Errors
    /// SQLite errors.
    pub fn pragmas(&self) -> Result<(String, i64, i64), StoreError> {
        let j = self
            .conn
            .query_row("PRAGMA journal_mode", [], |r| r.get(0))?;
        let s = self
            .conn
            .query_row("PRAGMA synchronous", [], |r| r.get(0))?;
        let f = self
            .conn
            .query_row("PRAGMA foreign_keys", [], |r| r.get(0))?;
        Ok((j, s, f))
    }
}
