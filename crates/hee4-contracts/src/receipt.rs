//! Receipt v1 (I4): a decision and what it observed, sealed together and hash-chained.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::hex::Sha256Hex;
use crate::ids::{ObservationId, ReceiptId, TaskId};
use crate::verdict::Verdict;

/// The decision a receipt records. Plain data; K4 `decide` owns the policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Decision {
    /// The verdict.
    pub verdict: Verdict,
}

/// What [`Receipt::seal`] seals: the decision and the observations it was made over, in one
/// value, so neither can be attached or replaced after the seal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReceiptBody {
    /// The receipt's id.
    pub id: ReceiptId,
    /// The task decided.
    pub task_id: TaskId,
    /// The decision.
    pub decision: Decision,
    /// The observations the decision read.
    pub observed: Vec<ObservationId>,
}

/// A sealed receipt. Fields are read-only; a deserialized receipt is unverified until
/// [`Receipt::verify_chain`] accepts the chain it sits in.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Receipt {
    id: ReceiptId,
    task_id: TaskId,
    decision: Decision,
    observed: Vec<ObservationId>,
    hash_prev: Sha256Hex,
    hash_self: Sha256Hex,
}

/// Which check broke.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BreakCause {
    /// `hash_prev` is not the previous receipt's `hash_self` (or [`Sha256Hex::GENESIS`] first).
    Link,
    /// `hash_self` is not the digest of the receipt's canonical body.
    SelfHash,
}

/// The first receipt at which a chain fails.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("receipt chain breaks at index {index} ({cause:?})")]
pub struct ChainBreak {
    /// Position in the slice.
    pub index: usize,
    /// What failed there.
    pub cause: BreakCause,
}

impl Receipt {
    /// Seal `body` after `prev`: `hash_self = sha256(canonical_json(body + hash_prev))`.
    #[must_use]
    pub fn seal(prev: Sha256Hex, body: ReceiptBody) -> Self {
        let ReceiptBody {
            id,
            task_id,
            decision,
            observed,
        } = body;
        let mut receipt = Self {
            id,
            task_id,
            decision,
            observed,
            hash_prev: prev,
            hash_self: Sha256Hex::GENESIS,
        };
        receipt.hash_self = receipt.digest();
        receipt
    }

    /// Check every link and every seal, first receipt against [`Sha256Hex::GENESIS`].
    ///
    /// # Errors
    /// The first [`ChainBreak`].
    pub fn verify_chain(chain: &[Self]) -> Result<(), ChainBreak> {
        let mut prev = Sha256Hex::GENESIS;
        for (index, receipt) in chain.iter().enumerate() {
            let cause = if receipt.hash_prev != prev {
                Some(BreakCause::Link)
            } else if receipt.digest() != receipt.hash_self {
                Some(BreakCause::SelfHash)
            } else {
                None
            };
            if let Some(cause) = cause {
                return Err(ChainBreak { index, cause });
            }
            prev = receipt.hash_self;
        }
        Ok(())
    }

    fn digest(&self) -> Sha256Hex {
        let mut body = Map::new();
        body.insert("id".into(), Value::String(self.id.to_string()));
        body.insert("task_id".into(), Value::String(self.task_id.to_string()));
        body.insert("decision".into(), decision_value(self.decision));
        body.insert(
            "observed".into(),
            Value::Array(
                self.observed
                    .iter()
                    .map(|o| Value::String(o.to_string()))
                    .collect(),
            ),
        );
        body.insert(
            "hash_prev".into(),
            Value::String(self.hash_prev.to_string()),
        );
        Sha256Hex::digest(canonical_json(&Value::Object(body)).as_bytes())
    }

    /// The receipt id.
    #[must_use]
    pub fn id(&self) -> &ReceiptId {
        &self.id
    }
    /// The task decided.
    #[must_use]
    pub fn task_id(&self) -> &TaskId {
        &self.task_id
    }
    /// The decision.
    #[must_use]
    pub fn decision(&self) -> Decision {
        self.decision
    }
    /// The observations the decision read.
    #[must_use]
    pub fn observed(&self) -> &[ObservationId] {
        &self.observed
    }
    /// The previous receipt's seal.
    #[must_use]
    pub fn hash_prev(&self) -> Sha256Hex {
        self.hash_prev
    }
    /// This receipt's seal.
    #[must_use]
    pub fn hash_self(&self) -> Sha256Hex {
        self.hash_self
    }
}

/// The same JSON shape serde derives for [`Decision`], built by hand so sealing cannot fail.
fn decision_value(decision: Decision) -> Value {
    let verdict = match decision.verdict {
        Verdict::Pass => Value::String("pass".into()),
        Verdict::Fail => Value::String("fail".into()),
        Verdict::Refused(reason) => {
            let mut m = Map::new();
            m.insert("refused".into(), Value::String(reason.as_str().into()));
            Value::Object(m)
        }
    };
    let mut m = Map::new();
    m.insert("verdict".into(), verdict);
    Value::Object(m)
}

/// Canonical JSON: object keys sorted bytewise at every depth, no whitespace. Sorting is done
/// here, not left to `serde_json`'s map type, so a `preserve_order` feature elsewhere in the
/// build cannot change a seal.
#[must_use]
pub fn canonical_json(value: &Value) -> String {
    let mut out = String::new();
    write_canonical(value, &mut out);
    out
}

fn write_canonical(value: &Value, out: &mut String) {
    match value {
        Value::Array(items) => {
            out.push('[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_canonical(item, out);
            }
            out.push(']');
        }
        Value::Object(map) => {
            let mut entries: Vec<_> = map.iter().collect();
            entries.sort_by(|a, b| a.0.cmp(b.0));
            out.push('{');
            for (i, (key, item)) in entries.into_iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                out.push_str(&Value::String(key.clone()).to_string());
                out.push(':');
                write_canonical(item, out);
            }
            out.push('}');
        }
        leaf => out.push_str(&leaf.to_string()),
    }
}
