//! The readers and verbs this slice adds: mode convergence on open, `serve_cgroup` in `meta`
//! and on admitted tasks, `boot` vs `epoch`, `event_high_water`, `cursor_check` (R13's inputs)
//! and `mark_restored_from`.

use std::error::Error;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use hee4_contracts::{Event, TaskId};
use hee4_core::{CursorVerdict, Observations, OperationKey, Store, reconcile};
use rusqlite::{Connection, OpenFlags};
use serde_json::json;

type R = Result<(), Box<dyn Error>>;

fn db(name: &str) -> Result<PathBuf, Box<dyn Error>> {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("hee4-core-readbacks");
    std::fs::create_dir_all(&dir)?;
    let path = dir.join(format!("{name}.sqlite"));
    for suffix in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{}{suffix}", path.display()));
    }
    Ok(path)
}

fn sidecar(path: &Path, suffix: &str) -> PathBuf {
    PathBuf::from(format!("{}{suffix}", path.display()))
}

fn mode(path: &Path) -> Result<u32, Box<dyn Error>> {
    Ok(std::fs::metadata(path)?.permissions().mode() & 0o777)
}

fn chmod(path: &Path, mode: u32) -> R {
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode))?;
    Ok(())
}

fn ro(path: &Path) -> Result<Connection, Box<dyn Error>> {
    Ok(Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY,
    )?)
}

fn proc_cgroup() -> Result<String, Box<dyn Error>> {
    std::fs::read_to_string("/proc/self/cgroup")?
        .lines()
        .find_map(|l| l.strip_prefix("0::"))
        .map(|p| p.trim_end().to_owned())
        .ok_or_else(|| "no 0:: line in /proc/self/cgroup".into())
}

#[test]
fn open_converges_ledger_and_sidecars_to_0600_and_dir_to_0700_twice() -> R {
    let path = db("modes")?;
    let dir = path.parent().ok_or("dir")?.to_path_buf();
    // The first store stays open as a keeper: its live connection keeps the -wal/-shm sidecars
    // on disk while each round's store closes.
    let keeper = Store::open(&path)?;
    keeper.apply(&"t1".parse()?, Event::Admit)?;
    let wal = sidecar(&path, "-wal");
    let shm = sidecar(&path, "-shm");
    for round in 1..=2 {
        assert!(
            wal.exists() && shm.exists(),
            "round {round}: sidecars exist"
        );
        chmod(&path, 0o644)?;
        chmod(&wal, 0o644)?;
        chmod(&shm, 0o644)?;
        chmod(&dir, 0o755)?;
        assert_eq!(
            mode(&path)?,
            0o644,
            "round {round}: the fixture drifted the mode"
        );
        let store = Store::open(&path)?;
        store.apply(&format!("t-round-{round}").parse()?, Event::Admit)?;
        assert_eq!(mode(&path)?, 0o600, "round {round}: ledger");
        assert_eq!(mode(&wal)?, 0o600, "round {round}: -wal");
        assert_eq!(mode(&shm)?, 0o600, "round {round}: -shm");
        assert_eq!(mode(&dir)?, 0o700, "round {round}: dir");
        drop(store);
    }
    drop(keeper);
    Ok(())
}

#[test]
fn serve_cgroup_is_in_meta_and_stamped_once_on_admitted_tasks() -> R {
    let path = db("cgroup")?;
    let expected = proc_cgroup()?;
    assert!(
        expected.starts_with('/') && (expected.contains(".scope") || expected.contains(".service")),
        "a cgroup v2 path of a scope or a service: {expected}"
    );
    let store = Store::open(&path)?;
    assert_eq!(store.serve_cgroup(), expected);
    let in_meta: String = ro(&path)?.query_row(
        "SELECT value FROM meta WHERE key = 'serve_cgroup'",
        [],
        |r| r.get(0),
    )?;
    assert_eq!(in_meta, expected);
    let t: TaskId = "t-cg".parse()?;
    let op = OperationKey {
        principal: "luke".into(),
        action: "task.submit".into(),
        version: 1,
        idem_key: "k-cg".into(),
    };
    store.admit(&t, &op, b"spec", |_, _| json!({}))?;
    let on_task = |id: &str| -> Result<String, Box<dyn Error>> {
        Ok(
            ro(&path)?.query_row("SELECT serve_cgroup FROM tasks WHERE id = ?1", [id], |r| {
                r.get(0)
            })?,
        )
    };
    assert_eq!(on_task("t-cg")?, expected);
    assert!(reconcile(&store, &Observations::default())?.complete);
    // A later event's ON CONFLICT update leaves the admission-time value alone.
    Connection::open(&path)?.execute(
        "UPDATE tasks SET serve_cgroup = 'planted-at-admission' WHERE id = 't-cg'",
        [],
    )?;
    store.apply(&t, Event::Dispatch)?;
    assert_eq!(on_task("t-cg")?, "planted-at-admission");
    Ok(())
}

#[test]
fn boot_increments_by_one_per_open_while_epoch_is_unchanged() -> R {
    let path = db("boot")?;
    let first = Store::open(&path)?;
    let (boot, epoch) = (first.boot()?, first.epoch()?);
    assert!(boot >= 1);
    assert_ne!(epoch, "");
    drop(first);
    let second = Store::open(&path)?;
    assert_eq!(second.boot()?, boot + 1);
    assert_eq!(second.epoch()?, epoch);
    drop(second);
    let third = Store::open(&path)?;
    assert_eq!(third.boot()?, boot + 2);
    assert_eq!(third.epoch()?, epoch);
    Ok(())
}

#[test]
fn event_high_water_and_cursor_check_answer_all_four_verdicts() -> R {
    let path = db("cursor")?;
    let store = Store::open(&path)?;
    let epoch = store.epoch()?;
    assert_eq!(store.event_high_water()?, 0, "empty ledger");
    assert_eq!(store.cursor_check(&epoch, 0)?, CursorVerdict::SnapshotOnly);
    assert_eq!(
        store.cursor_check(&epoch, 1)?,
        CursorVerdict::FutureSequence
    );
    for name in ["t-a", "t-b", "t-c"] {
        store.apply(&name.parse()?, Event::Admit)?;
    }
    let hw = store.event_high_water()?;
    let last_seq: u64 = ro(&path)?.query_row("SELECT max(seq) FROM events", [], |r| r.get(0))?;
    assert_eq!(hw, last_seq, "high water is the last seq");
    assert_eq!(hw, store.event_count()?, "no gaps: equals the count");
    assert_eq!(
        store.cursor_check(&epoch, hw + 1)?,
        CursorVerdict::FutureSequence
    );
    assert_eq!(store.cursor_check("other", 0)?, CursorVerdict::EpochChanged);
    assert_eq!(store.cursor_check(&epoch, hw)?, CursorVerdict::SnapshotOnly);
    assert_eq!(store.cursor_check(&epoch, 0)?, CursorVerdict::SnapshotOnly);
    store.mark_restored_from("old")?;
    assert_eq!(
        store.cursor_check("old", 0)?,
        CursorVerdict::PriorEpochOfRestore
    );
    assert_eq!(
        store.cursor_check("old", hw + 10)?,
        CursorVerdict::PriorEpochOfRestore,
        "restored_from is checked before the high water"
    );
    assert_eq!(store.cursor_check(&epoch, hw)?, CursorVerdict::SnapshotOnly);
    let restored_from: String = ro(&path)?.query_row(
        "SELECT value FROM meta WHERE key = 'restored_from'",
        [],
        |r| r.get(0),
    )?;
    assert_eq!(restored_from, "old");
    Ok(())
}
