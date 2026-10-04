//! The I3 observation: what one source saw, sealed to its input.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::bounds::MAX_TEXT_BYTES;
use crate::hex::{GitSha, Sha256Hex, serde_via_str};
use crate::ids::{EvidenceLabel, SourceId, ToolName, ToolVersion};
use crate::refusal::{Refusal, TokenFault, TokenKind};

/// The tool that produced an observation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ToolId {
    /// Name as the tool reports it.
    pub name: ToolName,
    /// Version as the tool reports it.
    pub version: ToolVersion,
}

/// Why an adapter refused to run (its exit 7): 1..=512 bytes, no control characters. Human text
/// for the record only; `decide` never branches on it.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RefusalText(String);

impl RefusalText {
    /// The text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for RefusalText {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl FromStr for RefusalText {
    type Err = Refusal;
    fn from_str(s: &str) -> Result<Self, Refusal> {
        let fault = if s.is_empty() {
            Some(TokenFault::Empty)
        } else if s.len() > MAX_TEXT_BYTES {
            Some(TokenFault::TooLong { found: s.len() })
        } else {
            s.bytes()
                .position(|b| b.is_ascii_control())
                .map(|at| TokenFault::Forbidden { at })
        };
        match fault {
            Some(fault) => Err(Refusal::MalformedToken {
                kind: TokenKind::RefusalText,
                fault,
            }),
            None => Ok(Self(s.to_owned())),
        }
    }
}

serde_via_str!(RefusalText);

/// What the source concluded.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Outcome {
    /// The check passed.
    Pass,
    /// The check failed.
    Fail,
    /// The check could not run to a conclusion.
    Error,
    /// The adapter refused to run (exit 7). `decide` maps this to `Refused(Invalid)`, never Pass.
    Refused {
        /// Why it refused.
        reason: RefusalText,
    },
}

/// One piece of evidence, by content address.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Evidence {
    /// What it is.
    pub label: EvidenceLabel,
    /// Its bytes' digest.
    pub sha256: Sha256Hex,
}

/// One observation (STACK-MAP §2 I3). Every field is a parsed type, so a deserialized value
/// cannot carry a malformed digest, sha or id. Whether a source "looked at nothing" is K4's
/// admission check over `evidence` and `input_sha256`, not this type's.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Observation {
    /// Who observed.
    pub source: SourceId,
    /// SHA-256 of the exact input the source read.
    pub input_sha256: Sha256Hex,
    /// The tool and its version.
    pub tool: ToolId,
    /// The commit observed.
    pub head_sha: GitSha,
    /// The source's conclusion.
    pub outcome: Outcome,
    /// What it saw.
    pub evidence: Vec<Evidence>,
    /// Advisory observations inform but never gate.
    pub advisory: bool,
    /// Wall time taken, in milliseconds.
    pub elapsed_ms: u64,
    /// Wall time allowed, in milliseconds.
    pub budget_ms: u64,
}
