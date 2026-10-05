//! Backup and restore (K1's half of DC-22 / V4-6 / V4-28). No SQL here: the ledger snapshot is
//! [`Store::snapshot_into`]; everything else is files.
//!
//! A backup is a directory `<dest_root>/<id>/` holding `ledger.sqlite3`, `objects/<name>.brief`
//! for every brief under `<work>/briefs/`, and `manifest.json` written LAST and renamed into
//! place, so a directory without a manifest is incomplete by construction and
//! [`restore`] refuses it by name. There is no backup thread, timer or daemon (V4-6), and no
//! trigger policy here: the four DC-22 triggers are K6's to fire.

use std::collections::BTreeMap;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::time::Instant;

use hee4_contracts::Sha256Hex;
use serde_json::{Value, json};

use crate::recovery::{Observations, reconcile};
use crate::store::{Store, StoreError};

/// The declared bound on objects in one backup. Over it, [`backup_to`] refuses rather than
/// writing a backup it would not verify in bounded time.
pub const MAX_BACKUP_OBJECTS: usize = 1024;

const LEDGER_FILE: &str = "ledger.sqlite3";
const MANIFEST_FILE: &str = "manifest.json";
const OBJECTS_DIR: &str = "objects";
const BRIEFS_DIR: &str = "briefs";
const BRIEF_EXT: &str = "brief";

/// Whether a backup may land on the device that holds the ledger.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SameDisk {
    /// Refuse a destination on the ledger's device (the default for `serve`).
    #[default]
    Refuse,
    /// Allow it (tests, a drill into a scratch dir).
    Allow,
}

/// Why a backup or restore wrote nothing (or stopped before the manifest).
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum BackupError {
    /// A store door refused.
    #[error("store: {0}")]
    Store(#[from] StoreError),
    /// The file system refused.
    #[error("io at {path}: {source}")]
    Io {
        /// The path the call touched.
        path: String,
        /// The OS's answer.
        #[source]
        source: std::io::Error,
    },
    /// The manifest cannot be written or read as JSON.
    #[error("manifest json: {0}")]
    Json(#[from] serde_json::Error),
    /// The manifest is JSON but not a manifest.
    #[error("manifest field {field} is missing or malformed")]
    Manifest {
        /// The field that failed.
        field: &'static str,
    },
    /// More briefs than [`MAX_BACKUP_OBJECTS`].
    #[error("{found} objects exceed the backup bound {bound}")]
    ObjectsOverBound {
        /// Briefs found.
        found: usize,
        /// The bound.
        bound: usize,
    },
    /// The destination shares a device with the ledger (`SameDisk::Refuse`).
    #[error("destination is on the ledger's device (ledger dev {ledger_dev}, dest dev {dest_dev})")]
    SameDevice {
        /// `st_dev` of the ledger.
        ledger_dev: u64,
        /// `st_dev` of the destination root.
        dest_dev: u64,
    },
    /// The backup directory has no manifest (the writer never reached its last step).
    #[error("backup {dir} is incomplete: no manifest.json")]
    Incomplete {
        /// The backup directory.
        dir: PathBuf,
    },
    /// The restore target already holds a ledger.
    #[error("restore target {path} already holds a ledger")]
    TargetOccupied {
        /// The occupied path.
        path: PathBuf,
    },
    /// A file's sha256 differs from the manifest's.
    #[error("digest of {file} differs from the manifest")]
    DigestMismatch {
        /// The manifest-relative path.
        file: String,
    },
    /// The objects dir is short of the manifest.
    #[error("objects dir holds {n} of the manifest's {total} objects")]
    ObjectsMissing {
        /// Objects present.
        n: usize,
        /// Objects the manifest lists.
        total: usize,
    },
}

/// What [`backup_to`] wrote.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackupReport {
    /// `b-<ts_ms as 12 hex digits>-<boot as 8 hex digits>`: lexically sortable by time.
    pub id: String,
    /// `<dest_root>/<id>`.
    pub dir: PathBuf,
    /// When the backup started (ms since the Unix epoch).
    pub ts_ms: i64,
    /// The ledger epoch at backup time.
    pub epoch: String,
    /// The ledger's `boot` at backup time.
    pub boot: u64,
    /// Tasks in the ledger at manifest time.
    pub task_count: usize,
    /// Objects copied.
    pub objects_n: usize,
    /// sha256 of the snapshot.
    pub ledger_sha256: String,
}

/// What [`restore`] did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RestoreReport {
    /// The manifest's `id`.
    pub backup_id: String,
    /// sha256 of the ledger as restored (equal to the manifest's).
    pub ledger_sha256: String,
    /// Objects restored.
    pub objects_n: usize,
    /// Objects the manifest lists.
    pub objects_total: usize,
    /// Whether the recovery pass inside the verb completed.
    pub recovery_complete: bool,
    /// Wall time of the verb.
    pub rto_ms: u64,
}

