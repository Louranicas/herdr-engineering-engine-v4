//! The ledger. The one directory in this crate that writes SQL (`FLOW.md`, enforced by
//! `tests/one_door.rs`).
//!
//! `events` is the source of truth; `tasks` is a cache of `TaskState::replay(events)` written in
//! the same transaction as the event that changed it. [`Store::apply`] is the only state writer:
//! a refused transition rolls back and writes nothing. Every commit runs under
//! `journal_mode=WAL` + `synchronous=FULL`, so the WAL is fsynced before `apply`, `admit`,
//! `record_observation` or `append_receipt` returns (the ack point, task.submit "fsync before
//! ack").
//!
//! The schema is a list of named migrations ([`migrations::MIGRATIONS`]); `open` applies the
//! ones a file lacks and refuses one it does not know. `Store::operate` is the one idempotent
//! operation primitive; [`Store::admit`] is its task-family caller.

mod backup;
pub(crate) mod migrations;

use std::collections::BTreeSet;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use hee4_contracts::{
    ChainBreak, Event, Observation, ObservationId, Phase, Receipt, Refusal, Sha256Hex, TaskId,
    TaskState, transition,
};
use rusqlite::{Connection, OptionalExtension, Transaction, TransactionBehavior, params};
use serde_json::Value;

use crate::codec;

const CGROUP_FILE: &str = "/proc/self/cgroup";

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
    /// The file system refused (mode convergence, the cgroup read, a snapshot path).
    #[error("io at {path}: {source}")]
    Io {
        /// The path the call touched.
        path: String,
        /// The OS's answer.
        #[source]
        source: std::io::Error,
    },
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
    /// The file holds a migration (or a legacy `user_version`) this binary does not know: the
    /// file is newer than the binary.
    #[error("migration {0} is not known to this binary")]
    UnknownMigration(String),
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

/// What `Store::operate` answered: the stored operation, new or replayed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Operation {
    /// `op-` + the first 24 hex digits of sha256 of the key's four fields joined by `\n`.
    pub operation_id: String,
    /// The task the operation created or acted on, if any.
    pub task_id: Option<TaskId>,
    /// The subject the operation is about (a task id, a roster name, ...), if any.
    pub subject: Option<String>,
    /// `true` when the key was already recorded with the same request bytes.
    pub replayed: bool,
    /// The stored result.
    pub result: Value,
}

/// One `operations` row as read back by [`Store::operation_by_key`] and
/// [`Store::last_operation_for`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationRow {
    /// The derived id (see [`Operation::operation_id`]).
    pub operation_id: String,
    /// The action, e.g. `task.submit`.
    pub action: String,
    /// The caller's idempotency key.
    pub idem_key: String,
    /// The task, if any.
    pub task_id: Option<TaskId>,
    /// The subject, if any.
    pub subject: Option<String>,
    /// The stored result.
    pub result: Value,
    /// When the row was written (ms since the Unix epoch).
    pub ts: i64,
}

/// What [`Store::cursor_check`] says about a subscriber cursor (R13; never a replay
/// authorization).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CursorVerdict {
    /// The cursor's epoch is the one this ledger was restored from.
    PriorEpochOfRestore,
    /// The cursor's epoch is not this ledger's epoch.
    EpochChanged,
    /// The cursor's sequence is past `event_high_water`.
    FutureSequence,
    /// The cursor is consistent with this ledger: a snapshot answer, no replay.
    SnapshotOnly,
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

/// An `operations` row as SQLite hands it over, before its ids and JSON are parsed.
struct RawOperation {
    operation_id: String,
    action: String,
    idem_key: String,
    task_id: Option<String>,
    subject: Option<String>,
    result_json: String,
    ts: i64,
    request_sha256: String,
}

impl RawOperation {
    fn from_row(r: &rusqlite::Row<'_>) -> Result<Self, rusqlite::Error> {
        Ok(Self {
            operation_id: r.get(0)?,
            action: r.get(1)?,
            idem_key: r.get(2)?,
            task_id: r.get(3)?,
            subject: r.get(4)?,
            result_json: r.get(5)?,
            ts: r.get(6)?,
            request_sha256: r.get(7)?,
        })
    }

