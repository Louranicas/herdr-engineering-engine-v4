//! One bounded execute–verify–repair policy over the existing effect owners.
//! The application implements these ports by composing Store, worker and check;
//! it does not maintain another task or retry state machine.

use super::{LoopGuard, LoopRefusal};
use crate::contracts::Generation;
use std::time::Duration;

/// Execution cannot report acceptance, even when the worker exits successfully.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Work {
    ReadyForCheck,
    Failed,
    Unsettled,
}

/// Only the independently collected current subject supplies criterion bits.
/// Historical progress is never combined into a current passing candidate.
#[derive(Debug)]
pub enum Checked<Evidence> {
    Passed { evidence: Evidence, criteria: u64 },
    Failed { criteria: u64 },
    Invalid,
    Error,
    Timeout,
    Unsettled,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StopReason {
    Policy(LoopRefusal),
    WorkerFailed,
    InvalidCheck,
    VerifierError,
    VerifierTimeout,
    Cancelled,
    Unsettled,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Outcome {
    Accepted,
    Stopped(StopReason),
    NeedsSettlement(StopReason),
}

/// What an acceptance commit answered. A cancellation the store committed first is not an error:
/// the store names it (`store::Error::Cancelled`, checked before the compare-and-set) and the
/// driver stops the task as `Cancelled` (B14a-R1.3).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Acceptance {
    Accepted,
    CancelledFirst,
}

/// Effect ports are implemented only by the trusted application composition.
/// Every method retains actual bytes/measurements or returns an explicit error.
/// No method interprets worker-provided success text as a verification result.
pub trait Runtime {
    type Error;
    type Attempt;
    type Evidence;

    /// Elapsed time from one immutable monotonic task origin, including repairs.
    fn elapsed(&self) -> Duration;
    /// Read durable intent immediately before each effect boundary.
    /// # Errors
    /// Returns a persistence/readback error rather than assuming no cancellation.
    fn cancellation_requested(&mut self) -> Result<bool, Self::Error>;
    /// Reserve verification, pin the recipe and workspace, then durably begin.
    /// `ordinal` is the policy generation, not a substitute for Store CAS.
    /// # Errors
    /// Refuses stale identity, missing authority or insufficient conservative budget.
    fn begin(&mut self, ordinal: Generation) -> Result<Self::Attempt, Self::Error>;
    /// Execute in the owned workspace and persist measured cost/worker settlement.
    /// # Errors
    /// Unknown effect or commit failure cannot be retried as a clean failure.
    fn execute(&mut self, attempt: &Self::Attempt) -> Result<Work, Self::Error>;
    /// Freeze and check this attempt; retain each receipt and charge settled cost,
    /// including failed checks, before returning its independently observed result.
    /// # Errors
    /// Propagates custody, storage or collector transport errors without acceptance.
    fn verify(&mut self, attempt: &Self::Attempt) -> Result<Checked<Self::Evidence>, Self::Error>;
    /// Recheck current identity/cancellation and atomically commit exact evidence,
    /// criterion manifest, acceptance and completion outbox through Store.
    /// # Errors
    /// Refuses stale/nonpassing proof, lost evidence and commit failure. A cancellation committed
    /// first is answered `Ok(Acceptance::CancelledFirst)`, never as an error.
    fn accept(
        &mut self,
        attempt: &Self::Attempt,
        evidence: Self::Evidence,
    ) -> Result<Acceptance, Self::Error>;
    /// Request a durable stop. Return true only after measured process/workspace/
    /// effect settlement permits a truthful terminal state. False retains pending
    /// obligations and reservations; a cancellation request alone returns false.
    /// # Errors
    /// Propagates persistence or ownership failures without claiming a terminal state.
    fn stop(&mut self, reason: StopReason) -> Result<bool, Self::Error>;
}

#[derive(Debug)]
pub enum Error<E> {
    Policy(LoopRefusal),
    Runtime(E),
}

/// Drive the original RC01 policy to accepted, truthful stop or retained obligation.
/// This is a single invocation; restart continuation belongs to recovery. Runtime
/// methods must enforce their own deadlines and Store compare-and-swap checks.
///
/// # Errors
/// Refuses invalid criteria or propagates any unresolved runtime/commit failure.
pub fn run<R: Runtime>(runtime: &mut R, criteria_count: u8) -> Result<Outcome, Error<R::Error>> {
    let initial = "1"
        .parse()
        .map_err(|_| Error::Policy(LoopRefusal::GenerationExhausted))?;
    let mut guard = LoopGuard::new(criteria_count, initial).map_err(Error::Policy)?;
    let required = if criteria_count == 64 {
        u64::MAX
    } else {
        (1_u64 << criteria_count) - 1
    };
    loop {
        if runtime.cancellation_requested().map_err(Error::Runtime)? {
            return stop(runtime, StopReason::Cancelled);
        }
        let ordinal = match guard.begin_attempt(runtime.elapsed()) {
            Ok(value) => value,
            Err(reason) => return stop(runtime, StopReason::Policy(reason)),
        };
        let attempt = runtime.begin(ordinal).map_err(Error::Runtime)?;
        match runtime.execute(&attempt).map_err(Error::Runtime)? {
            Work::ReadyForCheck => {}
            Work::Failed => return stop(runtime, StopReason::WorkerFailed),
            Work::Unsettled => return stop(runtime, StopReason::Unsettled),
        }
        if runtime.cancellation_requested().map_err(Error::Runtime)? {
            return stop(runtime, StopReason::Cancelled);
        }
        match runtime.verify(&attempt).map_err(Error::Runtime)? {
            Checked::Passed { evidence, criteria } => {
                // A subset from this subject cannot borrow historical satisfied bits.
                if criteria != required {
                    return stop(runtime, StopReason::InvalidCheck);
                }
                guard
                    .record_reconciled_attempt(ordinal, criteria, runtime.elapsed())
                    .map_err(Error::Policy)?;
                if runtime.elapsed() >= super::TASK_LIMIT {
                    return stop(runtime, StopReason::Policy(LoopRefusal::Deadline));
                }
                if runtime.cancellation_requested().map_err(Error::Runtime)? {
                    return stop(runtime, StopReason::Cancelled);
                }
                return match runtime.accept(&attempt, evidence).map_err(Error::Runtime)? {
                    Acceptance::Accepted => Ok(Outcome::Accepted),
                    Acceptance::CancelledFirst => stop(runtime, StopReason::Cancelled),
                };
            }
            Checked::Failed { criteria } => {
                guard
                    .record_reconciled_attempt(ordinal, criteria, runtime.elapsed())
                    .map_err(Error::Policy)?;
            }
            Checked::Invalid => return stop(runtime, StopReason::InvalidCheck),
            Checked::Error => return stop(runtime, StopReason::VerifierError),
            Checked::Timeout => return stop(runtime, StopReason::VerifierTimeout),
            Checked::Unsettled => return stop(runtime, StopReason::Unsettled),
        }
    }
}

fn stop<R: Runtime>(runtime: &mut R, reason: StopReason) -> Result<Outcome, Error<R::Error>> {
    Ok(if runtime.stop(reason).map_err(Error::Runtime)? {
        Outcome::Stopped(reason)
    } else {
        Outcome::NeedsSettlement(reason)
    })
}
