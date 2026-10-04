//! `hee4-contracts`: the rung-1 types the HEE v4 skeleton compiles against (K0).
//!
//! A value of each type here is already legal: a [`TaskState`] came out of [`transition`], a
//! [`Sha256Hex`] parsed as 64 lowercase hex digits, a [`Brief`] has all eleven fields, a
//! [`Receipt`] was sealed over its decision and observations together. See `FLOW.md`.
//!
//! A `TaskState` cannot be built from a `Phase` outside [`transition`]:
//! ```compile_fail,E0423
//! let _ = hee4_contracts::TaskState(hee4_contracts::Phase::Accepted);
//! ```
//! nor a `Sha256Hex` from an arbitrary string:
//! ```compile_fail,E0423
//! let _ = hee4_contracts::Sha256Hex(String::from("not hex"));
//! ```

pub mod bounds;
mod brief;
mod hex;
mod ids;
mod observation;
mod receipt;
mod refusal;
mod state;
mod verdict;

pub use brief::{Brief, BriefField};
pub use hex::{GitSha, Sha256Hex};
pub use ids::{EvidenceLabel, ObservationId, ReceiptId, SourceId, TaskId, ToolName, ToolVersion};
pub use observation::{Evidence, Observation, Outcome, ToolId};
pub use receipt::{BreakCause, ChainBreak, Decision, Receipt, ReceiptBody, canonical_json};
pub use refusal::{HexFault, HexKind, Refusal, TokenFault, TokenKind};
pub use state::{Event, Phase, RecoveryRule, Resolution, Settlement, TaskState, transition};
pub use verdict::{Reason, Verdict};
