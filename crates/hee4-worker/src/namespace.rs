//! The pure namespace plan for one candidate attempt.
//!
//! This module owns the plan value only; `hee4_host::spawn::plan` renders the argv and refuses
//! anything unlisted, and `spawn::run` is the only stop.

use std::path::{Path, PathBuf};
use std::time::Duration;

use hee4_contracts::TaskId;
use hee4_host::spawn::NamespacePlan;

use crate::WorkerError;

/// Read-only system binds, as host path = namespace path.
pub const RO_BINDS: [&str; 4] = ["/usr", "/lib", "/lib64", "/bin"];
/// Bound read-only only when present on this host.
pub const RO_BINDS_IF_PRESENT: [&str; 1] = ["/etc/alternatives"];

/// What the plan needs to know about a task.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamespaceTask {
    task_id: TaskId,
    work_dir: PathBuf,
    /// The task talks to the local model, so loopback is shared.
    pub needs_model: bool,
    /// Kill the child after this long.
    pub timeout: Duration,
}

impl NamespaceTask {
    /// A task whose work dir is `<work_root>/<task_id>` (`work_root` is `$HEE4_WORK`).
    ///
    /// # Errors
    /// [`WorkerError::WorkRoot`] when `work_root` is relative or is, or lies inside, a read-only
    /// system path, since a writable bind there would reach outside the work dir.
    pub fn new(
        task_id: TaskId,
        work_root: &Path,
        needs_model: bool,
        timeout: Duration,
    ) -> Result<Self, WorkerError> {
        if !work_root.is_absolute() {
            return Err(WorkerError::WorkRoot(format!(
                "{} is relative",
                work_root.display()
            )));
        }
        let work_dir = work_root.join(task_id.as_str());
        let inside = RO_BINDS
            .iter()
            .chain(&RO_BINDS_IF_PRESENT)
            .any(|ro| work_dir.starts_with(ro));
        if inside || work_dir == Path::new("/") {
            return Err(WorkerError::WorkRoot(format!(
                "{} is inside a read-only system path",
                work_dir.display()
            )));
        }
        Ok(Self {
            task_id,
            work_dir,
            needs_model,
            timeout,
        })
    }

    /// The task's id.
    #[must_use]
    pub fn task_id(&self) -> &TaskId {
        &self.task_id
    }

    /// `<work_root>/<task_id>`.
    #[must_use]
    pub fn work_dir(&self) -> &Path {
        &self.work_dir
    }
}

/// `$HEE4_WORK`, when set to a non-empty value.
#[must_use]
pub fn work_root_from_env() -> Option<PathBuf> {
    std::env::var_os("HEE4_WORK")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
}

/// The plan for `task`, binding `/etc/alternatives` only if it exists on this host.
#[must_use]
pub fn plan_for(task: &NamespaceTask) -> NamespacePlan {
    plan_for_with(task, Path::exists)
}

/// [`plan_for`] with the presence check supplied (so a test need not depend on the host).
#[must_use]
pub fn plan_for_with(task: &NamespaceTask, present: impl Fn(&Path) -> bool) -> NamespacePlan {
    let ro_binds: Vec<PathBuf> = RO_BINDS
        .iter()
        .chain(&RO_BINDS_IF_PRESENT)
        .map(PathBuf::from)
        .filter(|p| RO_BINDS.iter().any(|r| Path::new(r) == p) || present(p))
        .collect();
    let mut listed_mounts = ro_binds.clone();
    listed_mounts.push(task.work_dir.clone());
    NamespacePlan {
        listed_mounts,
        ro_binds,
        work_dir: task.work_dir.clone(),
        allow_loopback: task.needs_model,
        timeout: task.timeout,
    }
}

#[cfg(test)]
mod tests {
    type R = Result<(), Box<dyn std::error::Error>>;
    use super::*;
    use hee4_host::spawn::{Command, Permit, ReceiptId, SpawnScope, plan};

    fn task(model: bool) -> Result<NamespaceTask, Box<dyn std::error::Error>> {
        Ok(NamespaceTask::new(
            "t-1".parse()?,
            Path::new("/var/hee4-work"),
            model,
            Duration::from_secs(5),
        )?)
    }

    #[test]
    fn work_dir_is_root_slash_task_id() -> R {
        assert_eq!(
            plan_for(&task(false)?).work_dir,
            Path::new("/var/hee4-work/t-1")
        );
        Ok(())
    }

    #[test]
    fn loopback_only_when_model_needed() -> R {
        assert!(!plan_for(&task(false)?).allow_loopback);
        assert!(plan_for(&task(true)?).allow_loopback);
        Ok(())
    }

    #[test]
    fn alternatives_bound_only_if_present() -> R {
        let t = task(false)?;
        let with = plan_for_with(&t, |_| true);
        let without = plan_for_with(&t, |_| false);
        assert!(with.ro_binds.contains(&PathBuf::from("/etc/alternatives")));
        assert!(
            !without
                .ro_binds
                .contains(&PathBuf::from("/etc/alternatives"))
        );
        for p in ["/usr", "/lib", "/lib64", "/bin"] {
            assert!(without.ro_binds.contains(&PathBuf::from(p)), "{p}");
        }
        Ok(())
    }

    /// Rung-2 door: through the host's own renderer, the only writable bind is the work dir.
    #[test]
    fn no_writable_bind_outside_work_dir() -> R {
        let t = task(true)?;
        let ns = plan_for_with(&t, |_| true);
        let permit = Permit::mint(
            ReceiptId("r".into()),
            SpawnScope {
                programs: vec![PathBuf::from("/usr/bin/true")],
            },
        );
        let sp = plan(
            &permit,
            Command {
                program: PathBuf::from("/usr/bin/true"),
                args: vec![],
            },
            ns.clone(),
        )?;
        let writable: Vec<&str> = sp
            .argv
            .windows(3)
            .filter(|w| w[0] == "--bind" || w[0] == "--dev-bind" || w[0] == "--tmpfs")
            .map(|w| w[1].as_str())
            .collect();
        assert_eq!(writable, vec!["/var/hee4-work/t-1"]);
        for ro in &ns.ro_binds {
            assert!(
                !t.work_dir().starts_with(ro),
                "work dir inside {}",
                ro.display()
            );
        }
        Ok(())
    }

    #[test]
    fn work_root_inside_system_path_or_relative_is_refused() -> R {
        let id: TaskId = "t".parse()?;
        assert!(
            NamespaceTask::new(id.clone(), Path::new("/usr/work"), false, Duration::ZERO).is_err()
        );
        assert!(NamespaceTask::new(id, Path::new("work"), false, Duration::ZERO).is_err());
        Ok(())
    }
}
