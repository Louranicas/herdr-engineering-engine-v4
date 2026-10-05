//! The startup probe: the only IO recovery consults. It reads `/proc`, the workspace
//! directories and the receiver clock into [`Observations`]; the policy
//! (`recovery::decide`) then reads those as data and nothing else (EX-04). No SQL lives here
//! (`tests/one_door.rs` walks this file); the rows come in from the store's readers.
//!
//! Every bound is the caller's: `workspace_readback_bytes` caps the directory walk (wave 3
//! passes the K0 budget field); no literal budget lives in core.
//!
//! [`observe`] never panics and never fails: what cannot be read is `Unreadable`, `Unobserved`
//! or `None`.

use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::recovery::{Clock, Observations, ProcessCustody, WorkspaceReadback};
use crate::store::AttemptRow;

const BOOT_ID: &str = "/proc/sys/kernel/random/boot_id";

/// Field 22 of a `/proc/<pid>/stat` line (`starttime`), found after the last `)` because
/// `comm` may hold spaces and parentheses.
fn start_ticks(stat: &str) -> Option<u64> {
    let (_, after_comm) = stat.rsplit_once(')')?;
    // `after_comm` starts at field 3 (`state`); `starttime` is field 22.
    after_comm.split_whitespace().nth(19)?.parse().ok()
}

/// What `/proc/<pid>/stat` says against a recorded `(pid, start_ticks)`.
fn process_custody(pid: u32, recorded_ticks: u64) -> ProcessCustody {
    match std::fs::read_to_string(format!("/proc/{pid}/stat")) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => ProcessCustody::Absent,
        Err(_) => ProcessCustody::Unreadable,
        Ok(stat) => match start_ticks(&stat) {
            None => ProcessCustody::Unreadable,
            Some(ticks) if ticks == recorded_ticks => ProcessCustody::LiveSameIdentity,
            Some(_) => ProcessCustody::PidReused,
        },
    }
}

/// Sum of file sizes under `dir` (symlinks not followed), stopping once `bound` is reached.
/// An entry that cannot be read counts as nothing.
fn walk_dir(dir: &Path, bound: u64) -> u64 {
    let mut total: u64 = 0;
    let mut pending = vec![dir.to_path_buf()];
    while let Some(d) = pending.pop() {
        let Ok(entries) = std::fs::read_dir(&d) else {
            continue;
        };
        for entry in entries.flatten() {
            let Ok(meta) = entry.metadata() else {
                continue;
            };
            if meta.is_dir() {
                pending.push(entry.path());
            } else if meta.is_file() {
                total = total.saturating_add(meta.len());
            }
            if total >= bound {
                return bound;
            }
        }
    }
    total
}

/// What the file system says about a workspace path.
fn workspace_readback(path: &Path, bound: u64) -> WorkspaceReadback {
    match std::fs::symlink_metadata(path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => WorkspaceReadback::Absent,
        Ok(meta) if meta.is_dir() => WorkspaceReadback::Writable {
            bytes: walk_dir(path, bound),
        },
        Ok(_) | Err(_) => WorkspaceReadback::Unreadable,
    }
}

/// The receiver clock: `SystemTime` now and this boot's `boot_id`; `None` when the boot id
/// cannot be read (a lease is then `ClockUnavailable`).
fn read_clock() -> Option<Clock> {
    let clock_epoch = std::fs::read_to_string(BOOT_ID).ok()?.trim().to_owned();
    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|d| i64::try_from(d.as_millis()).ok())?;
    Some(Clock {
        now_ms,
        clock_epoch,
    })
}

/// Observe every row: custody from `/proc/<pid>/stat` (no pid recorded → `Unobserved`;
/// `ENOENT` → `Absent`; another error → `Unreadable`; equal start ticks →
/// `LiveSameIdentity`; other ticks → `PidReused`), the workspace (no workspace →
/// `Unobserved`; `ENOENT` → `Absent`; a directory → `Writable{bytes}` walked up to
/// `workspace_readback_bytes`; anything else → `Unreadable`), and the clock. Tasks without a
/// row keep the default custody (`Unobserved`).
#[must_use]
pub fn observe(rows: &[AttemptRow], workspace_readback_bytes: u64) -> Observations {
    let mut observed = Observations::default();
    for row in rows {
        let custody = row.pid.map_or(ProcessCustody::Unobserved, |(pid, ticks)| {
            process_custody(pid, ticks)
        });
        observed.process.insert(row.task_id.clone(), custody);
        let workspace = row
            .started
            .as_ref()
            .map_or(WorkspaceReadback::Unobserved, |s| {
                workspace_readback(&s.workspace, workspace_readback_bytes)
            });
        observed.workspace.insert(row.id.clone(), workspace);
    }
    observed.clock = read_clock();
    observed
}

/// This process's `(pid, start_ticks)` from `/proc/self/stat`, the identity a worker records
/// with `Store::attempt_pid`. `None` when the file cannot be read or parsed.
#[must_use]
pub fn self_identity() -> Option<(u32, u64)> {
    let stat = std::fs::read_to_string("/proc/self/stat").ok()?;
    let (pid, _) = stat.split_once(' ')?;
    Some((pid.parse().ok()?, start_ticks(&stat)?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn start_ticks_is_field_22_after_the_comm() {
        let stat = "123 (a (weird) name) S 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 999 20";
        assert_eq!(start_ticks(stat), Some(999));
        assert_eq!(start_ticks("garbage"), None);
    }

    #[test]
    fn self_identity_matches_proc_self() {
        let (pid, ticks) = self_identity().unwrap_or((0, 0));
        assert_eq!(pid, std::process::id());
        assert!(ticks > 0);
        assert_eq!(
            process_custody(pid, ticks),
            ProcessCustody::LiveSameIdentity
        );
        assert_eq!(process_custody(pid, ticks + 1), ProcessCustody::PidReused);
    }
}
