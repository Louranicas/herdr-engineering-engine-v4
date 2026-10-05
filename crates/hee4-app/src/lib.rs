//! `hee4-app` (K6): the `hee4` binary's parts. Startup order, the control socket, the actions
//! (catalogue + owner registry), the synchronous dispatcher and `doctor`. See `FLOW.md`.
//!
//! Startup: the budgets file (`--budgets`/`HEE4_BUDGETS`, else the default) →
//! `Store::open` → `probe::observe(open_attempts)` → `recovery::reconcile` → `Engine::new` (composes the registry) →
//! the families' `on_serve_start` → the DC-22 start/upgrade backup (`--backups`) → dispatcher thread → bind → serve. A
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
use hee4_host::clock::{Clock as _, SystemClock};

/// The build commit (`git rev-parse HEAD` at build time, or `unknown`).
pub const HEAD: &str = env!("HEE4_HEAD");

/// The engine version.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// The standards file the verdict is judged by, pinned at build time.
pub const GATE_TOML: &[u8] = include_bytes!("../../../gate.toml");

/// Budget fields `serve` loads, validates and `health` echoes, but nothing in this binary reads
/// yet: `health` lists them under `budgets_inert` so the reply never reports them as applied.
/// `attempt.ctx_tokens`: the roster default lives in `actions/roster.rs`;
/// `ledger.busy_timeout_ms`: K1's `Store::open` takes no budget from the app yet.
/// `ledger.checkpoint_every` is read by the dispatcher (`checkpoint_if_due` after each receipt)
/// but stays listed until the health test in `actions/task.rs`, which pins this list and is not
/// this slice's file, drops it (FLOW.md Gaps).
pub const INERT_BUDGETS: [&str; 3] = [
    "attempt.ctx_tokens",
    "ledger.busy_timeout_ms",
    "ledger.checkpoint_every",
];

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
    /// The DC-22 backup root (`--backups DIR`); `None` takes no backups.
    pub backups: Option<PathBuf>,
    /// `HEE4_REQUIRE_BACKUPS=1`: refuse to serve without `backups`.
    pub require_backups: bool,
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
    /// The ledger holds more open attempts than recovery will adopt
    /// (`recovery.open_attempt_limit`); the engine does not probe, reconcile or listen.
    #[error(
        "budgets: recovery.open_attempt_limit={limit} but the ledger holds {open} open attempts"
    )]
    OpenAttempts {
        /// Open attempts in the ledger.
        open: usize,
        /// `recovery.open_attempt_limit`.
        limit: u64,
    },
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
    /// `HEE4_REQUIRE_BACKUPS=1` and no `--backups`; the engine does not open the ledger.
    #[error("backups: --backups is required (HEE4_REQUIRE_BACKUPS=1)")]
    BackupsRequired,
    /// The start/upgrade backup failed; the store does not go into service.
    #[error("backups: {0}")]
    Backup(#[from] dispatcher::BackupFault),
}

/// Recovery adopts at most `limit` (`recovery.open_attempt_limit`) open attempts; more is a
/// refusal naming the field, before any probe.
///
/// # Errors
/// [`ServeError::OpenAttempts`] when `open > limit`.
fn adoptable(open: usize, limit: u64) -> Result<(), ServeError> {
    if u64::try_from(open).unwrap_or(u64::MAX) > limit {
        return Err(ServeError::OpenAttempts { open, limit });
    }
    Ok(())
}

/// The DC-22 start/upgrade backup into `root`: the store goes into service only after it.
/// The trigger is what [`dispatcher::backup_due`] names, else `Start` (a restart inside the
/// freshness window still backs up). No thread or timer (V4-6).
///
/// # Errors
/// [`dispatcher::BackupFault`]: the head is unknown, the root or log refused, or K1 refused.
fn start_backup(
    root: &std::path::Path,
    engine: &actions::Engine,
) -> Result<dispatcher::Backups, dispatcher::BackupFault> {
    let head = HEAD
        .parse::<hee4_contracts::GitSha>()
        .ok()
        .as_ref()
        .and_then(dispatcher::Head12::of)
        .ok_or(dispatcher::BackupFault::HeadUnknown)?;
    let mut backups = dispatcher::Backups::open(root, head)?;
    let clock = SystemClock;
    let trigger = dispatcher::backup_due(&backups.facts(clock.now()))
        .unwrap_or(dispatcher::BackupTrigger::Start);
    backups.take(engine, trigger, &clock)?;
    Ok(backups)
}

/// Open, reconcile, start the dispatcher, then listen. Does not return while serving.
///
/// # Errors
/// [`ServeError`].
pub fn serve(args: &ServeArgs, cfg: &dispatcher::Config) -> Result<(), ServeError> {
    if args.require_backups && args.backups.is_none() {
        return Err(ServeError::BackupsRequired);
    }
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
    adoptable(open.len(), budgets.recovery.open_attempt_limit)?;
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
    let mut backups = args
        .backups
        .as_deref()
        .map(|root| start_backup(root, &engine))
        .transpose()?;
    let worker = Arc::clone(&engine);
    let cfg = cfg.clone();
    std::thread::spawn(move || {
        loop {
            match dispatcher::step(&worker, &cfg, backups.as_mut()) {
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
        "hee4 {VERSION} {} serving socket={} peer_check={:?} recovery_applied={} catalogue={} budgets={budgets_from} backups={}",
        head12(),
        args.socket.display(),
        socket::PEER_CHECK,
        report.applied,
        catalogue12(),
        args.backups
            .as_deref()
            .map_or_else(|| "none".into(), |p| p.display().to_string())
    );
    socket::serve(&listener, &engine);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_attempts_over_the_limit_are_refused_by_field_name() {
        assert!(adoptable(0, 1).is_ok());
        assert!(adoptable(64, 64).is_ok());
        let refused = adoptable(65, 64).map_err(|e| e.to_string());
        assert_eq!(
            refused,
            Err(
                "budgets: recovery.open_attempt_limit=64 but the ledger holds 65 open attempts"
                    .to_owned()
            )
        );
    }
}
