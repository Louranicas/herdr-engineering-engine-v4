//! The spawn door: a bwrap plan built only from a permit, then run under a timeout.
//! No policy: the caller supplies the scope and the mount list; this module only
//! refuses what contradicts them.

use std::io::Read;
use std::os::unix::fs::FileTypeExt;
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
    /// The model door's unix socket (see [`crate::model_door`]), bound read-write at the same
    /// path and named to the candidate as `HEE4_MODEL_SOCKET`. It must be listed, absolute and a
    /// socket when the plan is built. There is no network: the door is the only path out.
    pub model_door: Option<PathBuf>,
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
    /// The model door is not a unix socket: binding it would be a second writable tree.
    #[error("model door is not a socket (a second writable bind): {0}")]
    DoorNotSocket(PathBuf),
}

/// The env var naming the model door inside the namespace.
pub const MODEL_SOCKET_ENV: &str = "HEE4_MODEL_SOCKET";

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
/// The argv always carries `--unshare-all --unshare-net`; there is no `--share-net` path. The
/// writable binds are the work dir and, when present, the model door, which must be a socket.
///
/// # Errors
/// [`HostRefusal`] for an out-of-scope command, relative path, unlisted mount, or a model door
/// that is not a socket.
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
    let mounts = || {
        ns.ro_binds
            .iter()
            .chain([&ns.work_dir])
            .chain(&ns.model_door)
    };
    for p in mounts().chain([&command.program]) {
        if !p.is_absolute() {
            return Err(HostRefusal::RelativePath(p.clone()));
        }
    }
    for p in mounts() {
        if !ns.listed_mounts.contains(p) {
            return Err(HostRefusal::UnlistedMount(p.clone()));
        }
    }
    if let Some(door) = &ns.model_door {
        let is_socket = std::fs::symlink_metadata(door).is_ok_and(|m| m.file_type().is_socket());
        if !is_socket {
            return Err(HostRefusal::DoorNotSocket(door.clone()));
        }
    }
    let s = |p: &Path| p.to_string_lossy().into_owned();
    let mut argv: Vec<String> = [
        "--unshare-all",
        "--unshare-net",
        "--die-with-parent",
        "--new-session",
    ]
    .map(String::from)
    .to_vec();
    for p in &ns.ro_binds {
        argv.extend(["--ro-bind".into(), s(p), s(p)]);
    }
    argv.extend(["--bind".into(), s(&ns.work_dir), s(&ns.work_dir)]);
    if let Some(door) = &ns.model_door {
        argv.extend(["--bind".into(), s(door), s(door)]);
        argv.extend(["--setenv".into(), MODEL_SOCKET_ENV.into(), s(door)]);
    }
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
    /// `/proc/<pid>/stat` could not be read or parsed right after the spawn; the child was
    /// killed and reaped, so no process runs under an unknown identity.
    #[error("process identity unreadable for pid {pid}: {reason}")]
    Identity {
        /// The child's pid at the time of the refusal.
        pid: u32,
        /// What could not be read or parsed.
        reason: String,
    },
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

/// Bytes of `/proc/<pid>/stat` read at most (AP-04: the bound comes before the read).
const STAT_READ_MAX: u64 = 4096;

/// A spawned child that has not been waited on yet, with its process identity.
///
/// `pid` is the spawned program's pid (bwrap's, for a planned spawn; the candidate dies with it
/// under `--die-with-parent`) and `start_ticks` is field 22 of `/proc/<pid>/stat`, the process
/// start time in clock ticks since boot. The pair is what a later custody probe compares: a pid
/// reused after this child is reaped carries a different `start_ticks`.
#[derive(Debug)]
#[must_use = "a Started child must be waited on, or it is left running"]
pub struct Started {
    /// The child's pid.
    pub pid: u32,
    /// Field 22 of `/proc/<pid>/stat`, read once before any wait.
    pub start_ticks: u64,
    child: std::process::Child,
    out: thread::JoinHandle<Vec<u8>>,
    err: thread::JoinHandle<Vec<u8>>,
    start: Instant,
    timeout: Duration,
}

/// Read `/proc/<pid>/stat`, bounded to [`STAT_READ_MAX`] bytes.
fn read_proc_stat(pid: u32) -> std::io::Result<String> {
    let mut s = String::new();
    std::fs::File::open(format!("/proc/{pid}/stat"))?
        .take(STAT_READ_MAX)
        .read_to_string(&mut s)?;
    Ok(s)
}

