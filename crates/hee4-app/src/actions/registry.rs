//! The owner registry (K6): which catalogue owners this release serves. Built once from the
//! families `composed()` names; `Registry::new` refuses a held owner (`Judge`, `Deploy`), a
//! duplicate owner, a handler id the catalogue does not carry, and a handler whose family owner
//! is not the catalogue's owner for that id. After it, dispatch looks ids up through
//! `catalogue::find` and owners through [`Registry::get`]; the miss is the one `unavailable` site.

use hee4_contracts::catalogue::{self, Owner};

use super::{Answer, Engine};
use crate::wire::{Fault, Request};

/// One action's handler: the engine and the parsed request in, a frame or a refusal out.
pub type Handler = fn(&Engine, &Request) -> Result<Answer, Fault>;

/// A family's startup hook, run by `serve` after the engine is built and before it listens.
pub type StartHook = fn(&Engine) -> Result<(), StartFault>;

/// Why a family's startup hook refused.
#[derive(Debug, thiserror::Error)]
pub enum StartFault {
    /// The hook could not read or write what it needs.
    #[error("{owner} family start: {source}")]
    Io {
        /// The family.
        owner: Owner,
        /// The failure.
        #[source]
        source: std::io::Error,
    },
    /// The hook refused by name.
    #[error("{owner} family start refused: {reason}")]
    Refused {
        /// The family.
        owner: Owner,
        /// The reason.
        reason: &'static str,
    },
}

/// One owner's handlers: the registration unit. Wave-2 families add one line to `composed()`.
#[derive(Debug, Clone, Copy)]
pub struct Family {
    /// The catalogue owner every handler id must carry.
    pub owner: Owner,
    /// `(id, handler)` pairs; each id must be catalogued under `owner`.
    pub handlers: &'static [(&'static str, Handler)],
    /// Run once at `serve` start, in registration order; the first `Err` stops the start.
    pub on_serve_start: Option<StartHook>,
}

/// Why `Registry::new` refused a family set (a compose-time defect, never a wire refusal).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RegistryFault {
    /// `Judge` (held, H-8) or `Deploy` (internal) may not be registered.
    #[error("owner {0} is held: no family may serve it")]
    HeldOwner(Owner),
    /// Two families name the same owner.
    #[error("owner {0} registered twice")]
    DuplicateOwner(Owner),
    /// A handler id the catalogue does not carry.
    #[error("handler id {0} is not in the catalogue")]
    UnknownId(String),
    /// A handler id catalogued under another owner than its family's.
    #[error("handler {id}: family owner {family}, catalogue owner {catalogue}")]
    OwnerMismatch {
        /// The handler id.
        id: String,
        /// The family that carries it.
        family: Owner,
        /// The catalogue's owner for it.
        catalogue: Owner,
    },
}

/// The composed families, checked against the catalogue.
#[derive(Debug)]
pub struct Registry {
    families: Vec<Family>,
}

impl Registry {
    /// Check and keep `families`, in order.
    ///
    /// # Errors
    /// [`RegistryFault`]: the first held owner, duplicate owner, unknown id or owner mismatch.
    pub fn new(families: &[Family]) -> Result<Self, RegistryFault> {
        let mut kept: Vec<Family> = Vec::with_capacity(families.len());
        for family in families {
            if family.owner.is_held() {
                return Err(RegistryFault::HeldOwner(family.owner));
            }
            if kept.iter().any(|f| f.owner == family.owner) {
                return Err(RegistryFault::DuplicateOwner(family.owner));
            }
            for (id, _) in family.handlers {
                let entry = catalogue::find(id)
                    .ok_or_else(|| RegistryFault::UnknownId((*id).to_owned()))?;
                if entry.owner != family.owner {
                    return Err(RegistryFault::OwnerMismatch {
                        id: (*id).to_owned(),
                        family: family.owner,
                        catalogue: entry.owner,
                    });
                }
            }
            kept.push(*family);
        }
        Ok(Self { families: kept })
    }

    /// The family registered for `owner`, if any. `None` is the `unavailable` signal.
    #[must_use]
    pub fn get(&self, owner: Owner) -> Option<&Family> {
        self.families.iter().find(|f| f.owner == owner)
    }

    /// The handler registered for `id`, if any.
    #[must_use]
    pub fn handler(&self, id: &str) -> Option<Handler> {
        self.families
            .iter()
            .flat_map(|f| f.handlers.iter())
            .find(|(name, _)| *name == id)
            .map(|(_, handler)| *handler)
    }

    /// Every family, in registration order.
    #[must_use]
    pub fn families(&self) -> &[Family] {
        &self.families
    }

    /// Run every family's `on_serve_start` in registration order; stop at the first `Err`.
    ///
    /// # Errors
    /// The first hook's [`StartFault`].
    pub fn on_serve_start(&self, engine: &Engine) -> Result<(), StartFault> {
        self.families
            .iter()
            .filter_map(|f| f.on_serve_start)
            .try_for_each(|hook| hook(engine))
    }
}