    fn parse(self) -> Result<OperationRow, StoreError> {
        let task_id = self
            .task_id
            .map(|t| {
                t.parse::<TaskId>().map_err(|e| StoreError::Corrupt {
                    task: t.clone(),
                    detail: format!("operations.task_id: {e}"),
                })
            })
            .transpose()?;
        Ok(OperationRow {
            operation_id: self.operation_id,
            action: self.action,
            idem_key: self.idem_key,
            task_id,
            subject: self.subject,
            result: serde_json::from_str(&self.result_json)?,
            ts: self.ts,
        })
    }
}

/// The ledger.
#[derive(Debug)]
pub struct Store {
    conn: Connection,
    path: PathBuf,
    serve_cgroup: String,
}

pub(crate) fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_millis()).unwrap_or(i64::MAX))
}

pub(crate) fn corrupt(task: &TaskId, detail: impl Into<String>) -> StoreError {
    StoreError::Corrupt {
        task: task.to_string(),
        detail: detail.into(),
    }
}

fn io_at(path: &Path, source: std::io::Error) -> StoreError {
    StoreError::Io {
        path: path.display().to_string(),
        source,
    }
}

/// `op-` + the first 24 hex digits of `sha256("principal\naction\nversion\nidem_key")`:
/// deterministic, so a migration and a fresh insert derive the same id.
pub(crate) fn operation_id(op: &OperationKey) -> String {
    let text = format!(
        "{}\n{}\n{}\n{}",
        op.principal, op.action, op.version, op.idem_key
    );
    let digest = Sha256Hex::digest(text.as_bytes()).to_string();
    format!("op-{}", &digest[..24])
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

pub(crate) fn meta_get(conn: &Connection, key: &str) -> Result<Option<String>, rusqlite::Error> {
    conn.query_row("SELECT value FROM meta WHERE key = ?1", [key], |r| r.get(0))
        .optional()
}

pub(crate) fn meta_set(conn: &Connection, key: &str, value: &str) -> Result<(), rusqlite::Error> {
    conn.execute(
        "INSERT INTO meta(key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        [key, value],
    )?;
    Ok(())
}

fn table_exists(conn: &Connection, name: &str) -> Result<bool, rusqlite::Error> {
    let n: i64 = conn.query_row(
        "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
        [name],
        |r| r.get(0),
    )?;
    Ok(n > 0)
}

/// The one state writer: replay, `transition`, append the event, update the cache. Runs inside
/// the caller's transaction; the caller commits. `serve_cgroup` is written only when the
/// `tasks` row is created (the `Admit` row); a later event never overwrites it.
fn apply_in(
    tx: &Transaction<'_>,
    serve_cgroup: &str,
    task: &TaskId,
    event: Event,
) -> Result<Phase, StoreError> {
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
        "INSERT INTO tasks(id, phase, cancel, generation, updated_ts, serve_cgroup)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)
         ON CONFLICT(id) DO UPDATE SET phase = excluded.phase, cancel = excluded.cancel,
           generation = excluded.generation, updated_ts = excluded.updated_ts",
        params![
            task.as_str(),
            phase.as_str(),
            phase.cancel_requested(),
            generation,
            ts,
            serve_cgroup
        ],
    )?;
    tx.execute(
        "INSERT INTO events(task_id, event_json, ts) VALUES (?1, ?2, ?3)",
        params![task.as_str(), codec::encode(event)?, ts],
    )?;
    Ok(phase)
}

