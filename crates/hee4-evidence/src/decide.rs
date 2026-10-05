//! `decide`: the one verdict authority. A fail-closed severity lattice over sealed observations.

use hee4_contracts::{
    Decision, GitSha, Observation, ObservationId, Reason, Receipt, ReceiptBody, ReceiptId, Refusal,
    Sha256Hex, SourceId, TaskId, Verdict, canonical_json,
};

/// A wired identity source, pinned by the digest of what it is (a binary, a lockfile, a
/// standards text).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Source {
    /// Which source.
    pub name: SourceId,
    /// Digest of the bytes that identify it.
    pub digest: Sha256Hex,
}

/// Why an identity source is not wired.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Why {
    /// Nobody wired it (v3's hard-wired `None`, AP-22).
    NotWired,
    /// It was configured but its bytes could not be read.
    Unreadable {
        /// What could not be read.
        what: String,
    },
}

/// One identity: wired to a pinned source, or unavailable for a stated reason.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Identity {
    /// Pinned.
    Wired(Source),
    /// Absent; `decide` refuses.
    Unavailable(Why),
}

impl Identity {
    const fn is_wired(&self) -> bool {
        matches!(self, Self::Wired(_))
    }
}

/// The three identity sources of `modules/hee4-evidence/check` §2.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Identities {
    /// Who collected the observations.
    pub collector: Identity,
    /// The dependency locks the candidate was built against.
    pub locks: Identity,
    /// The standards the candidate is judged by.
    pub standards: Identity,
}

/// What is being decided: one task's candidate, bound to its commit and its input bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Subject {
    /// The task decided.
    pub task_id: TaskId,
    /// The commit every observation must have observed.
    pub head_sha: GitSha,
    /// The digest of the input every observation must have read.
    pub input_sha256: Sha256Hex,
}

/// Total order on verdicts: `Refused > Fail > Pass`; among refusals, the one that sends the
/// task furthest from acceptance wins.
const fn severity(v: Verdict) -> u8 {
    match v {
        Verdict::Pass => 0,
        Verdict::Fail => 1,
        Verdict::Refused(Reason::Invalid) => 2,
        Verdict::Refused(Reason::Timeout) => 3,
        Verdict::Refused(Reason::Error) => 4,
        Verdict::Refused(Reason::Cancelled) => 5,
        Verdict::Refused(Reason::Unreconciled) => 6,
    }
}

const fn worse(a: Verdict, b: Verdict) -> Verdict {
    if severity(b) > severity(a) { b } else { a }
}

/// One observation's contribution to the lattice; `None` means it cannot move the verdict.
fn contribution(o: &Observation, subject: &Subject) -> Option<Verdict> {
    if o.head_sha != subject.head_sha || o.input_sha256 != subject.input_sha256 {
        return Some(Verdict::Refused(Reason::Unreconciled));
    }
    if o.advisory {
        return None;
    }
    Some(if o.evidence.is_empty() {
        Verdict::Refused(Reason::Invalid)
    } else if o.elapsed_ms > o.budget_ms {
        Verdict::Refused(Reason::Timeout)
    } else {
        match o.outcome {
            hee4_contracts::Outcome::Pass => Verdict::Pass,
            hee4_contracts::Outcome::Fail => Verdict::Fail,
            hee4_contracts::Outcome::Refused { .. } => Verdict::Refused(Reason::Invalid),
            // `Error`, and any conclusion a later `#[non_exhaustive]` variant adds: refuse.
            _ => Verdict::Refused(Reason::Error),
        }
    })
}

/// The verdict over `obs` for `subject`. Nothing else produces a verdict.
///
/// - any identity `Unavailable` → `Refused(Error)`, before the lattice runs (AP-22);
/// - no tier-0 observation → `Refused(Invalid)` (a gate that looked at nothing);
/// - otherwise the worst contribution, starting from `Pass`. See `FLOW.md` for the table.
#[must_use]
pub fn decide(ids: &Identities, obs: &[Observation], subject: &Subject) -> Decision {
    let wired = [&ids.collector, &ids.locks, &ids.standards]
        .iter()
        .all(|i| i.is_wired());
    let verdict = if wired {
        let floor = if obs.iter().any(|o| !o.advisory) {
            Verdict::Pass
        } else {
            Verdict::Refused(Reason::Invalid)
        };
        obs.iter()
            .filter_map(|o| contribution(o, subject))
            .fold(floor, worse)
    } else {
        Verdict::Refused(Reason::Error)
    };
    Decision { verdict }
}

/// Why a decision could not be sealed.
#[derive(Debug, thiserror::Error)]
pub enum SealError {
    /// An observation did not serialize.
    #[error("observation did not serialize: {0}")]
    Serialize(#[from] serde_json::Error),
    /// A derived id did not parse.
    #[error("derived observation id refused: {0}")]
    Id(#[from] Refusal),
}

/// The content address of an observation for one task: `obs-` + SHA-256 of the task id, a
/// newline, and the observation's canonical JSON. The ledger binds an observation row to its
/// task, so two tasks with byte-identical observations (same head, same input digest,
/// `/usr/bin/true` in 0 ms: the feature drive run twice on one ledger) must not share an id.
///
/// # Errors
/// [`SealError`] if the observation does not serialize or the id does not parse.
pub fn observation_id(task: &TaskId, o: &Observation) -> Result<ObservationId, SealError> {
    let mut bytes = task.as_str().as_bytes().to_vec();
    bytes.push(b'\n');
    bytes.extend_from_slice(canonical_json(&serde_json::to_value(o)?).as_bytes());
    let digest = Sha256Hex::digest(&bytes);
    Ok(format!("obs-{digest}").parse()?)
}

/// Decide over `obs` and seal the decision with exactly the observations read (sorted,
/// deduplicated content addresses) after `prev`, in one record.
///
/// This is the only sealing path in this crate, and `tests/one_sealer.rs` (the census) fails if
/// `Receipt::seal(` or `ReceiptBody {` appears in any `crates/*/src` file other than this one and
/// `hee4-contracts/src/receipt.rs`.
///
/// # Errors
/// [`SealError`] if an observation id cannot be derived; nothing is sealed then.
pub fn decide_and_seal(
    prev: Sha256Hex,
    id: ReceiptId,
    ids: &Identities,
    obs: &[Observation],
    subject: &Subject,
) -> Result<Receipt, SealError> {
    let mut observed = obs
        .iter()
        .map(|o| observation_id(&subject.task_id, o))
        .collect::<Result<Vec<_>, _>>()?;
    observed.sort_by(|a, b| a.as_str().cmp(b.as_str()));
    observed.dedup();
    Ok(Receipt::seal(
        prev,
        ReceiptBody {
            id,
            task_id: subject.task_id.clone(),
            decision: decide(ids, obs, subject),
            observed,
        },
    ))
}
