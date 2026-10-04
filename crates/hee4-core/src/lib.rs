//! `hee4-core` (K1): the SQLite ledger that is the single home of task state, and the startup
//! reconcile that runs to completion before any dispatch. See `FLOW.md`.
//!
//! - [`Store::apply`] is the only state writer: replay, `hee4_contracts::transition`, append,
//!   fsync. No other function writes `events` or `tasks`.
//! - [`recovery::reconcile`] applies R01–R14 through `Store::apply` and sets
//!   `recovery_complete`; `Dispatch` is refused until it is true.

mod codec;
pub mod recovery;
mod store;

pub use recovery::{Observations, ProcessCustody, RecoveryReport, reconcile};
pub use store::{Admission, CacheHeal, CachedRow, OperationKey, SCHEMA_VERSION, Store, StoreError};
