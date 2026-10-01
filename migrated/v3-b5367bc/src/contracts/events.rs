//! RC03 cursor representation only; subscription authority remains unimplemented.
use super::{ScalarError, Sha256Digest, UuidV4, parse_u64_decimal};
use serde::de::{Error as _, MapAccess, Visitor};

/// A selector, never a grant. Every wire scalar retains its canonical spelling.
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize)]
pub struct EventCursorV1 {
    pub epoch: String,
    pub sequence: String,
    pub filter_sha256: String,
    pub visibility_revision: String,
    pub issued_unix_ms: String,
    pub expires_unix_ms: String,
}

// deserialize_map deliberately rejects positional arrays, unlike derived struct
// decoding. Unknown/duplicate/missing keys and non-string values remain errors.
impl<'de> serde::Deserialize<'de> for EventCursorV1 {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct CursorVisitor;
        impl<'de> Visitor<'de> for CursorVisitor {
            type Value = EventCursorV1;
            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("a six-field event cursor object")
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                const FIELDS: &[&str] = &[
                    "epoch",
                    "sequence",
                    "filter_sha256",
                    "visibility_revision",
                    "issued_unix_ms",
                    "expires_unix_ms",
                ];
                let mut values: [Option<String>; 6] = std::array::from_fn(|_| None);
                while let Some(key) = map.next_key::<String>()? {
                    let index = FIELDS
                        .iter()
                        .position(|field| *field == key)
                        .ok_or_else(|| A::Error::unknown_field(&key, FIELDS))?;
                    if values[index].is_some() {
                        return Err(A::Error::duplicate_field(FIELDS[index]));
                    }
                    values[index] = Some(map.next_value()?);
                }
                let [
                    epoch,
                    sequence,
                    filter_sha256,
                    visibility_revision,
                    issued_unix_ms,
                    expires_unix_ms,
                ] = values;
                Ok(EventCursorV1 {
                    epoch: epoch.ok_or_else(|| A::Error::missing_field("epoch"))?,
                    sequence: sequence.ok_or_else(|| A::Error::missing_field("sequence"))?,
                    filter_sha256: filter_sha256
                        .ok_or_else(|| A::Error::missing_field("filter_sha256"))?,
                    visibility_revision: visibility_revision
                        .ok_or_else(|| A::Error::missing_field("visibility_revision"))?,
                    issued_unix_ms: issued_unix_ms
                        .ok_or_else(|| A::Error::missing_field("issued_unix_ms"))?,
                    expires_unix_ms: expires_unix_ms
                        .ok_or_else(|| A::Error::missing_field("expires_unix_ms"))?,
                })
            }
        }
        deserializer.deserialize_map(CursorVisitor)
    }
}

impl EventCursorV1 {
    /// # Errors
    /// Refuses every noncanonical RC03 scalar. Time/retention policy is the caller's.
    pub fn validate(&self) -> Result<(), ScalarError> {
        UuidV4::parse(&self.epoch)?;
        Sha256Digest::parse(&self.filter_sha256)?;
        for value in [
            &self.sequence,
            &self.visibility_revision,
            &self.issued_unix_ms,
            &self.expires_unix_ms,
        ] {
            parse_u64_decimal(value)?;
        }
        Ok(())
    }
}

/// Internal complete-snapshot result, deliberately not an RC03 replay response.
#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CursorDisposition {
    SnapshotOnly,
    ResyncRequired,
}
