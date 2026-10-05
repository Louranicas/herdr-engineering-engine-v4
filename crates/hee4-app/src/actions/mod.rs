//! The control actions behind one `answer(line)` door. Dispatch order, exactly: `wire::parse`
//! -> `catalogue::find` (else `unknown_action`) -> `action_version` -> `Registry::get(owner)`
//! (a miss is the one `unavailable` site, `because` = the entry's scope) -> the mutating gates
//! (`idempotency_key`, then `not_ready` while the ledger's `recovery_complete` is false) -> the
//! catalogue's precondition rule -> the registered handler. No string match on an action id
//! lives here: the ids are the catalogue's and the `Family` tables'.
//!
//! Every state change goes through `Store::admit` or `Store::apply`.

pub mod page;
pub mod registry;
pub mod roster;
pub mod service;
pub mod task;
pub mod tools;

use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use hee4_contracts::TaskId;
use hee4_contracts::catalogue::{self, PreconditionRule};
use hee4_core::{Store, StoreError};
use serde_json::Value;

use crate::dispatcher::Config;
use crate::wire::{self, Code, Fault, Request};

pub use registry::{Family, Handler, Registry, RegistryFault, StartFault, StartHook};

/// The families this release serves: one line per family, the single registration point.
///
/// # Errors
/// [`RegistryFault`]: a compose-time defect (held owner, duplicate owner, unknown id, owner
/// mismatch).
pub fn composed() -> Result<Registry, RegistryFault> {
    Registry::new(&[
        task::HEALTH,
        task::FAMILY,
        task::EVENTS,
        tools::FAMILY,
        roster::FAMILY,
        service::FAMILY,
    ])
}

/// Unix nanoseconds now, saturating. K1-store-foundation's `Store::boot()` replaces the source of
/// this value in a later wave (one line here; `page.rs` unchanged).
fn unix_nanos() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| u64::try_from(d.as_nanos()).unwrap_or(u64::MAX))
}

/// The engine one `serve` process holds: the ledger (one connection, one process), paths, and
/// the registry.
#[derive(Debug)]
pub struct Engine {
    store: Mutex<Store>,
    ledger: PathBuf,
    work: PathBuf,
    doors: PathBuf,
    cfg: Config,
    started: Instant,
    principal: String,
    registry: Registry,
    boot: u64,
}

impl Engine {
    /// Wrap an opened store. `ledger` is its file (read-only stream readers open it),
    /// `work` the work root (`<work>/<task_id>` per task), `doors` where each attempt serves its
    /// model door (`<doors>/<task_id>.model.sock`; the control socket's dir, so it stays short),
    /// `cfg` what `task.preview` routes by. The registry is [`composed`] here.
    ///
    /// # Errors
    /// [`RegistryFault`] from [`composed`]: a compose-time defect.
    pub fn new(
        store: Store,
        ledger: PathBuf,
        work: PathBuf,
        doors: PathBuf,
        cfg: Config,
    ) -> Result<Self, RegistryFault> {
        Ok(Self {
            store: Mutex::new(store),
            ledger,
            work,
            doors,
            cfg,
            started: Instant::now(),
            principal: crate::process_uid()
                .map_or_else(|| "uid:unknown".into(), |u| format!("uid:{u}")),
            registry: composed()?,
            boot: unix_nanos(),
        })
    }

    /// The ledger, locked for one call. A poisoned lock is still the same ledger: every write
    /// in it is its own committed transaction.
    pub fn store(&self) -> MutexGuard<'_, Store> {
        self.store
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// The ledger file.
    #[must_use]
    pub fn ledger(&self) -> &Path {
        &self.ledger
    }

    /// The work root.
    #[must_use]
    pub fn work(&self) -> &Path {
        &self.work
    }

    /// The door root: where attempts serve their model door sockets.
    #[must_use]
    pub fn doors(&self) -> &Path {
        &self.doors
    }

    /// Where a task's admitted brief text lives (the ledger has no brief column; DC proposal).
    #[must_use]
    pub fn brief_path(&self, task: &TaskId) -> PathBuf {
        self.work.join("briefs").join(format!("{task}.brief"))
    }

    /// The composed registry.
    #[must_use]
    pub fn registry(&self) -> &Registry {
        &self.registry
    }

    /// The boot epoch page cursors are pinned to: unix nanoseconds drawn once in [`Engine::new`].
    #[must_use]
    pub fn boot(&self) -> u64 {
        self.boot
    }
}

/// What one request line asks of the connection.
#[derive(Debug, Clone, PartialEq)]
pub enum Outcome {
    /// Write this frame and read the next request.
    Frame(Value),
    /// Write `ack`, then stream ledger events after `since_seq` until close.
    Subscribe {
        /// The result frame acknowledging the subscription.
        ack: Value,
        /// Events with `seq > since_seq` are streamed.
        since_seq: i64,
    },
}

