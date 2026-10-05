//! Named migrations. Each one is a `meta` row `migration:<name>` written in the same
//! transaction as its DDL; [`Store::open`](super::Store::open) applies every entry of
//! [`MIGRATIONS`] whose row is absent, in slice order, and refuses a file that holds a row this
//! binary does not know (a file newer than the binary). Never drop, rename or renumber an applied
//! migration; never edit [`SCHEMA_V1`] or [`SCHEMA_V2`]: a later change is a new entry.
//!
//! A family that lands later (attempts, roster, receipts) registers one line here.

use rusqlite::{Transaction, params};

use super::{OperationKey, StoreError, operation_id};

/// v1: the walking skeleton's six tables. Verbatim; a v1 file is migrated, never rewritten.
pub const SCHEMA_V1: &str = "
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
pub const SCHEMA_V2: &str = "
CREATE TABLE cache_heals(
  seq INTEGER PRIMARY KEY AUTOINCREMENT,
  task_id TEXT NOT NULL REFERENCES tasks(id),
  cached_phase TEXT NOT NULL,
  replayed_phase TEXT NOT NULL,
  ts INTEGER NOT NULL
) STRICT;
";

/// m003: `operations` gains a derived `operation_id` and a nullable `subject`, and `task_id`
/// becomes nullable, so a family other than tasks can record an idempotent operation.
const SCHEMA_M003: &str = "
CREATE TABLE operations_m003(
  operation_id TEXT NOT NULL UNIQUE,
  principal TEXT NOT NULL,
  action TEXT NOT NULL,
  version INTEGER NOT NULL,
  idem_key TEXT NOT NULL,
  request_sha256 TEXT NOT NULL CHECK (length(request_sha256) = 64),
  task_id TEXT NULL REFERENCES tasks(id),
  subject TEXT NULL,
  result_json TEXT NOT NULL,
  ts INTEGER NOT NULL,
  UNIQUE (principal, action, version, idem_key)
) STRICT;
";

/// One named migration: a function, not only SQL text, because a step may need to compute in
/// Rust what SQLite cannot (m003 derives sha256 operation ids).
#[derive(Debug, Clone, Copy)]
pub struct Migration {
    /// The `meta` row key is `migration:<name>`.
    pub name: &'static str,
    /// Runs inside `open`'s exclusive transaction; its row is written after it returns `Ok`.
    pub apply: fn(&Transaction<'_>) -> Result<(), StoreError>,
}

/// Every migration this binary knows, in application order. Append only.
pub const MIGRATIONS: &[Migration] = &[
    Migration {
        name: "m001_v1",
        apply: m001_v1,
    },
    Migration {
        name: "m002_cache_heals",
        apply: m002_cache_heals,
    },
    Migration {
        name: "m003_operations_subject",
        apply: m003_operations_subject,
    },
    Migration {
        name: "m004_serve_cgroup",
        apply: m004_serve_cgroup,
    },
    super::attempts::MIGRATION,
    super::receipts::m005_checkpoints(),
    super::roster::MIGRATION,
];

/// The migrations a legacy file (no `migration:` rows) has already applied, by its
/// `PRAGMA user_version`. `None` for a version this binary never wrote.
#[must_use]
pub fn seeded_by_user_version(user_version: i64) -> Option<&'static [&'static str]> {
    match user_version {
        0 => Some(&[]),
        1 => Some(&["m001_v1"]),
        2 => Some(&["m001_v1", "m002_cache_heals"]),
        _ => None,
    }
}

fn m001_v1(tx: &Transaction<'_>) -> Result<(), StoreError> {
    tx.execute_batch(SCHEMA_V1)?;
    Ok(())
}

fn m002_cache_heals(tx: &Transaction<'_>) -> Result<(), StoreError> {
    tx.execute_batch(SCHEMA_V2)?;
    Ok(())
}

fn m003_operations_subject(tx: &Transaction<'_>) -> Result<(), StoreError> {
    tx.execute_batch(SCHEMA_M003)?;
    let rows = {
        let mut stmt = tx.prepare(
            "SELECT principal, action, version, idem_key, request_sha256, task_id, result_json, ts
             FROM operations ORDER BY rowid",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok((
                OperationKey {
                    principal: r.get(0)?,
                    action: r.get(1)?,
                    version: r.get(2)?,
                    idem_key: r.get(3)?,
                },
                r.get::<_, String>(4)?,
                r.get::<_, String>(5)?,
                r.get::<_, String>(6)?,
                r.get::<_, i64>(7)?,
            ))
        })?;
        rows.collect::<Result<Vec<_>, _>>()?
    };
    for (key, sha, task_id, result_json, ts) in rows {
        tx.execute(
            "INSERT INTO operations_m003(operation_id, principal, action, version, idem_key,
               request_sha256, task_id, subject, result_json, ts)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                operation_id(&key),
                key.principal,
                key.action,
                key.version,
                key.idem_key,
                sha,
                task_id,
                task_id,
                result_json,
                ts
            ],
        )?;
    }
    tx.execute_batch(
        "DROP TABLE operations;
         ALTER TABLE operations_m003 RENAME TO operations;",
    )?;
    Ok(())
}

fn m004_serve_cgroup(tx: &Transaction<'_>) -> Result<(), StoreError> {
    tx.execute_batch("ALTER TABLE tasks ADD COLUMN serve_cgroup TEXT NOT NULL DEFAULT '';")?;
    Ok(())
}
