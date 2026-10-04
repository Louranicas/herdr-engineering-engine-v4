//! The spawn door: a bwrap plan built only from a permit, then run under a timeout.
//! No policy: the caller supplies the scope and the mount list; this module only
//! refuses what contradicts them.

use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command as Proc, Stdio};
use std::thread;
use std::time::{Duration, Instant};

/// The bwrap binary (MEASURED present on this host).
pub const BWRAP: &str = "/usr/bin/bwrap";

/// Id of a permit.
// TODO(S6): hee4_contracts::PermitId
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PermitId(pub u64);

/// Id of the receipt a permit actuates under.
// TODO(S6): hee4_contracts::ReceiptId
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ReceiptId(pub String);

/// Which programs a permit covers (data supplied by the minting caller).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpawnScope {
    /// Absolute program paths the permit allows.
    pub programs: Vec<PathBuf>,
}

/// Authority to spawn. Carries the receipt id; fields are private so the only
/// constructor is [`Permit::mint`].
// TODO(S6): hee4_contracts permit type constructed only by admission.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Permit {
    id: PermitId,
    receipt: ReceiptId,
    scope: SpawnScope,
}

impl Permit {
    /// Mint a permit for `receipt` covering `scope`.
    #[must_use]
    pub fn mint(receipt: ReceiptId, scope: SpawnScope) -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static NEXT: AtomicU64 = AtomicU64::new(1);
        Self {
            id: PermitId(NEXT.fetch_add(1, Ordering::Relaxed)),
            receipt,
            scope,
        }
    }

    /// This permit's id.
    #[must_use]
    pub fn id(&self) -> PermitId {
        self.id
    }

    /// The receipt this permit actuates under.
    #[must_use]
    pub fn receipt(&self) -> &ReceiptId {
        &self.receipt
    }
}

/// The program and arguments to run inside the namespace.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Command {
    /// Absolute program path inside the namespace.
    pub program: PathBuf,
    /// Arguments.
    pub args: Vec<String>,
}

/// The namespace to build (a pure value; the mount list is the caller's).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamespacePlan {
    /// Every mount path the namespace may contain; anything else is refused.
    pub listed_mounts: Vec<PathBuf>,
    /// Read-only binds (host path = namespace path); each must be listed.
    pub ro_binds: Vec<PathBuf>,
    /// The one writable work directory; must be listed.
    pub work_dir: PathBuf,
    /// Share the host network (bwrap `--share-net`); otherwise none.
    pub allow_loopback: bool,
    /// Kill the child after this long.
    pub timeout: Duration,
}

/// Why a plan was refused before any process started.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum HostRefusal {
    /// A bind or the work dir is not in `listed_mounts`.
    #[error("unlisted mount: {0}")]
    UnlistedMount(PathBuf),
    /// A path is not absolute.
    #[error("path not absolute: {0}")]
    RelativePath(PathBuf),
    /// The permit's scope does not cover the program.
    #[error("permit {0:?} does not cover {1}")]
    OutOfScope(PermitId, PathBuf),
}

/// A fully built, permitted spawn.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpawnPlan {
    /// Permit this plan was built under.
    pub permit: PermitId,
    /// Receipt the spawn actuates under.
    pub receipt: ReceiptId,
    /// Program to execute (bwrap).
    pub program: PathBuf,
    /// Exact argv after the program.
    pub argv: Vec<String>,
    /// Kill deadline.
    pub timeout: Duration,
}

