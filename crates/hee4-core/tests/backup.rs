//! The backup and restore doors: a consistent snapshot under a concurrent writer, the
//! manifest-last rule, the same-device refusal, a restore that reconciles and renews the
//! epoch, and the three tampering refusals.

use std::error::Error;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use hee4_contracts::{Event, Sha256Hex};
use hee4_core::backup::{backup_to, restore};
use hee4_core::{BackupError, CursorVerdict, Observations, SameDisk, Store, StoreError, reconcile};
use serde_json::Value;

type R = Result<(), Box<dyn Error>>;

fn scratch(name: &str) -> Result<PathBuf, Box<dyn Error>> {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("hee4-core-backup")
        .join(name);
    if dir.exists() {
        std::fs::remove_dir_all(&dir)?;
    }
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// A data dir with a ledger and `work/briefs/<n>.brief` for `briefs` tasks.
fn seed(base: &Path, briefs: usize) -> Result<(PathBuf, PathBuf), Box<dyn Error>> {
    let ledger = base.join("data").join("ledger.sqlite3");
    let work = base.join("data").join("work");
    std::fs::create_dir_all(work.join("briefs"))?;
    for i in 0..briefs {
        std::fs::write(
            work.join("briefs").join(format!("t-{i:03}.brief")),
            format!("brief {i}\n"),
        )?;
    }
    Ok((ledger, work))
}

fn manifest(dir: &Path) -> Result<Value, Box<dyn Error>> {
    Ok(serde_json::from_slice(&std::fs::read(
        dir.join("manifest.json"),
    )?)?)
}

fn walk(dir: &Path, root: &Path, out: &mut Vec<String>) -> R {
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            walk(&path, root, out)?;
        } else {
            out.push(path.strip_prefix(root)?.display().to_string());
        }
    }
    Ok(())
}

fn copy_tree(from: &Path, to: &Path) -> R {
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let dest = to.join(entry.file_name());
        if entry.path().is_dir() {
            copy_tree(&entry.path(), &dest)?;
        } else {
            std::fs::copy(entry.path(), &dest)?;
        }
    }
    Ok(())
}

#[test]
fn backup_under_a_concurrent_writer_is_a_consistent_snapshot_with_the_manifest_last() -> R {
    let base = scratch("concurrent")?;
    let (ledger, work) = seed(&base, 3)?;
    let store = Store::open(&ledger)?;
    assert!(reconcile(&store, &Observations::default())?.complete);
    let stop = Arc::new(AtomicBool::new(false));
    let writer = {
        let stop = Arc::clone(&stop);
        let ledger = ledger.clone();
        std::thread::spawn(move || -> Result<u64, String> {
            let mine = Store::open(&ledger).map_err(|e| e.to_string())?;
            let mut applied = 0u64;
            while !stop.load(Ordering::SeqCst) {
                mine.apply(
                    &format!("w-{applied:06}")
                        .parse()
                        .map_err(|e| format!("{e}"))?,
                    Event::Admit,
                )
                .map_err(|e| e.to_string())?;
                applied += 1;
            }
            Ok(applied)
        })
    };
    while store.event_count()? < 20 {
        std::thread::sleep(Duration::from_millis(5));
    }
    let before = store.event_count()?;
    let report = backup_to(&store, &work, &base.join("backups"), SameDisk::Allow)?;
    stop.store(true, Ordering::SeqCst);
    let applied = writer.join().map_err(|_| "writer panicked")??;

    let check = base.join("check.sqlite3");
    std::fs::copy(report.dir.join("ledger.sqlite3"), &check)?;
    let snapshot = Store::open(&check)?;
    assert_eq!(snapshot.integrity_check()?, "ok");
    let in_snapshot = snapshot.event_count()?;
    println!(
        "backup consistency: writer applied={applied} events_before_backup={before} \
         snapshot_event_count={in_snapshot} id={}",
        report.id
    );
    assert!(
        in_snapshot >= before && in_snapshot <= applied,
        "snapshot holds a prefix of the writer's acks"
    );
    assert_eq!(snapshot.schema_version()?, 5);
    assert_eq!(report.objects_n, 3);
    assert_eq!(report.epoch, store.epoch()?);

    let m = manifest(&report.dir)?;
    assert_eq!(m["id"], report.id);
    assert_eq!(m["epoch"], report.epoch);
    assert_eq!(m["files"]["ledger.sqlite3"], report.ledger_sha256);
    let mut on_disk = Vec::new();
    walk(&report.dir, &report.dir, &mut on_disk)?;
    on_disk.retain(|f| f != "manifest.json");
    on_disk.sort();
    let mut listed: Vec<String> = m["files"]
        .as_object()
        .ok_or("files")?
        .keys()
        .cloned()
        .collect();
    listed.sort();
    assert_eq!(
        on_disk, listed,
        "the manifest lists every file and nothing else"
    );
    assert_eq!(listed.len(), report.objects_n + 1);

    let no_manifest = base.join("no-manifest");
    copy_tree(&report.dir, &no_manifest)?;
    std::fs::remove_file(no_manifest.join("manifest.json"))?;
    let err = restore(&no_manifest, &base.join("into-incomplete"));
    assert!(
        matches!(&err, Err(BackupError::Incomplete { dir }) if dir == &no_manifest),
        "{err:?}"
    );
    Ok(())
}

