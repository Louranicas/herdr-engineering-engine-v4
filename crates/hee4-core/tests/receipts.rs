//! The receipt-checkpoint family: `checkpoint_if_due`, `verify_chain`, `verify_ledger` and
//! `open_read_only`, positive and negative, on real ledger files. The last two cases leave
//! `clean.sqlite` and `planted.sqlite` under `CARGO_TARGET_TMPDIR/hee4-core-receipts` for
//! `hee4 verify-ledger` and print `planted receipt=<id>` for the byte they planted.
//!
//! `Receipt::seal` is allowed here: `one_sealer` scans `src/` only.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::error::Error;
use std::num::NonZeroU64;
use std::path::PathBuf;

use hee4_contracts::{
    Budgets, Decision, Event, GitSha, Observation, ObservationId, Outcome, Receipt, ReceiptBody,
    Settlement, Sha256Hex, TaskId, ToolId, Verdict,
};
use hee4_core::receipts::{ChainCause, ChainFault, VerifyError};
use hee4_core::{Observations, Store, StoreError, reconcile};
use rusqlite::Connection;

type R = Result<(), Box<dyn Error>>;

/// The test cadence: three receipts per checkpoint. Typed, never a bare integer at a call site.
const EVERY_THREE: NonZeroU64 = NonZeroU64::new(3).unwrap();

/// The production cadence, read from the budget field the wave-3 caller will hand down.
fn budget_every() -> NonZeroU64 {
    NonZeroU64::new(Budgets::DEFAULT.ledger.checkpoint_every).expect("budget field is non-zero")
}

fn db(name: &str) -> Result<PathBuf, Box<dyn Error>> {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("hee4-core-receipts");
    std::fs::create_dir_all(&dir)?;
    let path = dir.join(format!("{name}.sqlite"));
    for suffix in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{}{suffix}", path.display()));
    }
    Ok(path)
}

/// A fresh store whose (empty) reconcile has completed, so `Dispatch` is open.
fn ready(name: &str) -> Result<(Store, PathBuf), Box<dyn Error>> {
    let path = db(name)?;
    let store = Store::open(&path)?;
    assert!(reconcile(&store, &Observations::default())?.complete);
    Ok((store, path))
}

fn observation() -> Result<Observation, Box<dyn Error>> {
    Ok(Observation {
        source: "cargo-test".parse()?,
        input_sha256: Sha256Hex::digest(b"input"),
        tool: ToolId {
            name: "cargo".parse()?,
            version: "1.99".parse()?,
        },
        head_sha: "a".repeat(40).parse::<GitSha>()?,
        outcome: Outcome::Pass,
        evidence: vec![],
        advisory: false,
        elapsed_ms: 1,
        budget_ms: 10,
    })
}

/// Admit, Dispatch, Settle(Ready): a task in `verifying`, ready to be decided.
fn seed_task(store: &Store, name: &str) -> Result<TaskId, Box<dyn Error>> {
    let task: TaskId = name.parse()?;
    store.apply(&task, Event::Admit)?;
    store.apply(&task, Event::Dispatch)?;
    store.apply(&task, Event::Settle(Settlement::Ready))?;
    Ok(task)
}

/// Ledger one observation and append one sealed receipt `r-<task>-<n>` after the chain head.
fn append(store: &Store, task: &TaskId, n: usize) -> Result<Receipt, Box<dyn Error>> {
    let obs: ObservationId = format!("o-{task}-{n}").parse()?;
    store.record_observation(task, &obs, &observation()?)?;
    let receipt = Receipt::seal(
        store.chain_head(task)?,
        ReceiptBody {
            id: format!("r-{task}-{n}").parse()?,
            task_id: task.clone(),
            decision: Decision {
                verdict: Verdict::Pass,
            },
            observed: vec![obs],
        },
    );
    store.append_receipt(&receipt)?;
    Ok(receipt)
}

/// Replace one byte with a different byte of the same class (tests/receipt.rs `twin`).
fn twin(b: u8) -> u8 {
    match b {
        b'0'..=b'8' | b'a'..=b'y' | b'A'..=b'Y' => b + 1,
        b'9' => b'0',
        b'z' => b'a',
        b'Z' => b'A',
        _ => b,
    }
}