/// Field 22 (`starttime`) of a `/proc/<pid>/stat` line: the comm in parentheses may contain
/// spaces or parens, so split after the last `)`; index 0 is then field 3, so `starttime` is
/// index 19.
fn start_ticks_from_stat(stat: &str) -> Result<u64, String> {
    let Some(close) = stat.rfind(')') else {
        return Err("no ')' in stat".to_owned());
    };
    let Some(field) = stat[close + 1..].split_whitespace().nth(19) else {
        return Err("stat has fewer than 22 fields".to_owned());
    };
    field
        .parse::<u64>()
        .map_err(|e| format!("starttime {field:?} not a u64: {e}"))
}

/// Spawn `plan` and read the child's identity once, with `read_stat` as the `/proc` reader.
/// A reader failure kills and reaps the child before the refusal, so no child outlives an
/// unknown identity.
fn start_with(
    plan: SpawnPlan,
    read_stat: impl FnOnce(u32) -> std::io::Result<String>,
) -> Result<Started, SpawnError> {
    let start = Instant::now();
    let mut child = Proc::new(plan.program)
        .args(plan.argv)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let out = drain(child.stdout.take());
    let err = drain(child.stderr.take());
    let pid = child.id();
    let identity = read_stat(pid)
        .map_err(|e| e.to_string())
        .and_then(|s| start_ticks_from_stat(&s));
    match identity {
        Ok(start_ticks) => Ok(Started {
            pid,
            start_ticks,
            child,
            out,
            err,
            start,
            timeout: plan.timeout,
        }),
        Err(reason) => {
            child.kill()?;
            child.wait()?;
            Err(SpawnError::Identity { pid, reason })
        }
    }
}

/// Spawn a plan and read the child's `(pid, start_ticks)` identity once, before any wait.
///
/// # Errors
/// [`SpawnError::Io`] if the process could not be started; [`SpawnError::Identity`] if
/// `/proc/<pid>/stat` could not be read or parsed (the child is killed and reaped first).
#[must_use = "the Started handle owns the child; call wait on it"]
pub fn start(plan: SpawnPlan) -> Result<Started, SpawnError> {
    start_with(plan, read_proc_stat)
}

