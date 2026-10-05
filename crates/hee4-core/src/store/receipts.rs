//! The receipt-checkpoint family (I4): an append-only `checkpoints` table holding the RFC 6962
//! Merkle root ([`Receipt::checkpoint`]) over every `receipts.hash_self` in `seq` order up to
//! one row, and the offline verify verbs that re-derive every link, every seal and every root
//! from the file alone (the Rekor rule: the verifier is not the writer). Chains are per task;
//! a checkpoint spans every task, so a cross-task truncation the per-task chains cannot see is
//! visible here. Nothing in this file writes `receipts`, `events` or `tasks`.

use std::fmt;
use std::num::NonZeroU64;
use std::path::Path;

use hee4_contracts::{BreakCause, Receipt, ReceiptId, Sha256Hex, TaskId};
use rusqlite::{Connection, OpenFlags, OptionalExtension, Transaction, params};

use super::migrations::Migration;
use super::{Store, StoreError, meta_get, now_ms};

/// The migration's name; `open` writes the `meta` row `migration:<name>` and
/// [`Store::open_read_only`] refuses a file without it.
const MIGRATION_NAME: &str = "m005_checkpoints";

/// m005: the `checkpoints` table, append-only like `receipts`.
const SCHEMA_M005: &str = "
CREATE TABLE checkpoints(
  seq INTEGER PRIMARY KEY AUTOINCREMENT,
  upto_receipt_seq INTEGER NOT NULL UNIQUE REFERENCES receipts(seq),
  count INTEGER NOT NULL CHECK (count >= 1),
  root TEXT NOT NULL CHECK (length(root) = 64),
  ts INTEGER NOT NULL
) STRICT;
CREATE TRIGGER checkpoints_no_update BEFORE UPDATE ON checkpoints
  BEGIN SELECT RAISE(ABORT, 'checkpoints are append-only'); END;
CREATE TRIGGER checkpoints_no_delete BEFORE DELETE ON checkpoints
  BEGIN SELECT RAISE(ABORT, 'checkpoints are append-only'); END;
";

/// This family's one entry in [`super::migrations::MIGRATIONS`].
pub(super) const fn m005_checkpoints() -> Migration {
    Migration {
        name: MIGRATION_NAME,
        apply: apply_m005,
    }
}

fn apply_m005(tx: &Transaction<'_>) -> Result<(), StoreError> {
    tx.execute_batch(SCHEMA_M005)?;
    Ok(())
}

/// One `checkpoints` row: the Merkle root over the first `count` receipts (every receipt with
/// `seq <= upto_receipt_seq`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Checkpoint {
    /// The row's own sequence.
    pub seq: i64,
    /// The highest `receipts.seq` the root covers.
    pub upto_receipt_seq: i64,
    /// How many receipts the root covers.
    pub count: u64,
    /// [`Receipt::checkpoint`] over those receipts' `hash_self`, in `seq` order.
    pub root: Sha256Hex,
}

/// What a verify verb found wrong.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChainCause {
    /// `hash_prev` is not the previous receipt's `hash_self` (K0 [`BreakCause::Link`]).
    Link,
    /// `hash_self` is not the digest of the receipt's canonical body (K0 [`BreakCause::SelfHash`]).
    SelfHash,
    /// A stored `id`, `task_id`, `hash_prev` or `hash_self` column disagrees with the sealed
    /// receipt's JSON; the fault names the sealed id.
    ColumnMismatch,
    /// The row's JSON is not a `Receipt`, or a hash column is not 64 lowercase hex digits.
    Unparsable,
    /// A checkpoint's `root` or `count` is not what the receipts it covers re-derive to.
    RootMismatch,
}

impl ChainCause {
    /// The variant's name, as `hee4 verify-ledger` prints it.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Link => "Link",
            Self::SelfHash => "SelfHash",
            Self::ColumnMismatch => "ColumnMismatch",
            Self::Unparsable => "Unparsable",
            Self::RootMismatch => "RootMismatch",
        }
    }
}