fn hash_selfs_upto(conn: &Connection, upto: i64) -> Result<Vec<Sha256Hex>, Box<dyn Error>> {
    let mut stmt = conn.prepare("SELECT hash_self FROM receipts WHERE seq <= ?1 ORDER BY seq")?;
    let rows = stmt.query_map([upto], |r| r.get::<_, String>(0))?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row?.parse()?);
    }
    Ok(out)
}

fn meta(conn: &Connection, key: &str) -> Result<Option<String>, Box<dyn Error>> {
    let mut stmt = conn.prepare("SELECT value FROM meta WHERE key = ?1")?;
    let mut rows = stmt.query_map([key], |r| r.get::<_, String>(0))?;
    Ok(rows.next().transpose()?)
}

fn user_version(conn: &Connection) -> Result<i64, Box<dyn Error>> {
    Ok(conn.query_row("PRAGMA user_version", [], |r| r.get(0))?)
}

fn fault(err: VerifyError) -> ChainFault {
    match err {
        VerifyError::Fault(f) => f,
        VerifyError::Store(e) => panic!("expected a chain fault, got a store error: {e}"),
    }
}

/// Case 1 (leaves `clean.sqlite`): due only at `every`, once, and the ledger verifies.
#[test]
fn checkpoint_is_due_at_every_receipts_and_only_once() -> R {
    let (store, _) = ready("clean")?;
    let t = seed_task(&store, "t1")?;
    append(&store, &t, 1)?;
    append(&store, &t, 2)?;
    assert_eq!(store.checkpoint_if_due(EVERY_THREE)?, None);
    append(&store, &t, 3)?;
    let cp = store
        .checkpoint_if_due(EVERY_THREE)?
        .ok_or("due after the third receipt")?;
    assert_eq!((cp.count, cp.upto_receipt_seq), (3, 3));
    assert_eq!(
        store.checkpoint_if_due(EVERY_THREE)?,
        None,
        "nothing new since the checkpoint"
    );
    let report = store.verify_ledger()?;
    assert_eq!(
        (
            report.receipts,
            report.tasks,
            report.checkpoints,
            report.root
        ),
        (3, 1, 1, cp.root)
    );
    Ok(())
}

/// Case 2: two checkpoints over six receipts across two tasks, each root re-derived.
#[test]
fn two_checkpoints_over_two_tasks_re_derive() -> R {
    let (store, path) = ready("two")?;
    let a = seed_task(&store, "t-a")?;
    let b = seed_task(&store, "t-b")?;
    for n in 1..=3 {
        append(&store, &a, n)?;
    }
    let first = store.checkpoint_if_due(EVERY_THREE)?.ok_or("first")?;
    for n in 1..=3 {
        append(&store, &b, n)?;
    }
    let second = store.checkpoint_if_due(EVERY_THREE)?.ok_or("second")?;
    assert_eq!((first.count, second.count), (3, 6));
    assert_eq!((first.upto_receipt_seq, second.upto_receipt_seq), (3, 6));
    let conn = Connection::open(&path)?;
    for cp in [first, second] {
        let hashes = hash_selfs_upto(&conn, cp.upto_receipt_seq)?;
        assert_eq!(hashes.len() as u64, cp.count);
        assert_eq!(Receipt::checkpoint(&hashes), cp.root, "seq {}", cp.seq);
    }
    let report = store.verify_ledger()?;
    assert_eq!(
        (
            report.receipts,
            report.tasks,
            report.checkpoints,
            report.root
        ),
        (6, 2, 2, second.root)
    );
    Ok(())
}