impl Started {
    /// Wait for the child; kill it if the plan's timeout passes.
    ///
    /// # Errors
    /// [`SpawnError::Io`] if the child could not be waited on or killed;
    /// [`SpawnError::TimedOut`] if the deadline passed (the child was killed and reaped).
    #[must_use = "the outcome carries the exit code and captured output"]
    pub fn wait(self) -> Result<SpawnOutcome, SpawnError> {
        let Self {
            mut child,
            out,
            err,
            start,
            timeout,
            ..
        } = self;
        let status = loop {
            if let Some(st) = child.try_wait()? {
                break st;
            }
            if start.elapsed() >= timeout {
                child.kill()?;
                child.wait()?;
                return Err(SpawnError::TimedOut(timeout));
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
}

/// Run a plan to completion: [`start`] then [`Started::wait`], nothing else.
///
/// # Errors
/// [`SpawnError`] on io failure, an unreadable identity, or timeout.
pub fn run(plan: SpawnPlan) -> Result<SpawnOutcome, SpawnError> {
    start(plan)?.wait()
}

#[cfg(test)]
mod tests {
    type R = Result<(), Box<dyn std::error::Error>>;
    use super::*;

    fn p(s: &str) -> PathBuf {
        PathBuf::from(s)
    }

    fn fixture(program: &str, model_door: Option<PathBuf>) -> (Permit, Command, NamespacePlan) {
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
        let mut listed_mounts = vec![p("/usr"), p("/lib"), p("/work")];
        listed_mounts.extend(model_door.clone());
        let ns = NamespacePlan {
            listed_mounts,
            ro_binds: vec![p("/usr"), p("/lib")],
            work_dir: p("/work"),
            model_door,
            timeout: Duration::from_secs(5),
        };
        (permit, cmd, ns)
    }

    /// A real unix socket at a fresh temp path (the listener must outlive the plan call).
    fn door(name: &str) -> std::io::Result<(std::os::unix::net::UnixListener, PathBuf)> {
        let path =
            std::env::temp_dir().join(format!("hee4-spawn-{}-{name}.sock", std::process::id()));
        let _ = std::fs::remove_file(&path);
        Ok((std::os::unix::net::UnixListener::bind(&path)?, path))
    }

    #[test]
    fn argv_for_fixture_is_exact() -> R {
        let (_l, d) = door("argv")?;
        let (permit, cmd, ns) = fixture("/usr/bin/true", Some(d.clone()));
        let sp = plan(&permit, cmd, ns)?;
        let d = d.display();
        let want = format!(
            "--unshare-all --unshare-net --die-with-parent --new-session --ro-bind /usr /usr \
             --ro-bind /lib /lib --bind /work /work --bind {d} {d} --setenv HEE4_MODEL_SOCKET {d} \
             --chdir /work -- /usr/bin/true --x"
        );
        println!("argv: bwrap {}", sp.argv.join(" "));
        assert_eq!(sp.argv.join(" "), want);
        assert!(!sp.argv.iter().any(|a| a == "--share-net"));
        assert_eq!(sp.program, p(BWRAP));
        assert_eq!(sp.receipt, ReceiptId("r-1".into()));
        let _ = std::fs::remove_file(d.to_string());
        Ok(())
    }

    #[test]
    fn no_door_means_no_network_and_one_writable_bind() -> R {
        let (permit, cmd, ns) = fixture("/usr/bin/true", None);
        let sp = plan(&permit, cmd, ns)?;
        let want = "--unshare-all --unshare-net --die-with-parent --new-session --ro-bind /usr /usr \
                    --ro-bind /lib /lib --bind /work /work --chdir /work -- /usr/bin/true --x";
        assert_eq!(sp.argv.join(" "), want);
        Ok(())
    }

    /// The door is the only writable path besides the work dir: a door that is a directory (or
    /// a regular file, or missing) would be a second writable bind, and is refused.
    #[test]
    fn second_writable_bind_is_refused() -> R {
        let dir = std::env::temp_dir().join(format!("hee4-spawn-{}-dir", std::process::id()));
        std::fs::create_dir_all(&dir)?;
        let file = dir.join("plain");
        std::fs::write(&file, b"x")?;
        for not_socket in [dir.clone(), file.clone(), dir.join("missing")] {
            let (permit, cmd, ns) = fixture("/usr/bin/true", Some(not_socket.clone()));
            assert_eq!(
                plan(&permit, cmd, ns),
                Err(HostRefusal::DoorNotSocket(not_socket))
            );
        }
        let _ = std::fs::remove_dir_all(&dir);
        let (permit, cmd, mut ns) = fixture("/usr/bin/true", None);
        ns.model_door = Some(p("/home"));
        assert_eq!(
            plan(&permit, cmd, ns),
            Err(HostRefusal::UnlistedMount(p("/home")))
        );
        Ok(())
    }

    #[test]
    fn unlisted_mount_is_refused() {
        let (permit, cmd, mut ns) = fixture("/usr/bin/true", None);
        ns.ro_binds.push(p("/etc"));
        assert_eq!(
            plan(&permit, cmd, ns),
            Err(HostRefusal::UnlistedMount(p("/etc")))
        );
    }

    #[test]
    fn uncovered_command_is_refused() {
        let (permit, mut cmd, ns) = fixture("/usr/bin/true", None);
        cmd.program = p("/usr/bin/false");
        let id = permit.id();
        assert_eq!(
            plan(&permit, cmd, ns),
            Err(HostRefusal::OutOfScope(id, p("/usr/bin/false")))
        );
    }

    #[test]
    fn relative_path_is_refused() {
        let (permit, cmd, mut ns) = fixture("/usr/bin/true", None);
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

    /// A no-bwrap plan for `/usr/bin/sleep 30` with the given kill deadline.
    fn sleep_plan(timeout: Duration) -> SpawnPlan {
        SpawnPlan {
            permit: PermitId(0),
            receipt: ReceiptId(String::new()),
            program: p("/usr/bin/sleep"),
            argv: vec!["30".into()],
            timeout,
        }
    }

    /// The test's own, independent parse of field 22: last `)`, then whitespace index 19.
    fn proc_starttime(pid: u32) -> Result<u64, Box<dyn std::error::Error>> {
        let stat = std::fs::read_to_string(format!("/proc/{pid}/stat"))?;
        let close = stat.rfind(')').ok_or("no ')'")?;
        let field = stat[close + 1..]
            .split_whitespace()
            .nth(19)
            .ok_or("fewer than 22 fields")?;
        Ok(field.parse()?)
    }

    #[test]
    fn start_exposes_identity_matching_proc_stat() -> R {
        let s = start(sleep_plan(Duration::from_millis(150)))?;
        println!(
            "started pid={} start_ticks={} (child.id()={})",
            s.pid,
            s.start_ticks,
            s.child.id()
        );
        assert_eq!(s.pid, s.child.id());
        assert!(s.start_ticks > 0);
        assert_eq!(s.start_ticks, proc_starttime(s.pid)?);
        let t = Instant::now();
        assert!(matches!(s.wait(), Err(SpawnError::TimedOut(_))));
        assert!(t.elapsed() < Duration::from_secs(5));
        Ok(())
    }

    #[test]
    fn reused_pid_probe_cannot_match() -> R {
        /// A custody probe keyed on the persisted pair: both halves must agree.
        fn probe_matches(persisted: (u32, u64), live: (u32, u64)) -> bool {
            persisted == live
        }
        let first = start(sleep_plan(Duration::from_millis(50)))?;
        let a = (first.pid, first.start_ticks);
        assert!(matches!(first.wait(), Err(SpawnError::TimedOut(_))));
        let second = start(sleep_plan(Duration::from_millis(50)))?;
        let b = (second.pid, second.start_ticks);
        println!("first={a:?} second={b:?}");
        // The pair differs even where the pid half coincides: a probe keyed on the first
        // child's pair never matches a pair carrying the first pid but the second start.
        assert!(!probe_matches(a, b));
        assert!(!probe_matches(a, (a.0, b.1)));
        assert!(!probe_matches((b.0, a.1), b));
        assert_ne!(a.1, b.1);
        // Independently: a live child's /proc starttime is its start_ticks, and another pid
        // (this test process) carries a different starttime. The second child started at least
        // 50 ms (five ticks at CLK_TCK=100) after this process, so the inequality is not a race;
        // the first child may share this process's tick, so it is not used here.
        assert_eq!(second.start_ticks, proc_starttime(second.pid)?);
        let me = proc_starttime(std::process::id())?;
        println!("self starttime={me}");
        assert_ne!((std::process::id(), me), b);
        assert_ne!(me, second.start_ticks);
        assert!(matches!(second.wait(), Err(SpawnError::TimedOut(_))));
        Ok(())
    }

    #[test]
    fn run_equals_start_then_wait() -> R {
        let true_plan = || SpawnPlan {
            permit: PermitId(0),
            receipt: ReceiptId(String::new()),
            program: p("/usr/bin/true"),
            argv: vec![],
            timeout: Duration::from_secs(5),
        };
        let via_run = run(true_plan())?;
        let via_split = start(true_plan())?.wait()?;
        assert_eq!(via_run.exit, Some(0));
        assert_eq!(via_split.exit, Some(0));
        assert_eq!(via_run.stdout, via_split.stdout);
        Ok(())
    }

    #[test]
    fn unreadable_identity_is_refused_and_child_reaped() -> R {
        let got = start_with(sleep_plan(Duration::from_secs(5)), |_| {
            Err(std::io::Error::other("injected: stat unreadable"))
        });
        let Err(e) = got else {
            return Err("identity reader failed yet start returned Ok".into());
        };
        println!("refused: {e}");
        assert!(matches!(e, SpawnError::Identity { .. }));
        assert!(e.to_string().contains("process identity unreadable"));
        let SpawnError::Identity { pid, reason } = e else {
            return Err("not Identity".into());
        };
        assert!(reason.contains("injected"));
        // Reaped: the pid is gone from /proc (or, if reused in the meantime, is not our sleep).
        let gone = match std::fs::read_to_string(format!("/proc/{pid}/stat")) {
            Err(_) => true,
            Ok(stat) => !stat.contains("(sleep)"),
        };
        assert!(gone, "pid {pid} still present in /proc");

        let garbage = start_with(sleep_plan(Duration::from_secs(5)), |_| {
            Ok("1 (sleep) S 1 2 3".to_owned())
        });
        assert!(matches!(garbage, Err(SpawnError::Identity { .. })));
        Ok(())
    }

    #[test]
    fn bwrap_runs_true() -> R {
        if !Path::new(BWRAP).exists() {
            println!("UNMEASURED: {BWRAP} absent; bwrap_runs_true skipped");
            return Ok(());
        }
        let work = std::env::temp_dir().join(format!("hee4-host-w-{}", std::process::id()));
        std::fs::create_dir_all(&work)?;
        let (permit, cmd, mut ns) = fixture("/usr/bin/true", None);
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