impl fmt::Display for ChainCause {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<BreakCause> for ChainCause {
    fn from(cause: BreakCause) -> Self {
        match cause {
            BreakCause::Link => Self::Link,
            BreakCause::SelfHash => Self::SelfHash,
        }
    }
}

fn receipt_or_none(receipt: Option<&ReceiptId>) -> String {
    receipt.map_or_else(|| "none".to_owned(), ToString::to_string)
}

/// The first row at which the ledger fails to re-derive. `receipt` is `None` only for
/// [`ChainCause::RootMismatch`], where `seq` is the checkpoint's; otherwise `seq` is the
/// receipt row's.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("ledger breaks at receipt={} seq={seq} cause={cause}", receipt_or_none(.receipt.as_ref()))]
pub struct ChainFault {
    /// The receipt named by its id.
    pub receipt: Option<ReceiptId>,
    /// The row's sequence (`receipts.seq`, or `checkpoints.seq` for a root mismatch).
    pub seq: i64,
    /// What failed there.
    pub cause: ChainCause,
}

/// Why a verify verb returned no report: the file could not be read, or it re-derived wrong.
#[derive(Debug, thiserror::Error)]
pub enum VerifyError {
    /// SQLite or a row this binary cannot read; not a verdict on the chain.
    #[error(transparent)]
    Store(#[from] StoreError),
    /// The chain or a checkpoint fails to re-derive: the verdict is FAIL at that row.
    #[error(transparent)]
    Fault(#[from] ChainFault),
}

/// One task's chain, re-derived from its rows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChainReport {
    /// The task.
    pub task: TaskId,
    /// How many receipts the chain holds.
    pub receipts: u64,
    /// The last `hash_self`, or [`Sha256Hex::GENESIS`] for an empty chain.
    pub head: Sha256Hex,
}

/// The whole ledger, re-derived: every task's chain and every checkpoint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LedgerReport {
    /// Receipts over every task.
    pub receipts: u64,
    /// Tasks that hold at least one receipt.
    pub tasks: u64,
    /// Checkpoint rows, each re-derived.
    pub checkpoints: u64,
    /// The latest checkpoint's root, or [`Receipt::checkpoint`] of nothing when there is none.
    pub root: Sha256Hex,
}

/// A `receipts` row as SQLite hands it over.
struct ReceiptRow {
    seq: i64,
    id: String,
    json: String,
    hash_prev: String,
    hash_self: String,
}

fn receipt_rows(conn: &Connection, task: &TaskId) -> Result<Vec<ReceiptRow>, StoreError> {
    let mut stmt = conn.prepare(
        "SELECT seq, id, json, hash_prev, hash_self FROM receipts WHERE task_id = ?1 ORDER BY seq",
    )?;
    let rows = stmt.query_map([task.as_str()], |r| {
        Ok(ReceiptRow {
            seq: r.get(0)?,
            id: r.get(1)?,
            json: r.get(2)?,
            hash_prev: r.get(3)?,
            hash_self: r.get(4)?,
        })
    })?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

/// Every `hash_self` with `seq <= upto`, in `seq` order, parsed.
fn hash_selfs_upto(conn: &Connection, upto: i64) -> Result<Vec<Sha256Hex>, StoreError> {
    let mut stmt =
        conn.prepare("SELECT seq, hash_self FROM receipts WHERE seq <= ?1 ORDER BY seq")?;
    let rows = stmt.query_map([upto], |r| {
        Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?))
    })?;
    let mut out = Vec::new();
    for row in rows {
        let (seq, text) = row?;
        out.push(text.parse().map_err(|e| StoreError::Corrupt {
            task: String::new(),
            detail: format!("receipts.seq={seq} hash_self: {e}"),
        })?);
    }
    Ok(out)
}

fn checkpoint_rows(conn: &Connection) -> Result<Vec<Checkpoint>, StoreError> {
    let mut stmt =
        conn.prepare("SELECT seq, upto_receipt_seq, count, root FROM checkpoints ORDER BY seq")?;
    let rows = stmt.query_map([], |r| {
        Ok((
            r.get::<_, i64>(0)?,
            r.get::<_, i64>(1)?,
            r.get::<_, i64>(2)?,
            r.get::<_, String>(3)?,
        ))
    })?;
    let mut out = Vec::new();
    for row in rows {
        let (seq, upto_receipt_seq, count, root) = row?;
        let root = root.parse().map_err(|e| StoreError::Corrupt {
            task: String::new(),
            detail: format!("checkpoints.seq={seq} root: {e}"),
        })?;
        out.push(Checkpoint {
            seq,
            upto_receipt_seq,
            count: u64::try_from(count).unwrap_or(0),
            root,
        });
    }
    Ok(out)
}

