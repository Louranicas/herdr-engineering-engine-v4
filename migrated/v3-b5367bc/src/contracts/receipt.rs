//! Closed RC04 revision-4 receipt values and exact-byte JSON codec.
//!
//! This boundary validates records; it does not resolve evidence, authenticate
//! producers, grant authority, execute work or admit modules.

mod codec;
mod primitives;
mod records;

pub use codec::{RECORD_MEDIA_TYPE, decode, decode_record, encode, reference_for};
pub use primitives::*;
pub use records::*;

/// Maximum finalized JSON bytes for one record or page.
pub const MAX_BYTES: usize = 1_048_576;
/// Maximum elements in each inline array.
pub const MAX_ITEMS: usize = 256;

/// Bounded static diagnostics; untrusted input is never echoed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    /// Invalid strict JSON syntax or a closed type mismatch.
    Json,
    /// An input, string or array exceeds its bound.
    Bound,
    /// A primitive value violates its fixed grammar.
    Scalar,
    /// A record's fields contradict its local contract.
    Invariant,
    /// A reference or dispatch names an unsupported target schema.
    Schema,
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Json => "invalid closed receipt JSON",
            Self::Bound => "receipt bound exceeded",
            Self::Scalar => "invalid receipt scalar",
            Self::Invariant => "receipt fields violate a local invariant",
            Self::Schema => "unsupported receipt reference target",
        })
    }
}
impl std::error::Error for Error {}

/// Recursively check primitive bounds and local record invariants.
pub trait Validate {
    /// # Errors
    /// Returns a static refusal for any invalid primitive or record.
    fn validate(&self) -> Result<(), Error>;
}
/// Fixed local addressing, separate from reference resolution.
pub trait Address {
    /// Exact schema identifier for this concrete type.
    const SCHEMA_ID: &'static str;
}
mod sealed {
    pub trait Sealed {}
}
/// A concrete addressable JSON record from the single frozen vocabulary.
pub trait ReceiptRecord:
    Address + Validate + serde::Serialize + serde::de::DeserializeOwned + sealed::Sealed
{
}

fn require(ok: bool) -> Result<(), Error> {
    if ok { Ok(()) } else { Err(Error::Invariant) }
}
