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

/// Field 3 of a `/proc/<pid>/stat` line (`state`), the first field after the last `)`.
fn proc_state(stat: &str) -> Option<&str> {
    let (_, after_comm) = stat.rsplit_once(')')?;
    after_comm.split_whitespace().next()
}

/// What one `/proc/<pid>/stat` line says against recorded start ticks: a zombie (`Z`) or a
/// dead task (`X`, `x`) is `Absent` (it runs nothing and never will again), whatever its
/// ticks; otherwise equal ticks are `LiveSameIdentity` and other ticks `PidReused`.
fn custody_from_stat(stat: &str, recorded_ticks: u64) -> ProcessCustody {
    if matches!(proc_state(stat), Some("Z" | "X" | "x")) {
        return ProcessCustody::Absent;
    }
    match start_ticks(stat) {
        None => ProcessCustody::Unreadable,
        Some(ticks) if ticks == recorded_ticks => ProcessCustody::LiveSameIdentity,
        Some(_) => ProcessCustody::PidReused,
    }
}

/// What `/proc/<pid>/stat` says against a recorded `(pid, start_ticks)`.
fn process_custody(pid: u32, recorded_ticks: u64) -> ProcessCustody {
    match std::fs::read_to_string(format!("/proc/{pid}/stat")) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => ProcessCustody::Absent,
        Err(_) => ProcessCustody::Unreadable,
        Ok(stat) => custody_from_stat(&stat, recorded_ticks),
    }
}

/// The row's worker was recorded under another boot: its lease carries a `clock_epoch`
/// (`boot_id`) and this boot's differs. Such a process cannot be alive now, whatever
/// `/proc` holds under the same pid. `false` when either side is unknown (a row with no lease
/// carries no boot id; an unreadable `boot_id` gives no clock).
fn recorded_in_another_boot(row: &AttemptRow, clock: Option<&Clock>) -> bool {
    let recorded = row
        .started
        .as_ref()
        .and_then(|s| s.lease.as_ref())
        .map(|l| l.clock_epoch.as_str());
    matches!((recorded, clock), (Some(r), Some(c)) if r != c.clock_epoch)
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

/// Observe every row: custody from `/proc/<pid>/stat` (no pid recorded → `Unobserved`; a row
/// whose lease `clock_epoch` is not this boot's `boot_id` → `Absent` without looking; `ENOENT`
/// → `Absent`; state `Z` or `X` → `Absent`; another error → `Unreadable`; equal start ticks →
/// `LiveSameIdentity`; other ticks → `PidReused`), the workspace (no workspace →
/// `Unobserved`; `ENOENT` → `Absent`; a directory → `Writable{bytes}` walked up to
/// `workspace_readback_bytes`; anything else → `Unreadable`), and the clock. Tasks without a
/// row keep the default custody (`Unobserved`).
#[must_use]
pub fn observe(rows: &[AttemptRow], workspace_readback_bytes: u64) -> Observations {
    let mut observed = Observations::default();
    let clock = read_clock();
    for row in rows {
        let custody = row.pid.map_or(ProcessCustody::Unobserved, |(pid, ticks)| {
            if recorded_in_another_boot(row, clock.as_ref()) {
                ProcessCustody::Absent
            } else {
                process_custody(pid, ticks)
            }
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
    observed.clock = clock;
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
    use crate::store::{AttemptId, AttemptStart, AttemptState, Cleanup, Effect, Lease};

    /// A running row for this test process, its lease issued under `clock_epoch`.
    fn own_row(clock_epoch: &str) -> Result<AttemptRow, Box<dyn std::error::Error>> {
        let task: hee4_contracts::TaskId = "probe-self".parse()?;
        Ok(AttemptRow {
            id: AttemptId::new(&task, 1),
            task_id: task,
            generation: 1,
            dispatch_seq: 2,
            started: Some(AttemptStart {
                receipt_id: "r-probe-self".parse()?,
                permit_id: 1,
                model: "none".into(),
                head_sha: "a".repeat(40).parse()?,
                workspace: "/nonexistent/probe-self".into(),
                lease: Some(Lease {
                    deadline_ms: i64::MAX,
                    clock_epoch: clock_epoch.to_owned(),
                }),
            }),
            started_ms: 1,
            pid: self_identity(),
            state: AttemptState::Running,
            effect: Effect::Pending,
            cleanup: Cleanup::None,
            closed_seq: None,
            outcome: None,
        })
    }

    /// This process is alive with the recorded ticks, so only the boot check can make it
    /// `Absent`: under this boot's id it is `LiveSameIdentity`, under another boot's `Absent`.
    #[test]
    fn custody_after_reboot_is_absent() -> Result<(), Box<dyn std::error::Error>> {
        let boot = read_clock().ok_or("boot_id unreadable")?.clock_epoch;
        let this_boot = own_row(&boot)?;
        let task = this_boot.task_id.clone();
        assert_eq!(
            observe(&[this_boot], 0).process.get(&task),
            Some(&ProcessCustody::LiveSameIdentity)
        );
        let other_boot = own_row("00000000-0000-0000-0000-000000000000")?;
        assert_eq!(
            observe(&[other_boot], 0).process.get(&task),
            Some(&ProcessCustody::Absent)
        );
        Ok(())
    }

    /// A zombie with the recorded ticks is not live: the parsed state `Z` (and `X`) reads
    /// `Absent`, checked on a stat line and on a real unreaped child.
    #[test]
    fn a_zombie_is_not_live_same_identity() -> Result<(), Box<dyn std::error::Error>> {
        let line = |state: &str| {
            format!("77 (w (x) y) {state} 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 999 20")
        };
        assert_eq!(
            custody_from_stat(&line("S"), 999),
            ProcessCustody::LiveSameIdentity
        );
        assert_eq!(custody_from_stat(&line("Z"), 999), ProcessCustody::Absent);
        assert_eq!(custody_from_stat(&line("X"), 999), ProcessCustody::Absent);

        let mut child = std::process::Command::new("true").spawn()?;
        let pid = child.id();
        let path = format!("/proc/{pid}/stat");
        let mut stat = std::fs::read_to_string(&path)?;
        for _ in 0..500 {
            if proc_state(&stat) == Some("Z") {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
            stat = std::fs::read_to_string(&path)?;
        }
        let ticks = start_ticks(&stat).ok_or("no ticks");
        let custody = ticks.map(|t| process_custody(pid, t));
        let state = proc_state(&stat).map(str::to_owned);
        child.wait()?; // reap by this child's handle only
        assert_eq!(
            state.as_deref(),
            Some("Z"),
            "the unreaped child is a zombie"
        );
        assert_eq!(custody?, ProcessCustody::Absent);
        Ok(())
    }

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