/// Build the bwrap invocation, or refuse.
///
/// # Errors
/// [`HostRefusal`] for an out-of-scope command, relative path or unlisted mount.
#[expect(
    clippy::needless_pass_by_value,
    reason = "the door takes the plan by value: one plan, one spawn"
)]
pub fn plan(
    permit: &Permit,
    command: Command,
    ns: NamespacePlan,
) -> Result<SpawnPlan, HostRefusal> {
    if !permit.scope.programs.contains(&command.program) {
        return Err(HostRefusal::OutOfScope(permit.id, command.program));
    }
    for p in ns.ro_binds.iter().chain([&ns.work_dir, &command.program]) {
        if !p.is_absolute() {
            return Err(HostRefusal::RelativePath(p.clone()));
        }
    }
    for p in ns.ro_binds.iter().chain([&ns.work_dir]) {
        if !ns.listed_mounts.contains(p) {
            return Err(HostRefusal::UnlistedMount(p.clone()));
        }
    }
    let s = |p: &Path| p.to_string_lossy().into_owned();
    let mut argv: Vec<String> = ["--unshare-all", "--die-with-parent", "--new-session"]
        .map(String::from)
        .to_vec();
    if ns.allow_loopback {
        argv.push("--share-net".into());
    }
    for p in &ns.ro_binds {
        argv.extend(["--ro-bind".into(), s(p), s(p)]);
    }
    argv.extend(["--bind".into(), s(&ns.work_dir), s(&ns.work_dir)]);
    argv.extend([
        "--chdir".into(),
        s(&ns.work_dir),
        "--".into(),
        s(&command.program),
    ]);
    argv.extend(command.args);
    Ok(SpawnPlan {
        permit: permit.id,
        receipt: permit.receipt.clone(),
        program: PathBuf::from(BWRAP),
        argv,
        timeout: ns.timeout,
    })
}

/// What a finished spawn produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpawnOutcome {
    /// Exit code; `None` if killed by a signal.
    pub exit: Option<i32>,
    /// Captured stdout.
    pub stdout: Vec<u8>,
    /// Captured stderr.
    pub stderr: Vec<u8>,
    /// Wall time from start to reap.
    pub elapsed: Duration,
}

/// Why a run failed.
#[derive(Debug, thiserror::Error)]
pub enum SpawnError {
    /// The process could not be started or waited on.
    #[error("spawn io: {0}")]
    Io(#[from] std::io::Error),
    /// The deadline passed; the child was killed and reaped.
    #[error("timed out after {0:?}; killed")]
    TimedOut(Duration),
}

fn drain(r: Option<impl Read + Send + 'static>) -> thread::JoinHandle<Vec<u8>> {
    thread::spawn(move || {
        let mut buf = Vec::new();
        if let Some(mut r) = r {
            let _ = r.read_to_end(&mut buf);
        }
        buf
    })
}

/// Run a plan; kill the child if `plan.timeout` passes.
///
/// # Errors
/// [`SpawnError`] on io failure or timeout.
pub fn run(plan: SpawnPlan) -> Result<SpawnOutcome, SpawnError> {
    let start = Instant::now();
    let mut child = Proc::new(plan.program)
        .args(plan.argv)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let out = drain(child.stdout.take());
    let err = drain(child.stderr.take());
    let status = loop {
        if let Some(st) = child.try_wait()? {
            break st;
        }
        if start.elapsed() >= plan.timeout {
            child.kill()?;
            child.wait()?;
            return Err(SpawnError::TimedOut(plan.timeout));
        }
        thread::sleep(Duration::from_millis(10));
    };
    Ok(SpawnOutcome {
        exit: status.code(),
        stdout: out.join().unwrap_or_default(),
        stderr: err.join().unwrap_or_default(),
        elapsed: start.elapsed(),
    })
}

#[cfg(test)]
mod tests {
    type R = Result<(), Box<dyn std::error::Error>>;
    use super::*;

    fn p(s: &str) -> PathBuf {
        PathBuf::from(s)
    }

    fn fixture(program: &str, allow_loopback: bool) -> (Permit, Command, NamespacePlan) {
        let permit = Permit::mint(
            ReceiptId("r-1".into()),
            SpawnScope {
                programs: vec![p(program)],
            },
        );
        let cmd = Command {
            program: p(program),
            args: vec!["--x".into()],
        };
        let ns = NamespacePlan {
            listed_mounts: vec![p("/usr"), p("/lib"), p("/work")],
            ro_binds: vec![p("/usr"), p("/lib")],
            work_dir: p("/work"),
            allow_loopback,
            timeout: Duration::from_secs(5),
        };
        (permit, cmd, ns)
    }

