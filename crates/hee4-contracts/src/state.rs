//! `TaskState` and the only function that derives one: [`transition`].
//!
//! The whitelist is the State and Transition Map §2c (50 legal pairs plus `∅ + Admit`), with the
//! brief's renames (`Begin` → `Dispatch`, `Verdict` → `Decide`) and two additions: `Observe`
//! (`verifying` self-edge) and `Recover(R07 | R08)` (the R-table's `Settle{unsettled}`).

use serde::Serialize;

use crate::refusal::Refusal;
use crate::verdict::{Reason, Verdict};

/// The observable phase of a task. Anyone may build a `Phase` to match on or compare with;
/// only [`transition`] turns one into a [`TaskState`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case", tag = "state")]
pub enum Phase {
    /// Admitted, no attempt yet.
    Admitted,
    /// An attempt is running.
    Running,
    /// The attempt settled ready; observations and the verdict are pending.
    Verifying,
    /// The verdict failed; a new attempt may begin.
    RepairPending,
    /// Cancellation requested of a non-waiting task.
    CancellationRequested,
    /// Quarantined. `cancel` records a cancel requested while waiting (UM-P3, one source).
    Blocked {
        /// Cancellation was requested.
        cancel: bool,
    },
    /// The external effect is unknown. `cancel` as for `Blocked`.
    EffectUnknown {
        /// Cancellation was requested.
        cancel: bool,
    },
    /// Terminal: accepted.
    Accepted,
    /// Terminal: failed.
    Failed,
    /// Terminal: cancelled.
    Cancelled,
    /// Terminal: abandoned.
    Abandoned,
}

impl Phase {
    /// `accepted`, `failed`, `cancelled` or `abandoned`.
    #[must_use]
    pub const fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Accepted | Self::Failed | Self::Cancelled | Self::Abandoned
        )
    }

    /// Cancellation was requested, in the state or in the waiting variant's field.
    #[must_use]
    pub const fn cancel_requested(self) -> bool {
        matches!(
            self,
            Self::CancellationRequested
                | Self::Blocked { cancel: true }
                | Self::EffectUnknown { cancel: true }
        )
    }

    /// The durable column spelling (T-02 `tasks.state`).
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Admitted => "admitted",
            Self::Running => "running",
            Self::Verifying => "verifying",
            Self::RepairPending => "repair_pending",
            Self::CancellationRequested => "cancellation_requested",
            Self::Blocked { .. } => "blocked",
            Self::EffectUnknown { .. } => "effect_unknown",
            Self::Accepted => "accepted",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
            Self::Abandoned => "abandoned",
        }
    }
}

/// A task's state. Opaque: the only values that exist are ones [`transition`] returned.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(transparent)]
pub struct TaskState(Phase);

impl TaskState {
    /// The phase, for matching.
    #[must_use]
    pub const fn phase(self) -> Phase {
        self.0
    }

    /// Fold `events` through [`transition`] from no task. This is `transition` repeated, not a
    /// second constructor: a ledger rehydrates a state by replaying its events.
    ///
    /// # Errors
    /// The first [`Refusal`] any step returns.
    pub fn replay<I: IntoIterator<Item = Event>>(events: I) -> Result<Self, Refusal> {
        let mut state = None;
        for event in events {
            state = Some(transition(state, event)?);
        }
        state.ok_or(Refusal::NotAdmitted {
            event: Event::Admit,
        })
    }
}

/// How an attempt settled.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Settlement {
    /// Settled, ready to verify.
    Ready,
    /// Settled, not ready: repair.
    NotReady,
    /// Did not settle: the effect is unknown.
    Unsettled,
}

/// An operator (or later, recovery) resolution of a stuck task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Resolution {
    /// Move to `blocked`, carrying any cancel request.
    Quarantine,
    /// Close: `cancelled` if cancellation was requested, else `abandoned`.
    Abandon,
}

/// The startup reconcile rules (gates/features/crash-restart.md, table R01–R14).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[allow(missing_docs)] // each name is the R-table row's name
pub enum RecoveryRule {
    R01StaleObservationEpoch,
    R02StaleGeneration,
    R03CommitOrdering,
    R04AcceptanceStands,
    R05CancellationStands,
    R06LiveOwnedChild,
    R07ProcessNotOurs,
    R08WorkerAbsent,
    R09WorkspaceReuse,
    R10EffectAmbiguity,
    R11CleanupReadback,
    R12VerificationBoundary,
    R13CursorEpoch,
    R14UnexpectedState,
}