/// Apply every migration the file lacks, inside `tx`; seed a legacy file's rows from its
/// `user_version`; refuse a row this binary does not know. Returns the count of applied rows.
fn migrate(tx: &Transaction<'_>) -> Result<usize, StoreError> {
    let user_version: i64 = tx.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    let mut applied: BTreeSet<String> = BTreeSet::new();
    if table_exists(tx, "meta")? {
        let mut stmt = tx.prepare("SELECT key FROM meta WHERE key LIKE 'migration:%'")?;
        for key in stmt.query_map([], |r| r.get::<_, String>(0))? {
            let key = key?;
            applied.insert(key.trim_start_matches("migration:").to_owned());
        }
    }
    if applied.is_empty() {
        let seeded = migrations::seeded_by_user_version(user_version)
            .ok_or_else(|| StoreError::UnknownMigration(format!("user_version={user_version}")))?;
        applied.extend(seeded.iter().map(|s| (*s).to_owned()));
    }
    let known: BTreeSet<&str> = migrations::MIGRATIONS.iter().map(|m| m.name).collect();
    if let Some(unknown) = applied.iter().find(|name| !known.contains(name.as_str())) {
        return Err(StoreError::UnknownMigration(unknown.clone()));
    }
    let stamp = format!("{:x}", now_ms());
    for m in migrations::MIGRATIONS {
        if !applied.contains(m.name) {
            (m.apply)(tx)?;
            applied.insert(m.name.to_owned());
        }
        // A seeded legacy row and a freshly applied one are written the same way; a present
        // row is left as it is.
        if meta_get(tx, &format!("migration:{}", m.name))?.is_none() {
            meta_set(tx, &format!("migration:{}", m.name), &stamp)?;
        }
    }
    let count = applied.len();
    tx.pragma_update(
        None,
        "user_version",
        i64::try_from(count).unwrap_or(i64::MAX),
    )?;
    Ok(count)
}

/// The `0::<path>` line of `/proc/self/cgroup`, the cgroup v2 path of this process.
fn read_serve_cgroup() -> Result<String, StoreError> {
    let text =
        std::fs::read_to_string(CGROUP_FILE).map_err(|e| io_at(Path::new(CGROUP_FILE), e))?;
    text.lines()
        .find_map(|line| line.strip_prefix("0::"))
        .map(|p| p.trim_end().to_owned())
        .ok_or_else(|| {
            io_at(
                Path::new(CGROUP_FILE),
                std::io::Error::new(std::io::ErrorKind::InvalidData, "no `0::` line"),
            )
        })
}

/// Ledger 0600 (with `-wal` and `-shm` when present), its directory 0700. Idempotent.
fn converge_modes(path: &Path) -> Result<(), StoreError> {
    let base = path.as_os_str().to_owned();
    for suffix in ["", "-wal", "-shm"] {
        let mut name = base.clone();
        name.push(suffix);
        let file = PathBuf::from(name);
        if file.exists() {
            std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o600))
                .map_err(|e| io_at(&file, e))?;
        }
    }
    if let Some(dir) = path.parent().filter(|d| !d.as_os_str().is_empty()) {
        std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700))
            .map_err(|e| io_at(dir, e))?;
    }
    Ok(())
}

impl Store {
    /// Open (creating or migrating) the ledger at `path`: WAL, `synchronous=FULL`, foreign keys
    /// on. Applies every named migration the file lacks, increments `meta.boot`, records this
    /// process's cgroup as `meta.serve_cgroup`, resets `recovery_complete` to false (every
    /// process reconciles before it dispatches), then converges the file modes to 0600 and the
    /// directory to 0700.
    ///
    /// # Errors
    /// SQLite or IO errors, or [`StoreError::UnknownMigration`] for a newer file.
    pub fn open(path: &Path) -> Result<Self, StoreError> {
        let serve_cgroup = read_serve_cgroup()?;
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
        migrate(&tx)?;
        if meta_get(&tx, "epoch")?.is_none() {
            meta_set(&tx, "epoch", &format!("{:x}", now_ms()))?;
        }
        let boot = meta_get(&tx, "boot")?
            .and_then(|b| b.parse::<u64>().ok())
            .unwrap_or(0)
            .saturating_add(1);
        meta_set(&tx, "boot", &boot.to_string())?;
        meta_set(&tx, "serve_cgroup", &serve_cgroup)?;
        meta_set(&tx, "recovery_complete", "0")?;
        tx.commit()?;
        converge_modes(path)?;
        Ok(Self {
            conn,
            path: path.to_path_buf(),
            serve_cgroup,
        })
    }