fn io_at(path: &Path, source: std::io::Error) -> BackupError {
    BackupError::Io {
        path: path.display().to_string(),
        source,
    }
}

fn sha256_of(path: &Path) -> Result<String, BackupError> {
    let bytes = std::fs::read(path).map_err(|e| io_at(path, e))?;
    Ok(Sha256Hex::digest(&bytes).to_string())
}

fn copy(from: &Path, to: &Path) -> Result<(), BackupError> {
    std::fs::copy(from, to).map_err(|e| io_at(to, e))?;
    Ok(())
}

/// Every `*.brief` under `<work>/briefs`, by file name; none when the dir is absent.
fn briefs(work: &Path) -> Result<Vec<PathBuf>, BackupError> {
    let dir = work.join(BRIEFS_DIR);
    let entries = match std::fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(io_at(&dir, e)),
    };
    let mut out = Vec::new();
    for entry in entries {
        let path = entry.map_err(|e| io_at(&dir, e))?.path();
        if path.is_file() && path.extension().is_some_and(|e| e == BRIEF_EXT) {
            out.push(path);
        }
    }
    out.sort();
    Ok(out)
}

fn file_name(path: &Path) -> Result<String, BackupError> {
    path.file_name()
        .and_then(|n| n.to_str())
        .map(str::to_owned)
        .ok_or_else(|| {
            io_at(
                path,
                std::io::Error::new(std::io::ErrorKind::InvalidInput, "no UTF-8 file name"),
            )
        })
}

/// Snapshot `store` and copy every brief under `<work_briefs>/briefs/` into
/// `<dest_root>/<id>/`, then write `manifest.json` last (renamed into place). Returns the
/// report; the backup dir is `report.dir`.
///
/// # Errors
/// [`BackupError::SameDevice`] under [`SameDisk::Refuse`] when `dest_root` is on the
/// ledger's device; [`BackupError::ObjectsOverBound`]; IO and store errors.
pub fn backup_to(
    store: &Store,
    work_briefs: &Path,
    dest_root: &Path,
    same_disk: SameDisk,
) -> Result<BackupReport, BackupError> {
    std::fs::create_dir_all(dest_root).map_err(|e| io_at(dest_root, e))?;
    let ledger_dev = std::fs::metadata(store.path())
        .map_err(|e| io_at(store.path(), e))?
        .dev();
    let dest_dev = std::fs::metadata(dest_root)
        .map_err(|e| io_at(dest_root, e))?
        .dev();
    if same_disk == SameDisk::Refuse && ledger_dev == dest_dev {
        return Err(BackupError::SameDevice {
            ledger_dev,
            dest_dev,
        });
    }
    let objects = briefs(work_briefs)?;
    if objects.len() > MAX_BACKUP_OBJECTS {
        return Err(BackupError::ObjectsOverBound {
            found: objects.len(),
            bound: MAX_BACKUP_OBJECTS,
        });
    }
    let ts_ms = crate::store::now_ms();
    let epoch = store.epoch()?;
    let boot = store.boot()?;
    let id = format!("b-{ts_ms:012x}-{boot:08x}");
    let dir = dest_root.join(&id);
    std::fs::create_dir(&dir).map_err(|e| io_at(&dir, e))?;
    let objects_dir = dir.join(OBJECTS_DIR);
    std::fs::create_dir(&objects_dir).map_err(|e| io_at(&objects_dir, e))?;

    let ledger = dir.join(LEDGER_FILE);
    store.snapshot_into(&ledger)?;
    let mut files: BTreeMap<String, String> = BTreeMap::new();
    let ledger_sha256 = sha256_of(&ledger)?;
    files.insert(LEDGER_FILE.to_owned(), ledger_sha256.clone());
    for src in &objects {
        let name = file_name(src)?;
        let dest = objects_dir.join(&name);
        copy(src, &dest)?;
        files.insert(format!("{OBJECTS_DIR}/{name}"), sha256_of(&dest)?);
    }
    let task_count = store.task_ids()?.len();
    let manifest = json!({
        "id": id,
        "ts_ms": ts_ms,
        "epoch": epoch,
        "boot": boot,
        "task_count": task_count,
        "objects_n": objects.len(),
        "objects_bound": MAX_BACKUP_OBJECTS,
        "files": files,
    });
    write_manifest_last(&dir, &manifest)?;
    Ok(BackupReport {
        id,
        dir,
        ts_ms,
        epoch,
        boot,
        task_count,
        objects_n: objects.len(),
        ledger_sha256,
    })
}