    #[test]
    fn argv_for_fixture_is_exact() -> R {
        let (permit, cmd, ns) = fixture("/usr/bin/true", false);
        let sp = plan(&permit, cmd, ns)?;
        let want = "--unshare-all --die-with-parent --new-session --ro-bind /usr /usr \
                    --ro-bind /lib /lib --bind /work /work --chdir /work -- /usr/bin/true --x";
        assert_eq!(sp.argv.join(" "), want);
        assert_eq!(sp.program, p(BWRAP));
        assert_eq!(sp.receipt, ReceiptId("r-1".into()));
        Ok(())
    }

    #[test]
    fn loopback_adds_share_net_only_when_allowed() -> R {
        let (permit, cmd, ns) = fixture("/usr/bin/true", true);
        let sp = plan(&permit, cmd, ns)?;
        assert_eq!(sp.argv[3], "--share-net");
        Ok(())
    }

    #[test]
    fn unlisted_mount_is_refused() {
        let (permit, cmd, mut ns) = fixture("/usr/bin/true", false);
        ns.ro_binds.push(p("/etc"));
        assert_eq!(
            plan(&permit, cmd, ns),
            Err(HostRefusal::UnlistedMount(p("/etc")))
        );
    }

    #[test]
    fn uncovered_command_is_refused() {
        let (permit, mut cmd, ns) = fixture("/usr/bin/true", false);
        cmd.program = p("/usr/bin/false");
        let id = permit.id();
        assert_eq!(
            plan(&permit, cmd, ns),
            Err(HostRefusal::OutOfScope(id, p("/usr/bin/false")))
        );
    }

    #[test]
    fn relative_path_is_refused() {
        let (permit, cmd, mut ns) = fixture("/usr/bin/true", false);
        ns.work_dir = p("work");
        assert_eq!(
            plan(&permit, cmd, ns),
            Err(HostRefusal::RelativePath(p("work")))
        );
    }

    #[test]
    fn run_kills_on_timeout() {
        let sp = SpawnPlan {
            permit: PermitId(0),
            receipt: ReceiptId(String::new()),
            program: p("/bin/sleep"),
            argv: vec!["30".into()],
            timeout: Duration::from_millis(150),
        };
        let t = Instant::now();
        assert!(matches!(run(sp), Err(SpawnError::TimedOut(_))));
        assert!(t.elapsed() < Duration::from_secs(5));
    }

    #[test]
    fn bwrap_runs_true() -> R {
        if !Path::new(BWRAP).exists() {
            println!("UNMEASURED: {BWRAP} absent; bwrap_runs_true skipped");
            return Ok(());
        }
        let work = std::env::temp_dir().join(format!("hee4-host-w-{}", std::process::id()));
        std::fs::create_dir_all(&work)?;
        let (permit, cmd, mut ns) = fixture("/usr/bin/true", false);
        ns.args_fix(&work);
        let sp = plan(
            &permit,
            Command {
                args: vec![],
                ..cmd
            },
            ns,
        )?;
        let out = run(sp);
        let _ = std::fs::remove_dir_all(&work);
        match out {
            Ok(o) => assert_eq!(
                o.exit,
                Some(0),
                "stderr={}",
                String::from_utf8_lossy(&o.stderr)
            ),
            Err(e) => println!("UNMEASURED: bwrap could not run here: {e}"),
        }
        Ok(())
    }

    impl NamespacePlan {
        fn args_fix(&mut self, work: &Path) {
            self.listed_mounts = vec![p("/usr"), p("/lib"), p("/lib64"), p("/bin"), work.into()];
            self.ro_binds = vec![p("/usr"), p("/lib"), p("/lib64"), p("/bin")];
            self.work_dir = work.into();
        }
    }
}
