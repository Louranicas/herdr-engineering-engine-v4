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
    door: PathBuf,
    /// The task talks to the local model, so the plan carries the model door. The candidate
    /// never has a network either way.
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
        let door = door_under(work_root, &task_id)?;
        Ok(Self {
            task_id,
            work_dir,
            door,
            needs_model,
            timeout,
        })
    }

    /// Serve the door from `root` instead of the work root: `<root>/<task_id>.model.sock`. The
    /// runtime dir (`$XDG_RUNTIME_DIR/hee4`) is short and already the unit's; a work root under
    /// a deep path pushed the door past [`DOOR_PATH_MAX`] once (the gate's export, 109 bytes).
    ///
    /// # Errors
    /// [`WorkerError::WorkRoot`] for a relative root; [`WorkerError::DoorPath`] when the path
    /// would not fit.
    pub fn with_door_root(mut self, root: &Path) -> Result<Self, WorkerError> {
        if !root.is_absolute() {
            return Err(WorkerError::WorkRoot(format!(
                "door root {} is relative",
                root.display()
            )));
        }
        self.door = door_under(root, &self.task_id)?;
        Ok(self)
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

    /// `<work_root>/<task_id>.model.sock`: where the attempt serves the model door. A sibling of
    /// the work dir, never inside it, so the candidate cannot replace it.
    #[must_use]
    pub fn door_path(&self) -> &Path {
        &self.door
    }
}

/// The longest socket path `bind(2)` takes on Linux: `sun_path` is 108 bytes with its NUL.
pub const DOOR_PATH_MAX: usize = 107;

/// `<root>/<task_id>.model.sock`, refused by name when it would not fit `sun_path`.
fn door_under(root: &Path, task_id: &TaskId) -> Result<PathBuf, WorkerError> {
    let door = root.join(format!("{}.model.sock", task_id.as_str()));
    let len = door.as_os_str().len();
    if len > DOOR_PATH_MAX {
        return Err(WorkerError::DoorPath {
            len,
            max: DOOR_PATH_MAX,
        });
    }
    Ok(door)
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
    let model_door = task.needs_model.then(|| task.door.clone());
    let mut listed_mounts = ro_binds.clone();
    listed_mounts.push(task.work_dir.clone());
    listed_mounts.extend(model_door.clone());
    NamespacePlan {
        listed_mounts,
        ro_binds,
        work_dir: task.work_dir.clone(),
        model_door,
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
    fn door_only_when_model_needed() -> R {
        assert_eq!(plan_for(&task(false)?).model_door, None);
        let with = plan_for(&task(true)?);
        assert_eq!(
            with.model_door.as_deref(),
            Some(Path::new("/var/hee4-work/t-1.model.sock"))
        );
        assert!(
            with.listed_mounts
                .contains(&"/var/hee4-work/t-1.model.sock".into())
        );
        Ok(())
    }

    #[test]
    fn door_moves_to_the_door_root_and_the_work_dir_stays() -> R {
        let t = task(true)?.with_door_root(Path::new("/run/user/1000/hee4"))?;
        assert_eq!(
            t.door_path(),
            Path::new("/run/user/1000/hee4/t-1.model.sock")
        );
        assert_eq!(t.work_dir(), Path::new("/var/hee4-work/t-1"));
        let p = plan_for(&t);
        assert_eq!(
            p.model_door.as_deref(),
            Some(Path::new("/run/user/1000/hee4/t-1.model.sock"))
        );
        assert!(
            !p.listed_mounts
                .contains(&"/var/hee4-work/t-1.model.sock".into())
        );
        Ok(())
    }

    #[test]
    fn a_door_path_over_sun_path_is_refused_by_name() -> R {
        // A 92-byte root plus the 15-byte "/t-1.model.sock" is exactly the limit; one more is not.
        let fits = PathBuf::from("/").join("d".repeat(DOOR_PATH_MAX - 16));
        assert!(task(true)?.with_door_root(&fits).is_ok());
        let deep = PathBuf::from("/").join("d".repeat(DOOR_PATH_MAX - 15));
        let Err(err) = NamespaceTask::new("t-1".parse()?, &deep, true, Duration::from_secs(5))
        else {
            return Err("accepted a door path that cannot bind".into());
        };
        assert!(
            matches!(err, WorkerError::DoorPath { len, max } if len == max + 1 && max == DOOR_PATH_MAX),
            "{err}"
        );
        let Err(err) = task(true)?.with_door_root(&deep) else {
            return Err("accepted a door root that cannot bind".into());
        };
        assert!(matches!(err, WorkerError::DoorPath { .. }), "{err}");
        assert!(
            task(true)?.with_door_root(Path::new("rt")).is_err(),
            "a relative door root"
        );
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

    /// Rung-2 door: through the host's own renderer, the only writable binds are the work dir
    /// and the model door (a socket), and the network is unshared.
    #[test]
    fn no_writable_bind_outside_work_dir_and_door() -> R {
        let root = std::env::temp_dir().join(format!("hee4-ns-{}", std::process::id()));
        std::fs::create_dir_all(&root)?;
        let t = NamespaceTask::new("t-1".parse()?, &root, true, Duration::from_secs(5))?;
        let _ = std::fs::remove_file(t.door_path());
        let _listener = std::os::unix::net::UnixListener::bind(t.door_path())?;
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
        let writable: Vec<String> = sp
            .argv
            .windows(3)
            .filter(|w| w[0] == "--bind" || w[0] == "--dev-bind" || w[0] == "--tmpfs")
            .map(|w| w[1].clone())
            .collect();
        let (work, door) = (t.work_dir().display(), t.door_path().display());
        assert_eq!(writable, vec![work.to_string(), door.to_string()]);
        assert!(sp.argv.iter().any(|a| a == "--unshare-net"));
        assert!(!sp.argv.iter().any(|a| a == "--share-net"));
        for ro in &ns.ro_binds {
            assert!(
                !t.work_dir().starts_with(ro),
                "work dir inside {}",
                ro.display()
            );
        }
        let _ = std::fs::remove_dir_all(&root);
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