#[test]
fn a_destination_on_the_ledgers_device_is_refused_unless_allowed() -> R {
    let base = scratch("device")?;
    let (ledger, work) = seed(&base, 1)?;
    let store = Store::open(&ledger)?;
    let dest = base.join("backups");
    let err = backup_to(&store, &work, &dest, SameDisk::Refuse);
    assert!(
        matches!(
            &err,
            Err(BackupError::SameDevice { ledger_dev, dest_dev }) if ledger_dev == dest_dev
        ),
        "{err:?}"
    );
    assert!(
        std::fs::read_dir(&dest)?.next().is_none(),
        "a refusal writes no backup dir"
    );
    let report = backup_to(&store, &work, &dest, SameDisk::Allow)?;
    assert!(report.dir.join("manifest.json").is_file());
    Ok(())
}

#[test]
fn restore_into_an_empty_dir_reconciles_and_renews_the_epoch() -> R {
    let base = scratch("restore")?;
    let (ledger, work) = seed(&base, 2)?;
    let store = Store::open(&ledger)?;
    assert!(reconcile(&store, &Observations::default())?.complete);
    store.apply(&"t-000".parse()?, Event::Admit)?;
    store.apply(&"t-001".parse()?, Event::Admit)?;
    store.apply(&"t-001".parse()?, Event::Dispatch)?;
    let old_epoch = store.epoch()?;
    let backup = backup_to(&store, &work, &base.join("backups"), SameDisk::Allow)?;
    let m = manifest(&backup.dir)?;

    let into = base.join("into");
    let report = restore(&backup.dir, &into)?;
    println!("restore: {report:?}");
    assert_eq!(report.backup_id, backup.id);
    assert!(report.recovery_complete);
    assert_eq!(report.objects_n, report.objects_total);
    assert_eq!(report.objects_total, backup.objects_n);
    assert_eq!(
        Value::from(report.ledger_sha256.clone()),
        m["files"]["ledger.sqlite3"]
    );
    let mut restored_briefs: Vec<String> = Vec::new();
    walk(
        &into.join("work").join("briefs"),
        &into.join("work").join("briefs"),
        &mut restored_briefs,
    )?;
    assert_eq!(restored_briefs.len(), backup.objects_n);

    let restored = Store::open(&into.join("ledger.sqlite3"))?;
    assert_ne!(restored.epoch()?, old_epoch);
    assert_eq!(
        restored.cursor_check(&old_epoch, 0)?,
        CursorVerdict::PriorEpochOfRestore
    );
    assert_eq!(
        restored.history(&"t-001".parse()?)?.first(),
        Some(&Event::Admit)
    );
    assert_eq!(restored.task_ids()?.len(), store.task_ids()?.len());
    Ok(())
}

