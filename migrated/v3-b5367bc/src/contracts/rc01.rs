//! RC01 numeric limits for `rust-library-change/1` (docs/contract-decisions.md, RC01 "Attempts and
//! loop" and "No-progress stop"). The task loop guard owns the policy and re-exports these; they are
//! defined here because every consumer may import `contracts`, while `store` may not import `task`.
use std::time::Duration;

/// Total monotonic elapsed per task, shared by every retry.
pub const TASK_LIMIT: Duration = Duration::from_mins(20);
/// Final share of [`TASK_LIMIT`] kept for verification and cleanup; no new candidate work starts in it.
pub const CLEANUP_RESERVE: Duration = Duration::from_mins(5);
/// How long after a stop intent a run may still be observed before its cleanup is late (RC04's
/// ten-second cutoff), in milliseconds. One spelling for three doors: `check::decision`'s
/// `CleanupLate` rule, the process owner's stop budget (`worker::process`) and the runtime's check
/// teardown share (`app::runtime::CHECK_TEARDOWN`) — R15 round 2, MEDIUM-9.
pub const CLEANUP_GRACE_MS: u64 = 10_000;
/// Attempts per task.
pub const MAX_ATTEMPTS: u8 = 3;
/// Consecutive attempts without a newly satisfied criterion before the task stops.
pub const MAX_NO_PROGRESS: u8 = 2;
/// Input tokens one future model invocation may be sent ("32,768 input tokens and 4,096 output
/// tokens per future model invocation; reject if the adapter cannot establish the bound",
/// docs/contract-decisions.md, RC01). Route reads it as the context a recipe must hold (B07 R1.7).
pub const MAX_INPUT_TOKENS: u64 = 32_768;
