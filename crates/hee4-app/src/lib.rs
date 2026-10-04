//! `hee4-app` (K6): the `hee4` binary's parts. Startup order, the control socket, the actions,
//! the synchronous dispatcher and `doctor`. See `FLOW.md`.
//!
//! Startup: `Store::open` → `recovery::reconcile` → dispatcher thread → bind → serve. A
//! mutating action is refused `not_ready` while the ledger's `recovery_complete` is false.

pub mod actions;
pub mod dispatcher;
pub mod doctor;
pub mod socket;
pub mod stream;
pub mod wire;

use std::os::unix::fs::MetadataExt as _;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use hee4_core::{Observations, ProcessCustody, Store, reconcile};

/// The build commit (`git rev-parse HEAD` at build time, or `unknown`).
pub const HEAD: &str = env!("HEE4_HEAD");

/// The engine version.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// The standards file the verdict is judged by, pinned at build time.
pub const GATE_TOML: &[u8] = include_bytes!("../../../gate.toml");

/// The first 12 digits of [`HEAD`].
#[must_use]
pub fn head12() -> &'static str {
    HEAD.get(..12).unwrap_or(HEAD)
}

/// This process's uid, read from `/proc/self` (no `libc`, no `unsafe`).
#[must_use]
pub fn process_uid() -> Option<u32> {
    std::fs::metadata("/proc/self").ok().map(|m| m.uid())
}

/// The startup worker probe of the work dir. A task whose `<work>/<task_id>` appears in a live
/// process's argv is `PidReused` (alive, and not this process's to re-attach: R07); every other
/// task's worker is `Absent` (R08). MEASURED: a bwrap child outlives `kill -9` of `hee4` (it is
/// re-parented to the user manager), so absence is probed, not assumed.
///
/// # Errors
/// The ledger's task list.
pub fn probe_workers(store: &Store, work: &Path) -> Result<Observations, hee4_core::StoreError> {
    let mut argvs: Vec<(String, Vec<String>)> = Vec::new();
    for entry in std::fs::read_dir("/proc").into_iter().flatten().flatten() {
        let pid = entry.file_name().to_string_lossy().into_owned();
        if pid.bytes().all(|b| b.is_ascii_digit())
            && let Ok(raw) = std::fs::read(entry.path().join("cmdline"))
        {
            let argv = raw
                .split(|b| *b == 0)
                .map(|a| String::from_utf8_lossy(a).into_owned());
            argvs.push((pid, argv.collect()));
        }
    }
    let mut observed = Observations::worker_absent();
    for task in store.task_ids()? {
        let dir = work.join(task.as_str()).to_string_lossy().into_owned();
        if let Some((pid, _)) = argvs.iter().find(|(_, argv)| argv.contains(&dir)) {
            eprintln!("recovery probe task={task} live_worker_pid={pid} custody=PidReused");
            observed.process.insert(task, ProcessCustody::PidReused);
        }
    }
    Ok(observed)
}

/// `hee4 serve` arguments.
#[derive(Debug, Clone)]
pub struct ServeArgs {
    /// Control socket path.
    pub socket: PathBuf,
    /// Ledger path.
    pub ledger: PathBuf,
    /// Work root.
    pub work: PathBuf,
}

/// Why `serve` stopped.
#[derive(Debug, thiserror::Error)]
pub enum ServeError {
    /// The ledger failed.
    #[error("ledger: {0}")]
    Store(#[from] hee4_core::StoreError),
    /// Reconcile left findings; the engine does not listen.
    #[error("recovery incomplete: {0} finding(s)")]
    Recovery(usize),
    /// The socket could not be bound.
    #[error("socket: {0}")]
    Bind(#[from] socket::BindError),
    /// IO.
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}

/// Open, reconcile, start the dispatcher, then listen. Does not return while serving.
///
/// # Errors
/// [`ServeError`].
pub fn serve(args: &ServeArgs, cfg: &dispatcher::Config) -> Result<(), ServeError> {
    if let Some(dir) = args.ledger.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::create_dir_all(&args.work)?;
    let store = Store::open(&args.ledger)?;
    let report = reconcile(&store, &probe_workers(&store, &args.work)?)?;
    for row in &report.rows {
        eprintln!(
            "recovery task={} rule={:?} {} -> {}",
            row.task_id,
            row.rule,
            row.before.as_str(),
            row.after.as_str()
        );
    }
    if !report.complete {
        return Err(ServeError::Recovery(report.findings.len()));
    }
    let engine = Arc::new(actions::Engine::new(
        store,
        args.ledger.clone(),
        args.work.clone(),
        // The door root is the control socket's dir: the runtime dir in production, short.
        args.socket
            .parent()
            .map_or_else(|| PathBuf::from("/"), std::path::Path::to_path_buf),
        cfg.clone(),
    ));
    let worker = Arc::clone(&engine);
    let cfg = cfg.clone();
    std::thread::spawn(move || {
        loop {
            match dispatcher::step(&worker, &cfg) {
                Ok(Some(_)) => {}
                Ok(None) => std::thread::sleep(Duration::from_millis(100)),
                Err(e) => {
                    eprintln!("dispatch error: {e}");
                    std::thread::sleep(Duration::from_secs(1));
                }
            }
        }
    });
    let listener = socket::bind(&args.socket)?;
    eprintln!(
        "hee4 {VERSION} {} serving socket={} peer_check={:?} recovery_applied={}",
        head12(),
        args.socket.display(),
        socket::PEER_CHECK,
        report.applied
    );
    socket::serve(&listener, &engine);
    Ok(())
}
