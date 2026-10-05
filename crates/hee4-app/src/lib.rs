//! `hee4-app` (K6): the `hee4` binary's parts. Startup order, the control socket, the actions
//! (catalogue + owner registry), the synchronous dispatcher and `doctor`. See `FLOW.md`.
//!
//! Startup: the budgets file (`--budgets`/`HEE4_BUDGETS`, else the default) →
//! `Store::open` → `probe::observe(open_attempts)` → `recovery::reconcile` → `Engine::new` (composes the registry) →
//! the families' `on_serve_start` → dispatcher thread → bind → serve. A
//! mutating action is refused `not_ready` while the ledger's `recovery_complete` is false.

pub mod actions;
pub mod dispatcher;
pub mod doctor;
pub mod service_runner;
pub mod socket;
pub mod stream;
pub mod wire;

use std::os::unix::fs::MetadataExt as _;
use std::path::PathBuf;
use std::sync::Arc;

use hee4_contracts::{BudgetParseError, Budgets};
use hee4_core::{Store, probe, reconcile};

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

/// The first 12 digits of the catalogue revision (`hee4_contracts::catalogue::revision`).
#[must_use]
pub fn catalogue12() -> String {
    let revision = hee4_contracts::catalogue::revision().to_string();
    revision.get(..12).unwrap_or(&revision).to_owned()
}

/// This process's uid, read from `/proc/self` (no `libc`, no `unsafe`).
#[must_use]
pub fn process_uid() -> Option<u32> {
    std::fs::metadata("/proc/self").ok().map(|m| m.uid())
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
    /// The budgets file (`--budgets P`, else `HEE4_BUDGETS`); `None` is [`Budgets::DEFAULT`].
    pub budgets: Option<PathBuf>,
}

/// Why `serve` stopped.
#[derive(Debug, thiserror::Error)]
pub enum ServeError {
    /// The budgets file could not be read; the engine does not open the ledger.
    #[error("budgets: {path}: {source}")]
    BudgetsFile {
        /// The file named by `--budgets` or `HEE4_BUDGETS`.
        path: PathBuf,
        /// The read error.
        source: std::io::Error,
    },
    /// The budgets file did not parse or a field failed validation (the text names the
    /// field, e.g. `budgets: door.max_body_bytes is zero`); the engine does not open the ledger.
    #[error("{0}")]
    Budgets(#[from] BudgetParseError),
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
    /// The family set did not compose (a held owner, a duplicate, an unknown id, a mismatch).
    #[error("compose: {0}")]
    Compose(#[from] actions::RegistryFault),
    /// A family's startup hook refused; the engine does not listen.
    #[error("start: {0}")]
    Start(#[from] actions::StartFault),
}

/// Open, reconcile, start the dispatcher, then listen. Does not return while serving.
///
/// # Errors
/// [`ServeError`].
pub fn serve(args: &ServeArgs, cfg: &dispatcher::Config) -> Result<(), ServeError> {
    let budgets = match &args.budgets {
        None => Budgets::DEFAULT,
        Some(path) => Budgets::parse(&std::fs::read_to_string(path).map_err(|source| {
            ServeError::BudgetsFile {
                path: path.clone(),
                source,
            }
        })?)?,
    };
    let budgets_from = args
        .budgets
        .as_ref()
        .map_or_else(|| "default".to_owned(), |p| format!("file:{}", p.display()));
    if let Some(dir) = args.ledger.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::create_dir_all(&args.work)?;
    let store = Store::open(&args.ledger)?;
    let open = store.open_attempts()?;
    let observed = probe::observe(&open, budgets.recovery.workspace_readback_bytes);
    for row in &open {
        eprintln!(
            "recovery probe attempt={} custody={:?} workspace={:?}",
            row.id,
            observed
                .process
                .get(&row.task_id)
                .copied()
                .unwrap_or_default(),
            observed.workspace.get(&row.id).copied().unwrap_or_default()
        );
    }
    let report = reconcile(&store, &observed)?;
    for row in &report.rows {
        eprintln!(
            "recovery task={} rule={} reason={} workspace={} {} -> {}",
            row.task_id,
            row.rule
                .map_or_else(|| "none".to_owned(), |r| format!("{r:?}")),
            row.reason
                .map_or_else(|| "none".to_owned(), |r| format!("{r:?}")),
            row.workspace
                .map_or_else(|| "none".to_owned(), |w| format!("{w:?}")),
            row.before.as_str(),
            row.after.as_str()
        );
    }
    if !report.complete {
        return Err(ServeError::Recovery(report.findings.len()));
    }
    let engine = Arc::new(
        actions::Engine::new(
            store,
            args.ledger.clone(),
            args.work.clone(),
            // The door root is the control socket's dir: the runtime dir in production, short.
            args.socket
                .parent()
                .map_or_else(|| PathBuf::from("/"), std::path::Path::to_path_buf),
            cfg.clone(),
        )?
        .with_budgets(budgets),
    );
    engine.registry().on_serve_start(&engine)?;
    let worker = Arc::clone(&engine);
    let cfg = cfg.clone();
    std::thread::spawn(move || {
        loop {
            match dispatcher::step(&worker, &cfg) {
                Ok(Some(_)) => {}
                Ok(None) => std::thread::sleep(budgets.dispatcher.idle()),
                Err(e) => {
                    eprintln!("dispatch error: {e}");
                    std::thread::sleep(budgets.dispatcher.error_backoff());
                }
            }
        }
    });
    let listener = socket::bind(&args.socket)?;
    eprintln!(
        "hee4 {VERSION} {} serving socket={} peer_check={:?} recovery_applied={} catalogue={} budgets={budgets_from}",
        head12(),
        args.socket.display(),
        socket::PEER_CHECK,
        report.applied,
        catalogue12()
    );
    socket::serve(&listener, &engine);
    Ok(())
}