/// Write `manifest.json.tmp`, fsync it, rename it to `manifest.json`: the manifest exists only
/// once every other file does.
fn write_manifest_last(dir: &Path, manifest: &Value) -> Result<(), BackupError> {
    let tmp = dir.join(format!("{MANIFEST_FILE}.tmp"));
    let bytes = serde_json::to_vec_pretty(manifest)?;
    {
        let mut file = std::fs::File::create(&tmp).map_err(|e| io_at(&tmp, e))?;
        std::io::Write::write_all(&mut file, &bytes).map_err(|e| io_at(&tmp, e))?;
        file.sync_all().map_err(|e| io_at(&tmp, e))?;
    }
    let manifest_path = dir.join(MANIFEST_FILE);
    std::fs::rename(&tmp, &manifest_path).map_err(|e| io_at(&manifest_path, e))?;
    Ok(())
}

/// A manifest `files` key this writer produces: `ledger.sqlite3`, or `objects/<name>.brief`
/// where `<name>.brief` is one normal path component (no `/`, not `.` or `..`).
fn is_manifest_key(key: &str) -> bool {
    if key == LEDGER_FILE {
        return true;
    }
    let Some(name) = key
        .strip_prefix(OBJECTS_DIR)
        .and_then(|rest| rest.strip_prefix('/'))
    else {
        return false;
    };
    let mut parts = Path::new(name).components();
    matches!(
        (parts.next(), parts.next()),
        (Some(std::path::Component::Normal(one)), None) if one == name
    ) && Path::new(name).extension().is_some_and(|e| e == BRIEF_EXT)
}

fn field<'a>(manifest: &'a Value, name: &'static str) -> Result<&'a Value, BackupError> {
    manifest
        .get(name)
        .ok_or(BackupError::Manifest { field: name })
}

/// Restore `backup_dir` into `into`: `<into>/ledger.sqlite3` and `<into>/work/briefs/*.brief`
/// (the live layout). Verifies every sha256 against the manifest before copying anything, then
/// stages the copy under `<into>/.restore-<id>.tmp/`: opens the staged ledger, records
/// `restored_from` (the manifest's epoch), gives the ledger a fresh epoch and runs `reconcile`
/// with unobserved custody there. Only then are the briefs and, last, the ledger renamed into
/// place. Any failure after the copy removes the staging dir, so `<into>` holds no ledger and a
/// retry is not `TargetOccupied`; the failure is the restore's only output.
///
/// # Errors
/// [`BackupError::Manifest`] (`files`) for a key other than `ledger.sqlite3` or
/// `objects/<name>.brief`, before any file is read;
/// [`BackupError::Incomplete`] (no manifest, or no ledger), [`BackupError::TargetOccupied`],
/// [`BackupError::ObjectsMissing`], [`BackupError::DigestMismatch`]; [`BackupError::Store`]
/// when the staged ledger cannot be opened or reconciled (a snapshot newer than this binary
/// answers `UnknownMigration`); IO errors.
pub fn restore(backup_dir: &Path, into: &Path) -> Result<RestoreReport, BackupError> {
    let started = Instant::now();
    let manifest_path = backup_dir.join(MANIFEST_FILE);
    if !manifest_path.is_file() {
        return Err(BackupError::Incomplete {
            dir: backup_dir.to_path_buf(),
        });
    }
    let manifest: Value = serde_json::from_slice(
        &std::fs::read(&manifest_path).map_err(|e| io_at(&manifest_path, e))?,
    )?;
    let backup_id = field(&manifest, "id")?
        .as_str()
        .ok_or(BackupError::Manifest { field: "id" })?
        .to_owned();
    let old_epoch = field(&manifest, "epoch")?
        .as_str()
        .ok_or(BackupError::Manifest { field: "epoch" })?
        .to_owned();
    let files = field(&manifest, "files")?
        .as_object()
        .ok_or(BackupError::Manifest { field: "files" })?;
    // Every key is a path inside the backup dir that this writer could have written; anything
    // else ('..', an absolute path, a nested dir) is refused before any file is read.
    if !files.keys().all(|key| is_manifest_key(key)) {
        return Err(BackupError::Manifest { field: "files" });
    }
    let ledger_sha256 = files
        .get(LEDGER_FILE)
        .and_then(Value::as_str)
        .ok_or(BackupError::Manifest { field: "files" })?
        .to_owned();

    let target_ledger = into.join(LEDGER_FILE);
    if target_ledger.exists() {
        return Err(BackupError::TargetOccupied {
            path: target_ledger,
        });
    }
    if !backup_dir.join(LEDGER_FILE).is_file() {
        return Err(BackupError::Incomplete {
            dir: backup_dir.to_path_buf(),
        });
    }
    let objects: Vec<&String> = files.keys().filter(|k| *k != LEDGER_FILE).collect();
    let objects_total = objects.len();
    let present = objects
        .iter()
        .filter(|rel| backup_dir.join(rel).is_file())
        .count();
    if present != objects_total {
        return Err(BackupError::ObjectsMissing {
            n: present,
            total: objects_total,
        });
    }
    for (rel, expected) in files {
        let expected = expected
            .as_str()
            .ok_or(BackupError::Manifest { field: "files" })?;
        if sha256_of(&backup_dir.join(rel))? != expected {
            return Err(BackupError::DigestMismatch { file: rel.clone() });
        }
    }

    let staging = into.join(format!(".restore-{backup_id}.tmp"));
    if staging.exists() {
        std::fs::remove_dir_all(&staging).map_err(|e| io_at(&staging, e))?;
    }
    let staged = stage(backup_dir, &staging, &objects, &old_epoch)
        .and_then(|done| move_into_place(&staging, into, &objects).map(|()| done));
    let (recovery_complete, objects_n) = match staged {
        Ok(done) => done,
        Err(e) => {
            if staging.exists() {
                std::fs::remove_dir_all(&staging).map_err(|e| io_at(&staging, e))?;
            }
            return Err(e);
        }
    };
    Ok(RestoreReport {
        backup_id,
        ledger_sha256,
        objects_n,
        objects_total,
        recovery_complete,
        rto_ms: u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
    })
}

