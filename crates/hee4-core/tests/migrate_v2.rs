//! A legacy v2 file (the verbatim v1+v2 SQL, `user_version=2`, no `migration:` rows) opened by
//! this binary: the named rows are seeded and m003/m004 applied, the data stays intact, the
//! operations row carries its derived id, a second open changes nothing, and a row naming a
//! migration this binary does not know is refused by name.

use std::error::Error;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

use hee4_contracts::{Decision, Event, Receipt, ReceiptBody, Sha256Hex, TaskId, Verdict};
use hee4_core::{OperationKey, Store, StoreError};
use rusqlite::{Connection, params};
use serde_json::json;

type R = Result<(), Box<dyn Error>>;

/// The v1 text as `store.rs` shipped it (a test may hold SQL; the source text is never edited).
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

const SCHEMA_V2: &str = "
CREATE TABLE cache_heals(
  seq INTEGER PRIMARY KEY AUTOINCREMENT,
  task_id TEXT NOT NULL REFERENCES tasks(id),
  cached_phase TEXT NOT NULL,
  replayed_phase TEXT NOT NULL,
  ts INTEGER NOT NULL
) STRICT;
";

const LEGACY_EPOCH: &str = "19c0ffee";
const LEGACY_TASK: &str = "t-legacy";

fn db(name: &str) -> Result<PathBuf, Box<dyn Error>> {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("hee4-core-migrate");
    std::fs::create_dir_all(&dir)?;
    let path = dir.join(format!("{name}.sqlite"));
    for suffix in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{}{suffix}", path.display()));
    }
    Ok(path)
}

fn legacy_op() -> OperationKey {
    OperationKey {
        principal: "luke".into(),
        action: "task.submit".into(),
        version: 1,
        idem_key: "k-legacy".into(),
    }
}

/// A v2 file holding one task, its Admit event, one operation and one receipt.
fn build_v2(path: &PathBuf) -> Result<Receipt, Box<dyn Error>> {
    let conn = Connection::open(path)?;
    conn.execute_batch(SCHEMA_V1)?;
    conn.execute_batch(SCHEMA_V2)?;
    conn.pragma_update(None, "user_version", 2)?;
    conn.execute(
        "INSERT INTO meta(key, value) VALUES ('epoch', ?1), ('recovery_complete', '1')",
        [LEGACY_EPOCH],
    )?;
    conn.execute(
        "INSERT INTO tasks(id, phase, cancel, generation, updated_ts) VALUES (?1, 'admitted', 0, 0, 1)",
        [LEGACY_TASK],
    )?;
    conn.execute(
        "INSERT INTO events(task_id, event_json, ts) VALUES (?1, ?2, 1)",
        params![LEGACY_TASK, serde_json::to_string(&Event::Admit)?],
    )?;
    let op = legacy_op();
    conn.execute(
        "INSERT INTO operations(principal, action, version, idem_key, request_sha256, task_id,
           result_json, ts) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 1)",
        params![
            op.principal,
            op.action,
            op.version,
            op.idem_key,
            Sha256Hex::digest(b"spec-legacy").to_string(),
            LEGACY_TASK,
            json!({"ok": true}).to_string()
        ],
    )?;
    let receipt = Receipt::seal(
        Sha256Hex::GENESIS,
        ReceiptBody {
            id: "r-legacy".parse()?,
            task_id: LEGACY_TASK.parse()?,
            decision: Decision {
                verdict: Verdict::Pass,
            },
            observed: vec![],
        },
    );
    conn.execute(
        "INSERT INTO receipts(id, task_id, json, hash_prev, hash_self, ts)
         VALUES (?1, ?2, ?3, ?4, ?5, 1)",
        params![
            receipt.id().as_str(),
            LEGACY_TASK,
            serde_json::to_string(&receipt)?,
            receipt.hash_prev().to_string(),
            receipt.hash_self().to_string()
        ],
    )?;
    Ok(receipt)
}

fn migration_rows(path: &PathBuf) -> Result<Vec<String>, Box<dyn Error>> {
    let conn = Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let mut stmt =
        conn.prepare("SELECT key FROM meta WHERE key LIKE 'migration:%' ORDER BY key")?;
    let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

fn meta_count(path: &PathBuf) -> Result<i64, Box<dyn Error>> {
    let conn = Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    Ok(conn.query_row("SELECT count(*) FROM meta", [], |r| r.get(0))?)
}

#[test]
fn a_v2_file_gains_the_named_rows_and_keeps_its_data() -> R {
    let path = db("v2")?;
    let receipt = build_v2(&path)?;
    let task: TaskId = LEGACY_TASK.parse()?;
    let store = Store::open(&path)?;
    assert_eq!(store.schema_version()?, 5);
    assert_eq!(
        migration_rows(&path)?,
        [
            "migration:m001_v1",
            "migration:m002_cache_heals",
            "migration:m003_operations_subject",
            "migration:m004_serve_cgroup",
            "migration:m005_checkpoints"
        ]
    );
    assert_eq!(store.epoch()?, LEGACY_EPOCH, "epoch is kept, not re-minted");
    assert_eq!(store.history(&task)?, [Event::Admit]);
    assert_eq!(store.chain_head(&task)?, receipt.hash_self());
    let row = store
        .operation_by_key(&legacy_op())?
        .ok_or("operation row lost")?;
    let expected = format!(
        "op-{}",
        &Sha256Hex::digest(b"luke\ntask.submit\n1\nk-legacy").to_string()[..24]
    );
    assert_eq!(row.operation_id, expected);
    assert_eq!(row.subject.as_deref(), Some(LEGACY_TASK));
    assert_eq!(row.task_id, Some(task.clone()));
    assert_eq!(row.result, json!({"ok": true}));
    assert_eq!(
        store
            .last_operation_for(LEGACY_TASK)?
            .map(|r| r.operation_id),
        Some(expected)
    );
    assert!(!store.recovery_complete()?, "open resets the gate");
    let mode = std::fs::metadata(&path)?.permissions().mode() & 0o777;
    assert_eq!(mode, 0o600, "ledger mode after open");
    let boot = store.boot()?;
    let rows = meta_count(&path)?;
    drop(store);

    let again = Store::open(&path)?;
    assert_eq!(again.schema_version()?, 5);
    assert_eq!(meta_count(&path)?, rows, "a second open adds no meta row");
    assert_eq!(again.boot()?, boot + 1);
    assert_eq!(again.history(&task)?, [Event::Admit]);
    assert_eq!(
        again
            .operation_by_key(&legacy_op())?
            .map(|r| r.operation_id),
        Some(format!(
            "op-{}",
            &Sha256Hex::digest(b"luke\ntask.submit\n1\nk-legacy").to_string()[..24]
        ))
    );
    Ok(())
}

#[test]
fn a_row_naming_an_unknown_migration_is_refused_by_name() -> R {
    let path = db("future")?;
    build_v2(&path)?;
    {
        let conn = Connection::open(&path)?;
        conn.execute(
            "INSERT INTO meta(key, value) VALUES ('migration:m999_future', '1')",
            [],
        )?;
    }
    let err = Store::open(&path);
    assert!(
        matches!(&err, Err(StoreError::UnknownMigration(name)) if name == "m999_future"),
        "{err:?}"
    );
    Ok(())
}
