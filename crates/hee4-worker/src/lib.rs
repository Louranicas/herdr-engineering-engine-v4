//! hee4-worker (K2): route a task to a model by capability floor, plan the candidate's
//! namespace, and drive one attempt against the local model.
//!
//! See `FLOW.md`. No policy about verdicts or state lives here: this crate never builds a
//! `TaskState` or a `Verdict` and never writes the ledger.

mod error;
pub mod namespace;
pub mod native;
pub mod route;

pub use error::WorkerError;
