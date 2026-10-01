//! Bounded exact-byte parsing and serialization; no graph resolver or publisher.
use super::{Error, Id, MAX_BYTES, Name, ReceiptRecord, Record, Ref, Sha, TypedRef};
use sha2::{Digest, Sha256};
use std::io::Write;
const HEX: &[u8; 16] = b"0123456789abcdef";

fn framing(bytes: &[u8]) -> Result<(), Error> {
    if bytes.len() > MAX_BYTES {
        return Err(Error::Bound);
    }
    let text = std::str::from_utf8(bytes).map_err(|_| Error::Json)?;
    if text.is_empty() || bytes.starts_with(&[0xef, 0xbb, 0xbf]) {
        return Err(Error::Json);
    }
    let mut quoted = false;
    let mut escaped = false;
    for &byte in bytes {
        if quoted {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                quoted = false;
            }
        } else if byte == b'"' {
            quoted = true;
        } else if byte.is_ascii_whitespace() {
            return Err(Error::Json);
        }
    }
    Ok(())
}
/// Decode one compact closed JSON record. Key ordering and valid escapes may vary.
/// # Errors
/// Refuses invalid/oversized UTF-8, noncompact framing, duplicate/unknown fields,
/// invalid primitive spellings, missing required nulls and local contradictions.
pub fn decode<T: ReceiptRecord>(bytes: &[u8]) -> Result<T, Error> {
    framing(bytes)?;
    let value: T = serde_json::from_slice(bytes).map_err(|_| Error::Json)?;
    value.validate()?;
    Ok(value)
}
/// Decode by one exact fixed schema ID; raw bytes are not JSON records.
/// # Errors
/// Refuses an unknown address or any strict typed decoding failure.
pub fn decode_record(schema_id: &str, bytes: &[u8]) -> Result<Record, Error> {
    super::records::decode_named(schema_id, bytes)
}
struct BoundedBytes(Vec<u8>);
impl Write for BoundedBytes {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if bytes.len() > MAX_BYTES - self.0.len() {
            return Err(std::io::Error::other("receipt byte bound exceeded"));
        }
        self.0.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
/// Validate, then serialize one compact record without BOM or trailing LF.
/// # Errors
/// Refuses primitive/record contradictions or an encoded object over 1MiB.
pub fn encode<T: ReceiptRecord>(record: &T) -> Result<Vec<u8>, Error> {
    record.validate()?;
    let mut output = BoundedBytes(Vec::new());
    serde_json::to_writer(&mut output, record).map_err(|_| Error::Bound)?;
    Ok(output.0)
}

/// The one media type a receipt cites a record under.
pub const RECORD_MEDIA_TYPE: &str = "application/json";

/// Validate and hash the supplied exact bytes; never reserialize before hashing.
/// This returns descriptive metadata and does not publish or establish custody.
/// # Errors
/// Refuses invalid records or metadata; no partial reference is returned.
pub fn reference_for<T: ReceiptRecord>(
    artifact_id: Id,
    bytes: &[u8],
) -> Result<TypedRef<T>, Error> {
    decode::<T>(bytes)?;
    let hash = Sha256::digest(bytes);
    let mut spelling = String::from("sha256:");
    for byte in hash {
        spelling.push(char::from(HEX[usize::from(byte >> 4)]));
        spelling.push(char::from(HEX[usize::from(byte & 15)]));
    }
    TypedRef::new(Ref {
        artifact_id,
        sha256: Sha::new(spelling)?,
        byte_length: u32::try_from(bytes.len()).map_err(|_| Error::Bound)?,
        media_type: Name::new(RECORD_MEDIA_TYPE)?,
        schema_id: Name::new(T::SCHEMA_ID)?,
    })
}