/// Case 3: an untouched file passes both verbs and `head` equals `chain_head`.
#[test]
fn untouched_file_verifies_and_head_equals_chain_head() -> R {
    let (store, _) = ready("untouched")?;
    let a = seed_task(&store, "t-a")?;
    let b = seed_task(&store, "t-b")?;
    append(&store, &a, 1)?;
    append(&store, &b, 1)?;
    let last = append(&store, &a, 2)?;
    let chain = store.verify_chain(&a)?;
    assert_eq!((chain.task.clone(), chain.receipts), (a.clone(), 2));
    assert_eq!(chain.head, store.chain_head(&a)?);
    assert_eq!(chain.head, last.hash_self());
    assert_eq!(store.verify_chain(&b)?.receipts, 1);
    let empty: TaskId = "never".parse()?;
    assert_eq!(store.verify_chain(&empty)?.head, Sha256Hex::GENESIS);
    let report = store.verify_ledger()?;
    assert_eq!(
        (report.receipts, report.tasks, report.checkpoints),
        (3, 2, 0)
    );
    assert_eq!(report.root, Receipt::checkpoint(&[]));
    assert_eq!(
        store.checkpoint_if_due(budget_every())?,
        None,
        "the budget cadence is far from due"
    );
    Ok(())
}

/// Case 4 (leaves `planted.sqlite`): one twinned byte in the middle receipt's JSON is refused
/// by that receipt's id, by both verbs.
#[test]
fn planted_byte_is_refused_by_receipt_id() -> R {
    let (store, path) = ready("planted")?;
    let t = seed_task(&store, "t1")?;
    append(&store, &t, 1)?;
    let middle = append(&store, &t, 2)?;
    append(&store, &t, 3)?;
    store.checkpoint_if_due(EVERY_THREE)?.ok_or("checkpoint")?;
    assert!(store.verify_ledger().is_ok(), "clean before the plant");

    let conn = Connection::open(&path)?;
    conn.execute_batch("DROP TRIGGER receipts_no_update")?;
    let json: String = conn.query_row(
        "SELECT json FROM receipts WHERE id = ?1",
        [middle.id().as_str()],
        |r| r.get(0),
    )?;
    let mut bytes = json.into_bytes();
    let marker = b"\"task_id\":\"";
    let at = bytes
        .windows(marker.len())
        .position(|w| w == marker)
        .ok_or("task_id in json")?
        + marker.len();
    let t = twin(bytes[at]);
    assert_ne!(t, bytes[at]);
    bytes[at] = t;
    let planted = String::from_utf8(bytes)?;
    conn.execute(
        "UPDATE receipts SET json = ?1 WHERE id = ?2",
        [&planted, middle.id().as_str()],
    )?;
    println!("planted receipt={}", middle.id());

    let f = fault(store.verify_ledger().err().ok_or("plant must be refused")?);
    assert_eq!(f.receipt.as_ref(), Some(middle.id()));
    assert_eq!(f.seq, 2);
    assert!(
        matches!(f.cause, ChainCause::SelfHash | ChainCause::Unparsable),
        "{f}"
    );
    let task: TaskId = "t1".parse()?;
    let g = fault(store.verify_chain(&task).err().ok_or("chain")?);
    assert_eq!((g.receipt, g.seq, g.cause), (f.receipt, f.seq, f.cause));
    Ok(())
}

/// Case 5: only the `hash_self` column of the last receipt is altered: `ColumnMismatch`.
#[test]
fn tampered_hash_self_column_is_a_column_mismatch() -> R {
    let (store, path) = ready("column")?;
    let t = seed_task(&store, "t1")?;
    append(&store, &t, 1)?;
    let last = append(&store, &t, 2)?;
    let conn = Connection::open(&path)?;
    conn.execute_batch("DROP TRIGGER receipts_no_update")?;
    conn.execute(
        "UPDATE receipts SET hash_self = ?1 WHERE id = ?2",
        [
            Sha256Hex::digest(b"planted").to_string().as_str(),
            last.id().as_str(),
        ],
    )?;
    let f = fault(store.verify_ledger().err().ok_or("refused")?);
    assert_eq!(
        (f.receipt.as_ref(), f.seq, f.cause),
        (Some(last.id()), 2, ChainCause::ColumnMismatch)
    );
    let g = fault(store.verify_chain(&t).err().ok_or("chain")?);
    assert_eq!(g, f);
    Ok(())
}

