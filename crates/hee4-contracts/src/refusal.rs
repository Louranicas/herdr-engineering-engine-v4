//! Every refusal this crate can return, by name. Never a string.

use crate::brief::BriefField;
use crate::state::{Event, Phase, RecoveryRule};
use crate::verify::VerifyFault;

/// Which hex newtype a malformed input was meant to become.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HexKind {
    /// A [`crate::Sha256Hex`]: 64 lowercase hex digits.
    Sha256,
    /// A [`crate::GitSha`]: 40 or 64 lowercase hex digits.
    GitSha,
}

/// What was wrong with a hex input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HexFault {
    /// The input had this many bytes; the kind admits a different length.
    Length {
        /// Bytes found.
        found: usize,
    },
    /// The byte at this offset is not one of `0-9a-f`.
    NotLowerHex {
        /// Byte offset.
        at: usize,
    },
}

/// Which identifier newtype a malformed token was meant to become.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TokenKind {
    /// [`crate::TaskId`].
    TaskId,
    /// [`crate::ReceiptId`].
    ReceiptId,
    /// [`crate::ObservationId`].
    ObservationId,
    /// [`crate::SourceId`].
    SourceId,
    /// [`crate::ToolName`].
    ToolName,
    /// [`crate::ToolVersion`].
    ToolVersion,
    /// [`crate::EvidenceLabel`].
    EvidenceLabel,
    /// [`crate::RefusalText`].
    RefusalText,
}

/// What was wrong with a token.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TokenFault {
    /// Zero bytes.
    Empty,
    /// Longer than [`crate::bounds::MAX_TOKEN_BYTES`].
    TooLong {
        /// Bytes found.
        found: usize,
    },
    /// The byte at this offset is a control character (or, for identifiers, whitespace).
    Forbidden {
        /// Byte offset.
        at: usize,
    },
}

/// A named refusal. Callers match on the variant; the text is for humans only.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum Refusal {
    /// A task that does not exist yet can only be admitted.
    #[error("no task yet: only Admit creates one, got {event:?}")]
    NotAdmitted {
        /// The refused event.
        event: Event,
    },
    /// Admission is creation; a repeated submit is a replay, never a transition (State map I-08).
    #[error("task already exists in {from:?}: Admit is creation only")]
    AlreadyAdmitted {
        /// The existing state.
        from: Phase,
    },
    /// A terminal state takes no event (State map I-01).
    #[error("{from:?} is terminal: {event:?} refused")]
    Terminal {
        /// The terminal state.
        from: Phase,
        /// The refused event.
        event: Event,
    },
    /// The pair is not in the whitelist.
    #[error("illegal transition: {event:?} from {from:?}")]
    Illegal {
        /// The state the event was applied to.
        from: Phase,
        /// The refused event.
        event: Event,
    },
    /// The recovery rule writes no task-side event from this state (crash-restart R-table).
    #[error("recovery rule {rule:?} has no task edge from {from:?}")]
    NoTaskEdge {
        /// The state the rule was applied to.
        from: Phase,
        /// The rule.
        rule: RecoveryRule,
    },
    /// A hex newtype refused its input.
    #[error("malformed {kind:?}: {fault:?}")]
    MalformedHex {
        /// The intended newtype.
        kind: HexKind,
        /// What was wrong.
        fault: HexFault,
    },
    /// An identifier newtype refused its input.
    #[error("malformed {kind:?}: {fault:?}")]
    MalformedToken {
        /// The intended newtype.
        kind: TokenKind,
        /// What was wrong.
        fault: TokenFault,
    },
    /// A brief lacks a required field.
    #[error("brief is missing {}", field.label())]
    MissingBriefField {
        /// The missing field.
        field: BriefField,
    },
    /// A brief names a field twice.
    #[error("brief names {} twice", field.label())]
    DuplicateBriefField {
        /// The repeated field.
        field: BriefField,
    },
    /// The worker's RESTATEMENT is empty; admission refuses it (narrative principle 13).
    #[error("RESTATEMENT is empty")]
    EmptyRestatement,
    /// The brief's VERIFY looks at nothing: empty, nothing runnable, or only no-ops (V4-94).
    #[error("VERIFY {cause}")]
    VacuousVerify {
        /// What makes it vacuous.
        cause: VerifyFault,
    },
}
