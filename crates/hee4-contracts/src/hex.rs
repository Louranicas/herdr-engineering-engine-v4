//! Hex digests that exist only after a successful parse.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use sha2::{Digest, Sha256};

use crate::refusal::{HexFault, HexKind, Refusal};

const LOWER_HEX: &[u8; 16] = b"0123456789abcdef";

fn check_lower_hex(s: &str, lengths: &[usize], kind: HexKind) -> Result<(), Refusal> {
    if !lengths.contains(&s.len()) {
        return Err(Refusal::MalformedHex {
            kind,
            fault: HexFault::Length { found: s.len() },
        });
    }
    match s.bytes().position(|b| !LOWER_HEX.contains(&b)) {
        Some(at) => Err(Refusal::MalformedHex {
            kind,
            fault: HexFault::NotLowerHex { at },
        }),
        None => Ok(()),
    }
}

/// A SHA-256 digest. Built only by [`Sha256Hex::digest`] or by parsing 64 lowercase hex digits.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Sha256Hex([u8; 32]);

impl Sha256Hex {
    /// The `hash_prev` of the first receipt in a chain: 32 zero bytes.
    pub const GENESIS: Self = Self([0; 32]);

    /// The SHA-256 of `bytes`.
    #[must_use]
    pub fn digest(bytes: &[u8]) -> Self {
        Self(Sha256::digest(bytes).into())
    }

    /// The raw 32 bytes.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Display for Sha256Hex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for b in self.0 {
            write!(f, "{b:02x}")?;
        }
        Ok(())
    }
}

impl fmt::Debug for Sha256Hex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Sha256Hex({self})")
    }
}

impl FromStr for Sha256Hex {
    type Err = Refusal;
    fn from_str(s: &str) -> Result<Self, Refusal> {
        check_lower_hex(s, &[64], HexKind::Sha256)?;
        let mut out = [0u8; 32];
        for (slot, [hi, lo]) in out.iter_mut().zip(s.as_bytes().as_chunks::<2>().0) {
            *slot = (nibble(*hi) << 4) | nibble(*lo);
        }
        Ok(Self(out))
    }
}

/// Value of one already-checked lowercase hex digit.
fn nibble(b: u8) -> u8 {
    if b.is_ascii_digit() {
        b - b'0'
    } else {
        b - b'a' + 10
    }
}

/// A git object id: 40 (SHA-1) or 64 (SHA-256 repositories) lowercase hex digits.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GitSha(String);

impl GitSha {
    /// The hex spelling.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for GitSha {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl fmt::Debug for GitSha {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "GitSha({})", self.0)
    }
}

impl FromStr for GitSha {
    type Err = Refusal;
    fn from_str(s: &str) -> Result<Self, Refusal> {
        check_lower_hex(s, &[40, 64], HexKind::GitSha)?;
        Ok(Self(s.to_owned()))
    }
}

macro_rules! serde_via_str {
    ($t:ty) => {
        impl Serialize for $t {
            fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
                s.collect_str(self)
            }
        }
        impl<'de> Deserialize<'de> for $t {
            fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
                let raw = String::deserialize(d)?;
                raw.parse().map_err(serde::de::Error::custom)
            }
        }
    };
}
pub(crate) use serde_via_str;

serde_via_str!(Sha256Hex);
serde_via_str!(GitSha);
