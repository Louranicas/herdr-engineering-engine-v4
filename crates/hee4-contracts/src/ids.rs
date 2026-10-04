//! Identifier newtypes. Each exists only after its token parse succeeds.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::bounds::MAX_TOKEN_BYTES;
use crate::hex::serde_via_str;
use crate::refusal::{Refusal, TokenFault, TokenKind};

fn check_token(s: &str, kind: TokenKind) -> Result<(), Refusal> {
    let fault = if s.is_empty() {
        Some(TokenFault::Empty)
    } else if s.len() > MAX_TOKEN_BYTES {
        Some(TokenFault::TooLong { found: s.len() })
    } else {
        s.bytes()
            .position(|b| b.is_ascii_whitespace() || b.is_ascii_control())
            .map(|at| TokenFault::Forbidden { at })
    };
    fault.map_or(Ok(()), |fault| Err(Refusal::MalformedToken { kind, fault }))
}

macro_rules! token {
    ($(#[$doc:meta])* $name:ident) => {
        $(#[$doc])*
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(String);

        impl $name {
            /// The token text.
            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl FromStr for $name {
            type Err = Refusal;
            fn from_str(s: &str) -> Result<Self, Refusal> {
                check_token(s, TokenKind::$name)?;
                Ok(Self(s.to_owned()))
            }
        }

        serde_via_str!($name);
    };
}

token!(
    /// A task's id.
    TaskId
);
token!(
    /// A receipt's id.
    ReceiptId
);
token!(
    /// A ledgered observation's id.
    ObservationId
);
token!(
    /// The source that produced an observation (a gate, a tool run, a plant).
    SourceId
);
token!(
    /// A tool's name, as the tool reports it.
    ToolName
);
token!(
    /// A tool's version, as the tool reports it.
    ToolVersion
);
token!(
    /// A label naming one piece of evidence.
    EvidenceLabel
);