/// Case 6: deleting the last receipt after a checkpoint: every per-task chain still passes
/// (a shorter prefix is a valid chain), the checkpoint names the truncation.
#[test]
fn deleting_a_checkpointed_receipt_is_a_root_mismatch() -> R {
    let (store, path) = ready("truncated")?;
    let a = seed_task(&store, "t-a")?;
    let b = seed_task(&store, "t-b")?;
    for n in 1..=3 {
        append(&store, &a, n)?;
        append(&store, &b, n)?;
    }
    let cp = store.checkpoint_if_due(EVERY_THREE)?.ok_or("checkpoint")?;
    assert_eq!(cp.count, 6);
    let conn = Connection::open(&path)?;
    conn.execute_batch("DROP TRIGGER receipts_no_delete")?;
    let delete = "DELETE FROM receipts WHERE seq = (SELECT max(seq) FROM receipts)";
    assert!(
        conn.execute(delete, []).is_err(),
        "with foreign keys on, `checkpoints.upto_receipt_seq` refuses the delete by itself"
    );
    conn.pragma_update(None, "foreign_keys", "OFF")?;
    assert_eq!(conn.execute(delete, [])?, 1);
    assert_eq!(store.verify_chain(&a)?.receipts, 3);
    assert_eq!(
        store.verify_chain(&b)?.receipts,
        2,
        "the per-task chain cannot see the truncation"
    );
    let f = fault(store.verify_ledger().err().ok_or("refused")?);
    assert_eq!(
        f,
        ChainFault {
            receipt: None,
            seq: cp.seq,
            cause: ChainCause::RootMismatch
        }
    );
    Ok(())
}

/// Case 7: `open_read_only` verifies, writes nothing (`recovery_complete`, `user_version`
/// unchanged; a write through it is refused by SQLite), and refuses a pre-checkpoint file by
/// name.
#[test]
fn open_read_only_verifies_writes_nothing_and_refuses_an_unmigrated_file() -> R {
    let (store, path) = ready("ro")?;
    let t = seed_task(&store, "t1")?;
    for n in 1..=3 {
        append(&store, &t, n)?;
    }
    store.checkpoint_if_due(EVERY_THREE)?.ok_or("checkpoint")?;
    drop(store);

    let conn = Connection::open(&path)?;
    let before = (meta(&conn, "recovery_complete")?, user_version(&conn)?);
    assert_eq!(before.1, 5, "five migrations applied");
    let ro = Store::open_read_only(&path)?;
    let report = ro.verify_ledger()?;
    assert_eq!((report.receipts, report.checkpoints), (3, 1));
    assert_eq!(ro.verify_chain(&t)?.receipts, 3);
    assert_eq!(ro.serve_cgroup(), "");
    drop(ro);
    assert_eq!(
        (meta(&conn, "recovery_complete")?, user_version(&conn)?),
        before
    );

    // A write through the read-only door is refused by SQLite, not by policy.
    let (due, due_path) = ready("ro-due")?;
    let t2 = seed_task(&due, "t2")?;
    for n in 1..=3 {
        append(&due, &t2, n)?;
    }
    drop(due);
    let ro = Store::open_read_only(&due_path)?;
    assert!(
        matches!(
            ro.checkpoint_if_due(EVERY_THREE),
            Err(StoreError::Sqlite(_))
        ),
        "a due checkpoint cannot be written through the read-only door"
    );
    assert_eq!(ro.verify_ledger()?.checkpoints, 0);
    drop(ro);

    // A copy without the m005 row is refused by name.
    let copy = db("ro-copy")?;
    conn.execute("VACUUM INTO ?1", [copy.to_str().ok_or("utf8")?])?;
    let copied = Connection::open(&copy)?;
    assert_eq!(
        copied.execute(
            "DELETE FROM meta WHERE key = 'migration:m005_checkpoints'",
            []
        )?,
        1
    );
    drop(copied);
    let refused = Store::open_read_only(&copy).err().ok_or("must refuse")?;
    assert!(
        matches!(&refused, StoreError::Corrupt { detail, .. } if detail == "ledger not migrated to checkpoints"),
        "{refused}"
    );
    assert!(
        matches!(
            Store::open_read_only(&db("absent")?),
            Err(StoreError::Sqlite(_))
        ),
        "a missing file is a SQLite open error"
    );
    Ok(())
}