/// Copy the verified ledger and objects into `staging`, open the staged ledger, mark
/// `restored_from`, renew the epoch and reconcile there. Returns `reconcile`'s `complete` and
/// the number of objects actually staged (the report's `objects_n`). The
/// `Store` is dropped before returning, so the staged ledger has no open connection (and, under
/// WAL, no live `-wal`/`-shm`) when it is renamed.
fn stage(
    backup_dir: &Path,
    staging: &Path,
    objects: &[&String],
    old_epoch: &str,
) -> Result<(bool, usize), BackupError> {
    let briefs_dir = staging.join("work").join(BRIEFS_DIR);
    std::fs::create_dir_all(&briefs_dir).map_err(|e| io_at(&briefs_dir, e))?;
    copy(&backup_dir.join(LEDGER_FILE), &staging.join(LEDGER_FILE))?;
    let mut staged = 0_usize;
    for rel in objects {
        let src = backup_dir.join(rel);
        copy(&src, &briefs_dir.join(file_name(&src)?))?;
        staged += 1;
    }
    let store = Store::open(&staging.join(LEDGER_FILE))?;
    store.mark_restored_from(old_epoch)?;
    store.renew_epoch()?;
    let report = reconcile(&store, &Observations::default())?;
    drop(store);
    Ok((report.complete, staged))
}

/// Rename the staged briefs into `<into>/work/briefs/`, then the ledger's `-wal`/`-shm` when
/// present, then `ledger.sqlite3` LAST: the target holds a ledger only once everything else is
/// in place. Converges `<into>` to 0700 and removes the emptied staging dir.
fn move_into_place(staging: &Path, into: &Path, objects: &[&String]) -> Result<(), BackupError> {
    let briefs_dir = into.join("work").join(BRIEFS_DIR);
    std::fs::create_dir_all(&briefs_dir).map_err(|e| io_at(&briefs_dir, e))?;
    let staged_briefs = staging.join("work").join(BRIEFS_DIR);
    for rel in objects {
        let name = file_name(Path::new(rel))?;
        let dest = briefs_dir.join(&name);
        std::fs::rename(staged_briefs.join(&name), &dest).map_err(|e| io_at(&dest, e))?;
    }
    for suffix in ["-wal", "-shm", ""] {
        let name = format!("{LEDGER_FILE}{suffix}");
        let from = staging.join(&name);
        if from.exists() {
            let dest = into.join(&name);
            std::fs::rename(&from, &dest).map_err(|e| io_at(&dest, e))?;
        }
    }
    std::fs::set_permissions(into, std::fs::Permissions::from_mode(0o700))
        .map_err(|e| io_at(into, e))?;
    std::fs::remove_dir_all(staging).map_err(|e| io_at(staging, e))?;
    Ok(())
}