/// Answer one request line.
#[must_use]
pub fn answer(engine: &Engine, line: &str) -> Outcome {
    match wire::parse(line) {
        Err((id, fault)) => Outcome::Frame(wire::error(&id, &fault)),
        Ok(req) => match dispatch(engine, &req) {
            Ok(Answer::Frame(replayed, body)) => {
                Outcome::Frame(wire::result(&req.request_id, replayed, body))
            }
            Ok(Answer::Subscribe(ack, since_seq)) => Outcome::Subscribe {
                ack: wire::result(&req.request_id, false, ack),
                since_seq,
            },
            Err(fault) => Outcome::Frame(wire::error(&req.request_id, &fault)),
        },
    }
}

/// Answer one request line with one frame (a subscription's frame is its ack).
#[must_use]
pub fn handle(engine: &Engine, line: &str) -> Value {
    match answer(engine, line) {
        Outcome::Frame(v) | Outcome::Subscribe { ack: v, .. } => v,
    }
}

/// A one-frame handler's reply: `(replayed, body)`.
pub(crate) type Reply = Result<(bool, Value), Fault>;

/// What a handler returns to dispatch.
#[derive(Debug, Clone, PartialEq)]
pub enum Answer {
    /// One result frame: `(replayed, body)`.
    Frame(bool, Value),
    /// Turn the connection into the event stream after `since_seq`; the handler built the ack body.
    Subscribe(Value, i64),
}

fn dispatch(engine: &Engine, req: &Request) -> Result<Answer, Fault> {
    dispatch_with(engine, engine.registry(), req)
}

/// Dispatch `req` through `registry` (the test seam; `answer` uses the engine's own registry).
///
/// # Errors
/// `unknown_action` (not catalogued), `unsupported_action_version`, `unavailable` (owner not
/// registered: the one site; `Registry::serve` misses exactly then, since `Registry::new`
/// refused any family missing one of its owner's ids), `invalid_argument` (missing key or
/// precondition), `not_ready`, or the handler's own refusal.
pub fn dispatch_with(engine: &Engine, registry: &Registry, req: &Request) -> Result<Answer, Fault> {
    let Some(entry) = catalogue::find(&req.action) else {
        return Err(Fault::new(
            Code::UnknownAction,
            "/action",
            "not in the catalogue",
        ));
    };
    if req.action_version != entry.version {
        return Err(Fault::new(
            Code::UnsupportedActionVersion,
            "/action_version",
            "only 1",
        ));
    }
    let Some(handler) = registry.serve(entry) else {
        return Err(Fault::new(
            Code::Unavailable,
            "/action",
            "owner not registered in this release",
        )
        .with_because(entry.scope.because()));
    };
    if entry.effect.mutates() {
        if req.idempotency_key.is_none() {
            return Err(Fault::new(
                Code::InvalidArgument,
                "/idempotency_key",
                "required for a mutating action",
            ));
        }
        if !engine
            .store()
            .recovery_complete()
            .map_err(|e| internal(&e))?
        {
            return Err(Fault::new(
                Code::NotReady,
                "/action",
                "startup reconcile has not completed",
            ));
        }
    }
    if let PreconditionRule::Required(resource) = entry.precondition
        && req.precondition.is_none()
    {
        return Err(Fault::new(
            Code::InvalidArgument,
            "/precondition",
            format!("required: {{resource: \"{resource}\", id, generation}}"),
        ));
    }
    handler(engine, req)
}

pub(crate) fn internal(e: &StoreError) -> Fault {
    Fault::new(Code::Internal, "/", e.to_string())
}

/// Test fixtures shared by the family modules: one home for the engine helper and the brief.
#[cfg(test)]
pub(crate) mod testing {
    use super::{Config, Engine, PathBuf, Store};

    /// An eleven-field brief whose VERIFY is `/usr/bin/test -d /usr` (real, silent, admitted).
    pub(crate) const BRIEF: &str = "GOAL: g\nSCOPE: s\nCONTEXT: c\nACCEPTANCE: a\nVERIFY: /usr/bin/test -d /usr\nTIMEBOX: 10s\nFORBIDDEN: f\nREPORT: r\nSTANDING: s\nRECON: r\nRESTATEMENT: run test in the namespace\n";

    /// A fresh engine over a store under `OUT_DIR/actions-<name>`.
    pub(crate) fn engine(name: &str) -> Result<Engine, Box<dyn std::error::Error>> {
        let dir = PathBuf::from(env!("OUT_DIR")).join(format!("actions-{name}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir)?;
        Ok(Engine::new(
            Store::open(&dir.join("ledger.sqlite3"))?,
            dir.join("ledger.sqlite3"),
            dir.join("work"),
            dir.join("rt"),
            Config {
                model: "m:1".into(),
                live: false,
            },
        )?)
    }
}