/// Task ids that hold receipts, in the order their first receipt was written.
fn tasks_with_receipts(conn: &Connection) -> Result<Vec<TaskId>, StoreError> {
    let mut stmt =
        conn.prepare("SELECT task_id FROM receipts GROUP BY task_id ORDER BY min(seq)")?;
    let ids = stmt.query_map([], |r| r.get::<_, String>(0))?;
    let mut out = Vec::new();
    for id in ids {
        let id = id?;
        out.push(id.parse().map_err(|e| StoreError::Corrupt {
            task: id.clone(),
            detail: format!("receipts.task_id: {e}"),
        })?);
    }
    Ok(out)
}

impl Store {
    /// Open an existing ledger for verification only: `SQLITE_OPEN_READ_ONLY`, no pragma
    /// write, no migration, no `meta` write (unlike [`Store::open`], which migrates, bumps
    /// `boot` and resets `recovery_complete`). The file must already carry the checkpoints
    /// migration. `serve_cgroup()` is empty on such a store.
    ///
    /// # Errors
    /// SQLite errors (a missing file among them); [`StoreError::Corrupt`] with detail
    /// `ledger not migrated to checkpoints` for a file without `migration:m005_checkpoints`.
    pub fn open_read_only(path: &Path) -> Result<Self, StoreError> {
        let conn = Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )?;
        conn.busy_timeout(std::time::Duration::from_secs(5))?;
        if meta_get(&conn, &format!("migration:{MIGRATION_NAME}"))?.is_none() {
            return Err(StoreError::Corrupt {
                task: String::new(),
                detail: "ledger not migrated to checkpoints".into(),
            });
        }
        Ok(Self {
            conn,
            path: path.to_path_buf(),
            serve_cgroup: String::new(),
        })
    }

    /// Write one checkpoint when at least `every` receipts have been appended since the last
    /// one (or since the start): the root is [`Receipt::checkpoint`] over every `hash_self`
    /// up to `max(receipts.seq)`, so each root covers a prefix of the whole log. One Immediate
    /// transaction; `None` writes nothing, and a second call right after `Some` is `None`.
    /// `every` is the caller's budget field (`ledger.checkpoint_every`), never a literal here.
    ///
    /// # Errors
    /// SQLite errors; [`StoreError::Corrupt`] for a `hash_self` column that does not parse.
    pub fn checkpoint_if_due(&self, every: NonZeroU64) -> Result<Option<Checkpoint>, StoreError> {
        let tx = self.begin()?;
        let max: Option<i64> = tx.query_row("SELECT max(seq) FROM receipts", [], |r| r.get(0))?;
        let Some(max) = max else {
            return Ok(None);
        };
        let last: Option<i64> = tx
            .query_row(
                "SELECT upto_receipt_seq FROM checkpoints ORDER BY seq DESC LIMIT 1",
                [],
                |r| r.get(0),
            )
            .optional()?;
        let pending: i64 = tx.query_row(
            "SELECT count(*) FROM receipts WHERE seq > ?1",
            [last.unwrap_or(0)],
            |r| r.get(0),
        )?;
        if u64::try_from(pending).unwrap_or(0) < every.get() {
            return Ok(None);
        }
        let hashes = hash_selfs_upto(&tx, max)?;
        let root = Receipt::checkpoint(&hashes);
        let count = i64::try_from(hashes.len()).map_err(|_| StoreError::Corrupt {
            task: String::new(),
            detail: "receipt count overflow".into(),
        })?;
        tx.execute(
            "INSERT INTO checkpoints(upto_receipt_seq, count, root, ts) VALUES (?1, ?2, ?3, ?4)",
            params![max, count, root.to_string(), now_ms()],
        )?;
        let seq = tx.last_insert_rowid();
        tx.commit()?;
        Ok(Some(Checkpoint {
            seq,
            upto_receipt_seq: max,
            count: u64::try_from(count).unwrap_or(0),
            root,
        }))
    }

    /// Re-derive `task`'s chain from its rows in `seq` order: each row's JSON must parse as a
    /// `Receipt`, its `id`, `task_id`, `hash_prev` and `hash_self` columns must equal the
    /// sealed values (`task_id` must be `task`), and the
    /// whole chain must pass [`Receipt::verify_chain`] from `GENESIS`.
    ///
    /// # Errors
    /// [`VerifyError::Fault`] naming the first failing receipt id, its `seq` and the cause;
    /// [`VerifyError::Store`] when the rows cannot be read.
    pub fn verify_chain(&self, task: &TaskId) -> Result<ChainReport, VerifyError> {
        let rows = receipt_rows(&self.conn, task)?;
        let mut chain = Vec::with_capacity(rows.len());
        let mut ids = Vec::with_capacity(rows.len());
        for row in &rows {
            let receipt = row.id.parse::<ReceiptId>().ok();
            let fault = |cause: ChainCause| ChainFault {
                receipt: receipt.clone(),
                seq: row.seq,
                cause,
            };
            let parsed: Receipt =
                serde_json::from_str(&row.json).map_err(|_| fault(ChainCause::Unparsable))?;
            let prev: Sha256Hex = row
                .hash_prev
                .parse()
                .map_err(|_| fault(ChainCause::Unparsable))?;
            let own: Sha256Hex = row
                .hash_self
                .parse()
                .map_err(|_| fault(ChainCause::Unparsable))?;
            // Every sealed field that has its own column must agree with it: `chain_head` and
            // `append_receipt` select by `task_id`, and a fault names the receipt by `id`.
            let sealed = Some(parsed.id().clone());
            if row.id != parsed.id().as_str()
                || parsed.task_id() != task
                || prev != parsed.hash_prev()
                || own != parsed.hash_self()
            {
                return Err(ChainFault {
                    receipt: sealed,
                    seq: row.seq,
                    cause: ChainCause::ColumnMismatch,
                }
                .into());
            }
            chain.push(parsed);
            ids.push((sealed, row.seq));
        }
        if let Err(brk) = Receipt::verify_chain(&chain) {
            let (receipt, seq) = ids.get(brk.index).cloned().unwrap_or((None, 0));
            return Err(ChainFault {
                receipt,
                seq,
                cause: brk.cause.into(),
            }
            .into());
        }
        Ok(ChainReport {
            task: task.clone(),
            receipts: u64::try_from(chain.len()).unwrap_or(u64::MAX),
            head: chain.last().map_or(Sha256Hex::GENESIS, Receipt::hash_self),
        })
    }

    /// Re-derive the whole ledger: [`Store::verify_chain`] for every task that holds receipts
    /// (in first-receipt order), then every checkpoint row's `count` and `root` against the
    /// `hash_self` columns with `seq <= upto_receipt_seq`.
    ///
    /// # Errors
    /// [`VerifyError::Fault`]: a chain fault as `verify_chain` reports it, or
    /// [`ChainCause::RootMismatch`] with `receipt: None` and the checkpoint's `seq`;
    /// [`VerifyError::Store`] when the rows cannot be read.
    pub fn verify_ledger(&self) -> Result<LedgerReport, VerifyError> {
        let tasks = tasks_with_receipts(&self.conn)?;
        let mut receipts = 0_u64;
        for task in &tasks {
            receipts = receipts.saturating_add(self.verify_chain(task)?.receipts);
        }
        let checkpoints = checkpoint_rows(&self.conn)?;
        for cp in &checkpoints {
            let hashes = hash_selfs_upto(&self.conn, cp.upto_receipt_seq)?;
            let count = u64::try_from(hashes.len()).unwrap_or(u64::MAX);
            if count != cp.count || Receipt::checkpoint(&hashes) != cp.root {
                return Err(ChainFault {
                    receipt: None,
                    seq: cp.seq,
                    cause: ChainCause::RootMismatch,
                }
                .into());
            }
        }
        Ok(LedgerReport {
            receipts,
            tasks: u64::try_from(tasks.len()).unwrap_or(u64::MAX),
            checkpoints: u64::try_from(checkpoints.len()).unwrap_or(u64::MAX),
            root: checkpoints
                .last()
                .map_or_else(|| Receipt::checkpoint(&[]), |cp| cp.root),
        })
    }
}