    fn begin(&self) -> Result<Transaction<'_>, rusqlite::Error> {
        Transaction::new_unchecked(&self.conn, TransactionBehavior::Immediate)
    }

    /// The path `open` was given.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
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
        let phase = apply_in(&tx, &self.serve_cgroup, task, event)?;
        tx.commit()?;
        Ok(phase)
    }

    /// The one idempotent-operation primitive. Same key and same request digest: the stored
    /// operation, `replayed`. Same key, other digest: [`StoreError::Conflict`]. New key: run `f`
    /// inside one Immediate transaction, record the operation with its derived id, commit
    /// (fsync). An `Err` from `f` rolls everything back and records nothing.
    ///
    /// Crate-private: a family file under `src/store/` wraps it in a `pub fn <family>_*` verb,
    /// so no crate outside `hee4-core` ever holds a `Transaction`.
    ///
    /// # Errors
    /// [`StoreError::Conflict`]; whatever `f` returns.
    pub(crate) fn operate<F>(
        &self,
        op: &OperationKey,
        request: &[u8],
        f: F,
    ) -> Result<Operation, StoreError>
    where
        F: FnOnce(&Transaction<'_>) -> Result<(Option<TaskId>, Option<String>, Value), StoreError>,
    {
        let sha = Sha256Hex::digest(request).to_string();
        let tx = self.begin()?;
        let stored = tx
            .query_row(
                "SELECT operation_id, action, idem_key, task_id, subject, result_json, ts,
                        request_sha256
                 FROM operations
                 WHERE principal = ?1 AND action = ?2 AND version = ?3 AND idem_key = ?4",
                params![op.principal, op.action, op.version, op.idem_key],
                RawOperation::from_row,
            )
            .optional()?;
        if let Some(raw) = stored {
            if raw.request_sha256 != sha {
                return Err(StoreError::Conflict(op.clone()));
            }
            let row = raw.parse()?;
            return Ok(Operation {
                operation_id: row.operation_id,
                task_id: row.task_id,
                subject: row.subject,
                replayed: true,
                result: row.result,
            });
        }
        let (task_id, subject, result) = f(&tx)?;
        let operation_id = operation_id(op);
        tx.execute(
            "INSERT INTO operations(operation_id, principal, action, version, idem_key,
               request_sha256, task_id, subject, result_json, ts)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                operation_id,
                op.principal,
                op.action,
                op.version,
                op.idem_key,
                sha,
                task_id.as_ref().map(TaskId::as_str),
                subject,
                serde_json::to_string(&result)?,
                now_ms()
            ],
        )?;
        tx.commit()?;
        Ok(Operation {
            operation_id,
            task_id,
            subject,
            replayed: false,
            result,
        })
    }

    /// The idempotency door for task admission, a caller of `Store::operate`. Same key and
    /// same request digest: the stored result, `replayed`. Same key, other digest:
    /// [`StoreError::Conflict`]. New key: `Admit` `task`, compute the result with `f`, record
    /// the operation (subject = the task id), all in one transaction, then commit (fsync).
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
        let operation = self.operate(op, request, |tx| {
            let phase = apply_in(tx, &self.serve_cgroup, task, Event::Admit)?;
            let result = f(task, phase);
            Ok((Some(task.clone()), Some(task.to_string()), result))
        })?;
        let task_id = operation
            .task_id
            .ok_or_else(|| corrupt(task, "operations.task_id is null for an admission"))?;
        Ok(Admission {
            task_id,
            replayed: operation.replayed,
            result: operation.result,
        })
    }

    /// The `operations` row for `op`, if recorded.
    ///
    /// # Errors
    /// SQLite errors; [`StoreError::Corrupt`] for an unparsable task id or result.
    pub fn operation_by_key(&self, op: &OperationKey) -> Result<Option<OperationRow>, StoreError> {
        self.operation_row(
            "SELECT operation_id, action, idem_key, task_id, subject, result_json, ts,
                    request_sha256
             FROM operations
             WHERE principal = ?1 AND action = ?2 AND version = ?3 AND idem_key = ?4",
            params![op.principal, op.action, op.version, op.idem_key],
        )
    }

    /// The newest `operations` row whose `subject` is `subject` (by `ts`, ties by rowid).
    ///
    /// # Errors
    /// SQLite errors; [`StoreError::Corrupt`] for an unparsable task id or result.
    pub fn last_operation_for(&self, subject: &str) -> Result<Option<OperationRow>, StoreError> {
        self.operation_row(
            "SELECT operation_id, action, idem_key, task_id, subject, result_json, ts,
                    request_sha256
             FROM operations WHERE subject = ?1 ORDER BY ts DESC, rowid DESC LIMIT 1",
            params![subject],
        )
    }

    fn operation_row(
        &self,
        sql: &str,
        args: impl rusqlite::Params,
    ) -> Result<Option<OperationRow>, StoreError> {
        self.conn
            .query_row(sql, args, RawOperation::from_row)
            .optional()?
            .map(RawOperation::parse)
            .transpose()
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

    /// The highest `events.seq`, or 0 for an empty ledger (R13's high water).
    ///
    /// # Errors
    /// SQLite errors.
    pub fn event_high_water(&self) -> Result<u64, StoreError> {
        let n: Option<i64> = self
            .conn
            .query_row("SELECT max(seq) FROM events", [], |r| r.get(0))?;
        Ok(n.and_then(|n| u64::try_from(n).ok()).unwrap_or(0))
    }

    /// The ledger epoch, set when the file was created (or renewed by a restore).
    ///
    /// # Errors
    /// SQLite errors.
    pub fn epoch(&self) -> Result<String, StoreError> {
        Ok(meta_get(&self.conn, "epoch")?.unwrap_or_default())
    }

    /// Give the ledger a fresh `meta.epoch` (a restore's new generation) and return it. Every
    /// cursor minted under the old epoch then answers `EpochChanged` or `PriorEpochOfRestore`.
    pub(crate) fn renew_epoch(&self) -> Result<String, StoreError> {
        let epoch = format!("{:x}", now_ms());
        let tx = self.begin()?;
        meta_set(&tx, "epoch", &epoch)?;
        tx.commit()?;
        Ok(epoch)
    }

    /// How many times this file has been opened (`meta.boot`, incremented by every `open`).
    ///
    /// # Errors
    /// SQLite errors; [`StoreError::Corrupt`] for an unparsable value.
    pub fn boot(&self) -> Result<u64, StoreError> {
        let text = meta_get(&self.conn, "boot")?.unwrap_or_default();
        text.parse().map_err(|e| StoreError::Corrupt {
            task: String::new(),
            detail: format!("meta.boot {text:?}: {e}"),
        })
    }

    /// Record the epoch this ledger was restored from (`meta.restored_from`); the one writer of
    /// that key, called by `backup::restore`.
    ///
    /// # Errors
    /// SQLite errors.
    pub fn mark_restored_from(&self, epoch: &str) -> Result<(), StoreError> {
        let tx = self.begin()?;
        meta_set(&tx, "restored_from", epoch)?;
        tx.commit()?;
        Ok(())
    }

    /// R13 over a subscriber cursor, pure over `meta` and `max(events.seq)`, in this order:
    /// `epoch == restored_from` → `PriorEpochOfRestore`; `epoch != meta.epoch` → `EpochChanged`;
    /// `seq > event_high_water` → `FutureSequence`; else `SnapshotOnly`. Never a replay
    /// authorization.
    ///
    /// # Errors
    /// SQLite errors.
    pub fn cursor_check(&self, epoch: &str, seq: u64) -> Result<CursorVerdict, StoreError> {
        if meta_get(&self.conn, "restored_from")?.as_deref() == Some(epoch) {
            return Ok(CursorVerdict::PriorEpochOfRestore);
        }
        if self.epoch()? != epoch {
            return Ok(CursorVerdict::EpochChanged);
        }
        if seq > self.event_high_water()? {
            return Ok(CursorVerdict::FutureSequence);
        }
        Ok(CursorVerdict::SnapshotOnly)
    }

    /// `PRAGMA user_version`: the count of applied migrations, as `open` left it.
    ///
    /// # Errors
    /// SQLite errors.
    pub fn schema_version(&self) -> Result<i64, StoreError> {
        Ok(self
            .conn
            .query_row("PRAGMA user_version", [], |r| r.get(0))?)
    }

    /// The cgroup path of the process that opened this store, as recorded in
    /// `meta.serve_cgroup` and stamped on every task it admits.
    #[must_use]
    pub fn serve_cgroup(&self) -> &str {
        &self.serve_cgroup
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

#[cfg(test)]
mod tests {
    //! `operate` through its crate-private seam: the behaviour `admit` and the wave-2 family
    //! verbs inherit.

    use super::*;
    use serde_json::json;

    type R = Result<(), Box<dyn std::error::Error>>;

    fn fresh(name: &str) -> Result<Store, Box<dyn std::error::Error>> {
        let dir = std::env::temp_dir().join(format!("hee4-core-operate-{}", std::process::id()));
        std::fs::create_dir_all(&dir)?;
        let path = dir.join(format!("{name}.sqlite"));
        for suffix in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{}{suffix}", path.display()));
        }
        Ok(Store::open(&path)?)
    }

    fn key(idem: &str) -> OperationKey {
        OperationKey {
            principal: "luke".into(),
            action: "roster.add".into(),
            version: 1,
            idem_key: idem.into(),
        }
    }

    #[test]
    fn operate_replays_same_bytes_conflicts_on_other_bytes_and_rolls_back_an_err() -> R {
        let store = fresh("operate")?;
        let first = store.operate(&key("k1"), b"body-A", |_| {
            Ok((None, Some("roster:alpha".into()), json!({"n": 1})))
        })?;
        assert!(!first.replayed);
        assert_eq!(first.operation_id, operation_id(&key("k1")));
        assert_eq!(first.subject.as_deref(), Some("roster:alpha"));
        let replay = store.operate(&key("k1"), b"body-A", |_| Ok((None, None, json!("never"))))?;
        assert_eq!(
            (replay.replayed, &replay.operation_id, &replay.result),
            (true, &first.operation_id, &first.result)
        );
        let conflict = store.operate(&key("k1"), b"body-B", |_| Ok((None, None, json!("never"))));
        assert!(matches!(conflict, Err(StoreError::Conflict(k)) if k == key("k1")));

        let before = store.event_count()?;
        let task: TaskId = "t-err".parse()?;
        let err = store.operate(&key("k2"), b"body", |tx| {
            apply_in(tx, "cg", &task, Event::Admit)?;
            Err(StoreError::RecoveryIncomplete)
        });
        assert!(matches!(err, Err(StoreError::RecoveryIncomplete)));
        assert_eq!(
            store.operation_by_key(&key("k2"))?,
            None,
            "no row from a failed closure"
        );
        assert_eq!(
            store.event_count()?,
            before,
            "the closure's event rolled back too"
        );
        assert_eq!(store.phase(&task)?, None);
        Ok(())
    }

    #[test]
    fn last_operation_for_returns_the_newest_of_a_subject() -> R {
        let store = fresh("newest")?;
        for (idem, n) in [("k1", 1), ("k2", 2), ("k3", 3)] {
            store.operate(&key(idem), b"x", |_| {
                Ok((None, Some("roster:beta".into()), json!({"n": n})))
            })?;
        }
        store.operate(&key("other"), b"x", |_| {
            Ok((None, Some("roster:gamma".into()), json!({"n": 99})))
        })?;
        let newest = store.last_operation_for("roster:beta")?.ok_or("row")?;
        assert_eq!(
            (newest.idem_key.as_str(), &newest.result),
            ("k3", &json!({"n": 3}))
        );
        assert_eq!(newest.task_id, None);
        assert_eq!(
            store
                .last_operation_for("roster:gamma")?
                .map(|r| r.idem_key),
            Some("other".into())
        );
        Ok(())
    }
}
