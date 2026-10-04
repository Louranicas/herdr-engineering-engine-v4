//! The crate's one error type for the attempt driver.

use hee4_contracts::Refusal;
use hee4_host::spawn::{HostRefusal, SpawnError};

/// Why an attempt or a namespace request could not proceed.
#[derive(Debug, thiserror::Error)]
pub enum WorkerError {
    /// The brief or a derived token was refused by the contracts crate.
    #[error("contract refusal: {0}")]
    Contract(#[from] Refusal),
    /// The spawn door refused the plan before any process started.
    #[error("host refusal: {0}")]
    Host(#[from] HostRefusal),
    /// A process could not be started or waited on.
    #[error("spawn failed: {0}")]
    Spawn(#[from] SpawnError),
    /// The work root cannot be the writable bind (relative, or inside a read-only system path).
    #[error("work root unusable: {0}")]
    WorkRoot(String),
}