impl RecoveryRule {
    /// R01..R14 in order.
    pub const ALL: [Self; 14] = [
        Self::R01StaleObservationEpoch,
        Self::R02StaleGeneration,
        Self::R03CommitOrdering,
        Self::R04AcceptanceStands,
        Self::R05CancellationStands,
        Self::R06LiveOwnedChild,
        Self::R07ProcessNotOurs,
        Self::R08WorkerAbsent,
        Self::R09WorkspaceReuse,
        Self::R10EffectAmbiguity,
        Self::R11CleanupReadback,
        Self::R12VerificationBoundary,
        Self::R13CursorEpoch,
        Self::R14UnexpectedState,
    ];
}

/// Everything that can happen to a task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Event {
    /// Create the task (`task.submit`).
    Admit,
    /// Begin an attempt (State map `Begin`).
    Dispatch,
    /// An observation was ledgered while verifying.
    Observe,
    /// The attempt settled.
    Settle(Settlement),
    /// K4 `decide` issued a verdict (State map `Verdict`).
    Decide(Verdict),
    /// Accept a passed task.
    Accept,
    /// Request cancellation.
    Cancel,
    /// Resolve a task (`task.resolve`).
    Resolve(Resolution),
    /// Close through the stop door.
    Stop,
    /// A startup reconcile rule fired.
    Recover(RecoveryRule),
}

/// The one way to derive a [`TaskState`]: `None` is "no task yet".
///
/// # Errors
/// A named [`Refusal`] for every pair outside the whitelist.
pub fn transition(state: Option<TaskState>, event: Event) -> Result<TaskState, Refusal> {
    use Event as E;
    use Phase as P;
    let Some(TaskState(from)) = state else {
        return match event {
            E::Admit => Ok(TaskState(P::Admitted)),
            _ => Err(Refusal::NotAdmitted { event }),
        };
    };
    if from.is_terminal() {
        return Err(Refusal::Terminal { from, event });
    }
    let cancel = from.cancel_requested();
    let to = match (from, event) {
        (_, E::Admit) => return Err(Refusal::AlreadyAdmitted { from }),
        (P::Admitted | P::RepairPending, E::Dispatch) => P::Running,
        (P::Running, E::Settle(Settlement::Ready))
        | (P::Verifying, E::Observe | E::Decide(Verdict::Pass)) => P::Verifying,
        (P::Running, E::Settle(Settlement::NotReady))
        | (P::Verifying, E::Decide(Verdict::Fail)) => P::RepairPending,
        (
            P::Running | P::CancellationRequested,
            E::Settle(Settlement::Unsettled)
            | E::Recover(RecoveryRule::R07ProcessNotOurs | RecoveryRule::R08WorkerAbsent),
        )
        | (
            P::Verifying | P::CancellationRequested,
            E::Decide(Verdict::Refused(Reason::Unreconciled)),
        ) => P::EffectUnknown { cancel },
        (P::CancellationRequested, E::Settle(_) | E::Decide(_))
        | (
            P::Admitted | P::Running | P::Verifying | P::RepairPending | P::CancellationRequested,
            E::Cancel,
        ) => P::CancellationRequested,
        (P::Blocked { .. }, E::Settle(_)) => from,
        (
            P::Verifying,
            E::Decide(Verdict::Refused(Reason::Invalid | Reason::Error | Reason::Timeout)),
        )
        | (P::Admitted | P::RepairPending | P::Verifying, E::Stop) => P::Failed,
        (P::Verifying, E::Accept) => P::Accepted,
        (P::Blocked { .. }, E::Cancel) => P::Blocked { cancel: true },
        (P::EffectUnknown { .. }, E::Cancel) => P::EffectUnknown { cancel: true },
        (_, E::Resolve(Resolution::Quarantine)) => P::Blocked { cancel },
        (_, E::Resolve(Resolution::Abandon)) | (P::CancellationRequested, E::Stop) if cancel => {
            P::Cancelled
        }
        (_, E::Resolve(Resolution::Abandon)) => P::Abandoned,
        (_, E::Recover(rule)) => return Err(Refusal::NoTaskEdge { from, rule }),
        _ => return Err(Refusal::Illegal { from, event }),
    };
    Ok(TaskState(to))
}
