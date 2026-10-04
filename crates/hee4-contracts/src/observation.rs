//! The I3 observation: what one source saw, sealed to its input.

use serde::{Deserialize, Serialize};

use crate::hex::{GitSha, Sha256Hex};
use crate::ids::{EvidenceLabel, SourceId, ToolName, ToolVersion};

/// The tool that produced an observation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ToolId {
    /// Name as the tool reports it.
    pub name: ToolName,
    /// Version as the tool reports it.
    pub version: ToolVersion,
}

/// What the source concluded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    /// The check passed.
    Pass,
    /// The check failed.
    Fail,
    /// The check could not run to a conclusion.
    Error,
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