#[test]
fn restore_refuses_a_planted_byte_a_missing_object_and_an_occupied_target() -> R {
    let base = scratch("tamper")?;
    let (ledger, work) = seed(&base, 3)?;
    let store = Store::open(&ledger)?;
    store.apply(&"t-000".parse()?, Event::Admit)?;
    let backup = backup_to(&store, &work, &base.join("backups"), SameDisk::Allow)?;

    let planted = base.join("planted");
    copy_tree(&backup.dir, &planted)?;
    let file = planted.join("ledger.sqlite3");
    let mut bytes = std::fs::read(&file)?;
    let at = bytes.len() / 2;
    bytes[at] ^= 0xff;
    std::fs::write(&file, bytes)?;
    let err = restore(&planted, &base.join("into-planted"));
    assert!(
        matches!(&err, Err(BackupError::DigestMismatch { file }) if file == "ledger.sqlite3"),
        "{err:?}"
    );
    assert!(
        !base.join("into-planted").join("ledger.sqlite3").exists(),
        "a refused restore copies nothing"
    );

    let short = base.join("short");
    copy_tree(&backup.dir, &short)?;
    std::fs::remove_file(short.join("objects").join("t-001.brief"))?;
    let err = restore(&short, &base.join("into-short"));
    assert!(
        matches!(
            &err,
            Err(BackupError::ObjectsMissing { n, total }) if *total == backup.objects_n && *n == total - 1
        ),
        "{err:?}"
    );

    let into = base.join("into");
    restore(&backup.dir, &into)?;
    let err = restore(&backup.dir, &into);
    assert!(
        matches!(&err, Err(BackupError::TargetOccupied { path }) if path == &into.join("ledger.sqlite3")),
        "{err:?}"
    );
    Ok(())
}

/// A backup whose ledger this binary cannot open (a `migration:m999_future` row: a snapshot
/// newer than the binary) is refused as `BackupError::Store(UnknownMigration)` AFTER the
/// digests passed, and still leaves nothing under `<into>`: no ledger, no briefs, no staging
/// dir. The retry with a good backup into the same dir is not `TargetOccupied`.
#[test]
fn a_restore_whose_open_fails_leaves_nothing_behind_and_the_retry_is_not_occupied() -> R {
    let base = scratch("future")?;
    let (ledger, work) = seed(&base, 2)?;
    let store = Store::open(&ledger)?;
    store.apply(&"t-000".parse()?, Event::Admit)?;
    let backup = backup_to(&store, &work, &base.join("backups"), SameDisk::Allow)?;

    let future = base.join("future");
    copy_tree(&backup.dir, &future)?;
    let future_ledger = future.join("ledger.sqlite3");
    {
        let conn = rusqlite::Connection::open(&future_ledger)?;
        conn.execute(
            "INSERT INTO meta(key, value) VALUES ('migration:m999_future', '1')",
            [],
        )?;
    }
    let mut m = manifest(&future)?;
    m["files"]["ledger.sqlite3"] =
        Value::from(Sha256Hex::digest(&std::fs::read(&future_ledger)?).to_string());
    std::fs::write(future.join("manifest.json"), serde_json::to_vec_pretty(&m)?)?;

    let into = base.join("into");
    let err = restore(&future, &into);
    assert!(
        matches!(
            &err,
            Err(BackupError::Store(StoreError::UnknownMigration(name))) if name == "m999_future"
        ),
        "{err:?}"
    );
    assert!(
        !into.join("ledger.sqlite3").exists(),
        "a restore that fails after the copy leaves no ledger behind"
    );
    let mut left = Vec::new();
    if into.exists() {
        walk(&into, &into, &mut left)?;
    }
    assert!(left.is_empty(), "nothing is left under <into>: {left:?}");
    let staging: Vec<String> = match std::fs::read_dir(&into) {
        Ok(entries) => entries
            .filter_map(Result::ok)
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|n| n.starts_with(".restore-"))
            .collect(),
        Err(_) => Vec::new(),
    };
    assert!(staging.is_empty(), "no staging dir survives: {staging:?}");

    let report = restore(&backup.dir, &into)?;
    assert!(report.recovery_complete, "{report:?}");
    assert!(into.join("ledger.sqlite3").is_file());
    assert!(
        !into.join(format!(".restore-{}.tmp", backup.id)).exists(),
        "the staging dir is gone after a completed restore"
    );
    let restored = Store::open(&into.join("ledger.sqlite3"))?;
    assert_eq!(
        restored.cursor_check(&store.epoch()?, 0)?,
        CursorVerdict::PriorEpochOfRestore
    );
    Ok(())
}
