//! `hee4-contracts`: the rung-1 types the HEE v4 skeleton compiles against (K0).
//!
//! The types here are built through their checked constructors: a [`TaskState`] came out of
//! [`transition`], a [`Sha256Hex`] parsed as 64 lowercase hex digits, a [`Brief`] has all eleven
//! fields, a [`Receipt`] was sealed over its decision and observations together. A [`Budgets`]
//! is legal when it came from [`Budgets::parse`] or is [`Budgets::DEFAULT`], its only legal
//! constructors; its section fields stay writable (`pub`)
//! pending a DC row, so a literal or a later write is not checked. See `FLOW.md`.
//!
//! A `TaskState` cannot be built from a `Phase` outside [`transition`]:
//! ```compile_fail,E0423
//! let _ = hee4_contracts::TaskState(hee4_contracts::Phase::Accepted);
//! ```
//! nor a `Sha256Hex` from an arbitrary string:
//! ```compile_fail,E0423
//! let _ = hee4_contracts::Sha256Hex(String::from("not hex"));
//! ```
//! nor a budgets section read on its own, past [`Budgets::parse`]'s checks:
//! ```compile_fail,E0277
//! let _: hee4_contracts::DoorBudget = serde_json::from_str(r#"{"pool":0}"#).unwrap();
//! ```
//! nor a whole `Budgets` read through serde, which over a `serde_json::Value` would take a key
//! named twice last-wins:
//! ```compile_fail,E0277
//! let _: hee4_contracts::Budgets = serde_json::from_value(serde_json::json!({})).unwrap();
//! ```

pub mod bounds;
mod brief;
mod budgets;
pub mod catalogue;
mod hex;
mod ids;
mod observation;
mod receipt;
mod refusal;
mod state;
mod verdict;
mod verify;

pub use brief::{Brief, BriefField};
pub use budgets::{
    AttemptBudget, BudgetParseError, BudgetRefusal, Budgets, DispatcherBudget, DoorBudget,
    LedgerBudget, ModelBudget, RecoveryBudget, SocketBudget, StreamBudget,
};
pub use hex::{GitSha, Sha256Hex};
pub use ids::{EvidenceLabel, ObservationId, ReceiptId, SourceId, TaskId, ToolName, ToolVersion};
pub use observation::{Evidence, Observation, Outcome, RefusalText, ToolId};
pub use receipt::{
    BreakCause, ChainBreak, Decision, Receipt, ReceiptBody, canonical_json, merkle_root,
};
pub use refusal::{HexFault, HexKind, Refusal, TokenFault, TokenKind};
pub use state::{
    AbandonReason, Event, Phase, QuarantineReason, RecoveryRule, Resolution, Settlement, TaskState,
    transition,
};
pub use verdict::{Reason, Verdict};
pub use verify::{VerifyFault, VerifyLine};
