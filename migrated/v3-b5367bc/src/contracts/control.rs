//! HEE3-Control/1 on the wire (`docs/contract-decisions.md` RC03 §§2–4): frame acquisition, strict
//! JSON admission, the exact-byte request digest, the common request envelope, and the typed
//! result and error records a receiver writes back.
//!
//! What this module decides is only what the wire decides. Which actions exist, what each one
//! requires of the envelope and who may invoke it belong to `actions`; the principal belongs to
//! the transport that authenticated the peer. Three properties are structural here:
//!
//! * **A limit is a limit at the point of acquisition.** [`FrameReader`] never holds more than
//!   [`MAX_FRAME_BYTES`] + 1 unterminated bytes, and refuses before any JSON is parsed.
//! * **The digest names the admitted bytes.** [`request_sha256`] hashes the payload before it is
//!   parsed into a lossy value; there is deliberately no canonicalization (RC03 §3).
//! * **Diagnostics never echo input.** Every [`Fault`] field is `&'static str`, so no refusal can
//!   carry attacker-chosen text back to a caller or into a log.

use super::{Generation, Sha256Digest, UuidV4, parse_u64_decimal};
use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::io::{self, Read};

/// The largest frame payload, excluding its terminal LF (RC03 §3).
pub const MAX_FRAME_BYTES: usize = 1_048_576;
/// The deepest permitted nesting; the top-level object is depth 1.
pub const MAX_DEPTH: usize = 32;
/// How far ahead of the receiver's clock a request deadline may lie (RC03 §6).
pub const MAX_DEADLINE_AHEAD_MS: u64 = 60_000;
/// The protocol name every native control record carries.
pub const PROTOCOL: &str = "hee3.control";
/// The only protocol version this receiver speaks.
pub const PROTOCOL_VERSION: u64 = 1;

const READ_CHUNK: usize = 8 * 1024;
const BYTE_ORDER_MARK: &[u8] = b"\xef\xbb\xbf";

/// Why a frame was refused without a reply. RC03 §3: on any of these the receiver does not
/// dispatch and closes the connection; a correlated error needs safely parsed metadata.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FrameFault {
    /// The stream ended inside a record.
    Truncated,
    /// The payload exceeded [`MAX_FRAME_BYTES`].
    Oversize,
    /// The payload is not UTF-8.
    InvalidUtf8,
    /// The payload starts with a byte-order mark.
    ByteOrderMark,
    /// A blank record.
    Empty,
    /// Whitespace outside a string: pretty-printing, padding or a CR before the LF.
    Whitespace,
    /// The record is JSON but not an object.
    NotObject,
    /// An object repeats a member name.
    DuplicateName,
    /// Nesting deeper than [`MAX_DEPTH`].
    TooDeep,
    /// A number that is not a non-negative integer token (`-1`, `1.0`, `1e0`, `01`).
    NumberToken,
    /// An unescaped control character inside a string.
    ControlCharacter,
    /// A `\u` escape naming half of a surrogate pair.
    UnpairedSurrogate,
    /// Anything else that is not one JSON value.
    Syntax,
    /// Well-formed, but without a request identity to correlate a reply with.
    Uncorrelated,
}

impl FrameFault {
    /// The stable diagnostic name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Truncated => "stream ended inside a record",
            Self::Oversize => "frame exceeds 1048576 bytes",
            Self::InvalidUtf8 => "frame is not UTF-8",
            Self::ByteOrderMark => "frame starts with a byte-order mark",
            Self::Empty => "blank record",
            Self::Whitespace => "whitespace outside a string",
            Self::NotObject => "record is not a JSON object",
            Self::DuplicateName => "duplicate member name",
            Self::TooDeep => "nesting exceeds 32",
            Self::NumberToken => "number is not a non-negative integer token",
            Self::ControlCharacter => "unescaped control character in a string",
            Self::UnpairedSurrogate => "unpaired surrogate escape",
            Self::Syntax => "not one JSON value",
            Self::Uncorrelated => "no valid request_id to correlate a reply with",
        }
    }
}

/// Why [`FrameReader::next_frame`] produced no frame.
#[derive(Debug)]
pub enum ReadError {
    /// The stream broke the framing rules; the reader is now closed.
    Fault(FrameFault),
    /// The source failed.
    Io(io::Error),
}

/// Splits an LF-delimited stream into bounded frames.
///
/// Partial reads and several records in one read are both accepted (RC03 §3). After a fault the
/// reader refuses every later call with the same fault: a malformed or oversized record cannot be
/// used to resynchronize by scanning attacker-selected content.
#[derive(Debug)]
pub struct FrameReader<R> {
    source: R,
    buffer: Vec<u8>,
    scanned: usize,
    closed: Option<FrameFault>,
}

impl<R: Read> FrameReader<R> {
    /// A reader over `source`.
    pub fn new(source: R) -> Self {
        Self {
            source,
            buffer: Vec::new(),
            scanned: 0,
            closed: None,
        }
    }

    /// The next frame payload, without its LF; `Ok(None)` at a clean end of stream.
    ///
    /// # Errors
    ///
    /// [`ReadError::Fault`] for a record over [`MAX_FRAME_BYTES`] (refused before the rest of it
    /// is read) or a stream ending inside a record; [`ReadError::Io`] when the source fails.
    pub fn next_frame(&mut self) -> Result<Option<Vec<u8>>, ReadError> {
        if let Some(fault) = self.closed {
            return Err(ReadError::Fault(fault));
        }
        let mut chunk = [0_u8; READ_CHUNK];
        loop {
            if let Some(offset) = self.buffer[self.scanned..].iter().position(|&b| b == b'\n') {
                let end = self.scanned + offset;
                if end > MAX_FRAME_BYTES {
                    return Err(self.close(FrameFault::Oversize));
                }
                let frame = self.buffer[..end].to_vec();
                self.buffer.drain(..=end);
                self.scanned = 0;
                return Ok(Some(frame));
            }
            self.scanned = self.buffer.len();
            // Never hold more than one oversized byte: the acquisition itself is the bound.
            if self.buffer.len() > MAX_FRAME_BYTES {
                return Err(self.close(FrameFault::Oversize));
            }
            let room = (MAX_FRAME_BYTES + 1 - self.buffer.len()).min(READ_CHUNK);
            let read = match self.source.read(&mut chunk[..room]) {
                Ok(read) => read,
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                Err(error) => return Err(ReadError::Io(error)),
            };
            if read == 0 {
                if self.buffer.is_empty() {
                    return Ok(None);
                }
                return Err(self.close(FrameFault::Truncated));
            }
            self.buffer.extend_from_slice(&chunk[..read]);
        }
    }

    fn close(&mut self, fault: FrameFault) -> ReadError {
        self.closed = Some(fault);
        self.buffer = Vec::new();
        ReadError::Fault(fault)
    }
}

/// The digest a task binds its acceptance criteria by: `request_sha256` of the criteria's compact
/// JSON array, strings in the order given, duplicates kept. The one door for it — submit records it
/// and the dispatcher compares a class's criteria with it (B14-P2a) — so the two cannot encode the
/// same list differently. It is over the re-serialised strings, so two wire spellings of one string
/// bind the same.
#[must_use]
pub fn criteria_digest<S: AsRef<str>>(criteria: &[S]) -> String {
    let array = serde_json::Value::Array(
        criteria
            .iter()
            .map(|criterion| serde_json::Value::String(criterion.as_ref().to_owned()))
            .collect(),
    );
    request_sha256(array.to_string().as_bytes())
}

/// `sha256:` over the exact payload bytes, excluding the terminal LF (RC03 §3).
#[must_use]
pub fn request_sha256(payload: &[u8]) -> String {
    let alphabet = b"0123456789abcdef";
    let mut text = String::from("sha256:");
    for byte in Sha256::digest(payload) {
        text.push(char::from(alphabet[usize::from(byte >> 4)]));
        text.push(char::from(alphabet[usize::from(byte & 15)]));
    }
    text
}

/// Admit one payload as a single compact JSON object, or refuse it by name.
///
/// The rules are the ones JSON Schema cannot see (RC03 §3): no byte-order mark, no whitespace
/// outside strings, unique member names at every level, nesting at most [`MAX_DEPTH`], integer
/// tokens only, no unpaired surrogate. They are checked on the bytes before any value exists.
///
/// # Errors
///
/// The first [`FrameFault`] the payload commits.
pub fn admit_object(payload: &[u8]) -> Result<Map<String, Value>, FrameFault> {
    if payload.len() > MAX_FRAME_BYTES {
        return Err(FrameFault::Oversize);
    }
    if payload.is_empty() {
        return Err(FrameFault::Empty);
    }
    if payload.starts_with(BYTE_ORDER_MARK) {
        return Err(FrameFault::ByteOrderMark);
    }
    if std::str::from_utf8(payload).is_err() {
        return Err(FrameFault::InvalidUtf8);
    }
    if payload[0] != b'{' {
        return Err(match payload[0] {
            b' ' | b'\t' | b'\r' | b'\n' => FrameFault::Whitespace,
            _ => FrameFault::NotObject,
        });
    }
    let mut scanner = Scanner {
        bytes: payload,
        at: 0,
    };
    scanner.value(0)?;
    if scanner.at != payload.len() {
        return Err(scanner.stray());
    }
    match serde_json::from_slice(payload) {
        Ok(Value::Object(members)) => Ok(members),
        Ok(_) => Err(FrameFault::NotObject),
        Err(_) => Err(FrameFault::Syntax),
    }
}

struct Scanner<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl Scanner<'_> {
    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.at).copied()
    }

    fn stray(&self) -> FrameFault {
        match self.peek() {
            Some(b' ' | b'\t' | b'\r' | b'\n') => FrameFault::Whitespace,
            _ => FrameFault::Syntax,
        }
    }

    fn expect(&mut self, byte: u8) -> Result<(), FrameFault> {
        if self.peek() == Some(byte) {
            self.at += 1;
            Ok(())
        } else {
            Err(self.stray())
        }
    }

    fn value(&mut self, depth: usize) -> Result<(), FrameFault> {
        match self.peek() {
            Some(b'{') => self.object(depth + 1),
            Some(b'[') => self.array(depth + 1),
            Some(b'"') => self.string().map(drop),
            Some(b't') => self.literal(b"true"),
            Some(b'f') => self.literal(b"false"),
            Some(b'n') => self.literal(b"null"),
            Some(b'0'..=b'9') => self.number(),
            Some(b'-') => Err(FrameFault::NumberToken),
            _ => Err(self.stray()),
        }
    }

    fn object(&mut self, depth: usize) -> Result<(), FrameFault> {
        if depth > MAX_DEPTH {
            return Err(FrameFault::TooDeep);
        }
        self.expect(b'{')?;
        if self.peek() == Some(b'}') {
            self.at += 1;
            return Ok(());
        }
        // An ordered set, so one bound-sized frame of ~10^5 names costs n log n comparisons,
        // not the n^2 a rescan per member would (CON-04; LRN03).
        let mut names: BTreeSet<String> = BTreeSet::new();
        loop {
            if self.peek() != Some(b'"') {
                return Err(self.stray());
            }
            let name = self.string()?;
            if !names.insert(name) {
                return Err(FrameFault::DuplicateName);
            }
            self.expect(b':')?;
            self.value(depth)?;
            match self.peek() {
                Some(b',') => self.at += 1,
                Some(b'}') => {
                    self.at += 1;
                    return Ok(());
                }
                _ => return Err(self.stray()),
            }
        }
    }

    fn array(&mut self, depth: usize) -> Result<(), FrameFault> {
        if depth > MAX_DEPTH {
            return Err(FrameFault::TooDeep);
        }
        self.expect(b'[')?;
        if self.peek() == Some(b']') {
            self.at += 1;
            return Ok(());
        }
        loop {
            self.value(depth)?;
            match self.peek() {
                Some(b',') => self.at += 1,
                Some(b']') => {
                    self.at += 1;
                    return Ok(());
                }
                _ => return Err(self.stray()),
            }
        }
    }

    fn literal(&mut self, word: &[u8]) -> Result<(), FrameFault> {
        if self.bytes[self.at..].starts_with(word) {
            self.at += word.len();
            Ok(())
        } else {
            Err(FrameFault::Syntax)
        }
    }

    fn number(&mut self) -> Result<(), FrameFault> {
        let leading_zero = self.peek() == Some(b'0');
        self.at += 1;
        while let Some(b'0'..=b'9') = self.peek() {
            if leading_zero {
                return Err(FrameFault::NumberToken);
            }
            self.at += 1;
        }
        match self.peek() {
            Some(b'.' | b'e' | b'E' | b'-' | b'+') => Err(FrameFault::NumberToken),
            _ => Ok(()),
        }
    }

    /// Decode one string, so two spellings of one name (`"a"`, `"a"`) are one name.
    fn string(&mut self) -> Result<String, FrameFault> {
        self.expect(b'"')?;
        let mut decoded = String::new();
        loop {
            let Some(byte) = self.peek() else {
                return Err(FrameFault::Syntax);
            };
            match byte {
                b'"' => {
                    self.at += 1;
                    return Ok(decoded);
                }
                b'\\' => {
                    self.at += 1;
                    decoded.push(self.escape()?);
                }
                0x00..=0x1f => return Err(FrameFault::ControlCharacter),
                _ => {
                    // UTF-8 was validated for the whole payload; copy one scalar value.
                    let rest = &self.bytes[self.at..];
                    let width = match byte {
                        0x00..=0x7f => 1,
                        0xc0..=0xdf => 2,
                        0xe0..=0xef => 3,
                        _ => 4,
                    };
                    let text = std::str::from_utf8(rest.get(..width).ok_or(FrameFault::Syntax)?)
                        .map_err(|_| FrameFault::InvalidUtf8)?;
                    decoded.push_str(text);
                    self.at += width;
                }
            }
        }
    }

    fn escape(&mut self) -> Result<char, FrameFault> {
        let Some(byte) = self.peek() else {
            return Err(FrameFault::Syntax);
        };
        self.at += 1;
        Ok(match byte {
            b'"' => '"',
            b'\\' => '\\',
            b'/' => '/',
            b'b' => '\u{8}',
            b'f' => '\u{c}',
            b'n' => '\n',
            b'r' => '\r',
            b't' => '\t',
            b'u' => {
                let unit = self.hex4()?;
                match unit {
                    0xd800..=0xdbff => {
                        if !self.bytes[self.at..].starts_with(b"\\u") {
                            return Err(FrameFault::UnpairedSurrogate);
                        }
                        self.at += 2;
                        let low = self.hex4()?;
                        if !(0xdc00..=0xdfff).contains(&low) {
                            return Err(FrameFault::UnpairedSurrogate);
                        }
                        let scalar = 0x1_0000 + ((unit - 0xd800) << 10) + (low - 0xdc00);
                        char::from_u32(scalar).ok_or(FrameFault::UnpairedSurrogate)?
                    }
                    0xdc00..=0xdfff => return Err(FrameFault::UnpairedSurrogate),
                    _ => char::from_u32(unit).ok_or(FrameFault::Syntax)?,
                }
            }
            _ => return Err(FrameFault::Syntax),
        })
    }

    fn hex4(&mut self) -> Result<u32, FrameFault> {
        let digits = self
            .bytes
            .get(self.at..self.at + 4)
            .ok_or(FrameFault::Syntax)?;
        let mut unit = 0_u32;
        for &digit in digits {
            let value = char::from(digit).to_digit(16).ok_or(FrameFault::Syntax)?;
            unit = unit * 16 + value;
        }
        self.at += 4;
        Ok(unit)
    }
}

/// `ErrorCodeV1`: the closed v1 code list (RC03 §4).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ErrorCode {
    InvalidFrame,
    UnsupportedProtocol,
    UnsupportedVersion,
    UnknownAction,
    UnsupportedActionVersion,
    Unauthenticated,
    Forbidden,
    InvalidArgument,
    NotFound,
    Conflict,
    StaleGeneration,
    DeadlineExceeded,
    Cancelled,
    ResourceExhausted,
    Unavailable,
    ResyncRequired,
    EffectUnknown,
    Internal,
}

impl ErrorCode {
    /// The wire spelling.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::InvalidFrame => "invalid_frame",
            Self::UnsupportedProtocol => "unsupported_protocol",
            Self::UnsupportedVersion => "unsupported_version",
            Self::UnknownAction => "unknown_action",
            Self::UnsupportedActionVersion => "unsupported_action_version",
            Self::Unauthenticated => "unauthenticated",
            Self::Forbidden => "forbidden",
            Self::InvalidArgument => "invalid_argument",
            Self::NotFound => "not_found",
            Self::Conflict => "conflict",
            Self::StaleGeneration => "stale_generation",
            Self::DeadlineExceeded => "deadline_exceeded",
            Self::Cancelled => "cancelled",
            Self::ResourceExhausted => "resource_exhausted",
            Self::Unavailable => "unavailable",
            Self::ResyncRequired => "resync_required",
            Self::EffectUnknown => "effect_unknown",
            Self::Internal => "internal",
        }
    }

    /// Every code, in the schema's order.
    pub const ALL: [Self; 18] = [
        Self::InvalidFrame,
        Self::UnsupportedProtocol,
        Self::UnsupportedVersion,
        Self::UnknownAction,
        Self::UnsupportedActionVersion,
        Self::Unauthenticated,
        Self::Forbidden,
        Self::InvalidArgument,
        Self::NotFound,
        Self::Conflict,
        Self::StaleGeneration,
        Self::DeadlineExceeded,
        Self::Cancelled,
        Self::ResourceExhausted,
        Self::Unavailable,
        Self::ResyncRequired,
        Self::EffectUnknown,
        Self::Internal,
    ];
}

/// When a caller may send again.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Retry {
    /// The same bytes will be refused the same way.
    Never,
    /// The same bytes may succeed later.
    SameExactRequest,
    /// Read back first.
    AfterReadback,
    /// After a condition outside the request changes (a grant, a composed owner).
    AfterCondition,
}

impl Retry {
    /// The wire spelling.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Never => "never",
            Self::SameExactRequest => "same_exact_request",
            Self::AfterReadback => "after_readback",
            Self::AfterCondition => "after_condition",
        }
    }
}

/// A correlated refusal. Every text is static, so none can echo the request.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Fault {
    /// The error code.
    pub code: ErrorCode,
    /// When to send again.
    pub retry: Retry,
    /// The JSON pointer of the offending member, when one is to blame.
    pub field: Option<&'static str>,
    /// The rule it broke.
    pub constraint: Option<&'static str>,
    /// A diagnostic; never program logic.
    pub message: &'static str,
    /// The read to send next: for `effect_unknown` the read that settles what happened (RC03 §4);
    /// for a `resource_exhausted` over a bounded view, the narrower route (contract-decisions.md
    /// "Overflow returns `resource_exhausted` with a narrower query/readback route").
    pub readback: Option<Value>,
    /// Set only for `stale_generation`: the generation the caller can now see (RC03 §6).
    pub current_generation: Option<Generation>,
}

impl Fault {
    /// `invalid_argument` at `field`, breaking `constraint`.
    #[must_use]
    pub const fn invalid(field: &'static str, constraint: &'static str) -> Self {
        Self {
            code: ErrorCode::InvalidArgument,
            retry: Retry::Never,
            field: Some(field),
            constraint: Some(constraint),
            message: "request does not satisfy HEE3-Control/1",
            readback: None,
            current_generation: None,
        }
    }

    /// A refusal that is not about one member.
    #[must_use]
    pub const fn of(code: ErrorCode, retry: Retry, message: &'static str) -> Self {
        Self {
            code,
            retry,
            field: None,
            constraint: None,
            message,
            readback: None,
            current_generation: None,
        }
    }

    /// `effect_unknown`: the durable boundary may or may not have committed. RC03 §4: always
    /// `effect: unknown`, `retry: after_readback`, and the concrete read that settles it.
    #[must_use]
    pub fn effect_unknown(readback: Value) -> Self {
        Self {
            code: ErrorCode::EffectUnknown,
            retry: Retry::AfterReadback,
            field: None,
            constraint: None,
            message: "the commit may or may not have happened; read back before any retry",
            readback: Some(readback),
            current_generation: None,
        }
    }

    /// `deadline_exceeded` at `/deadline_unix_ms`: the request's deadline had passed at receipt, and
    /// it is not an exact replay of a recorded request (RC03 §6: an unseen expired request is refused
    /// before dispatch). The one spelling of this refusal, for the wire and the catalogue alike.
    #[must_use]
    pub const fn expired() -> Self {
        Self::of(
            ErrorCode::DeadlineExceeded,
            Retry::Never,
            "the request deadline has passed",
        )
        .at("/deadline_unix_ms")
    }

    /// `resync_required` at `field`: the cursor no longer names the listing it was issued for; the
    /// caller starts again without one.
    #[must_use]
    pub const fn resync(field: &'static str) -> Self {
        Self::of(
            ErrorCode::ResyncRequired,
            Retry::Never,
            "the cursor no longer names this listing; start again without one",
        )
        .at(field)
    }

    /// `stale_generation` at `field`: the precondition names a generation the resource has moved
    /// past, and `current` is the one the caller can now see (RC03 §6: "with current visible
    /// generation where authorized"). `retry: never`, because a generation only grows: the same
    /// bytes are refused the same way for ever, and a new request must name `current`.
    #[must_use]
    pub const fn stale(field: &'static str, current: Generation) -> Self {
        Self {
            code: ErrorCode::StaleGeneration,
            retry: Retry::Never,
            field: Some(field),
            constraint: Some("the resource's current generation"),
            message: "the resource has moved past the expected generation",
            readback: None,
            current_generation: Some(current),
        }
    }

    /// The same refusal, naming the rule it applied.
    #[must_use]
    pub const fn because(mut self, constraint: &'static str) -> Self {
        self.constraint = Some(constraint);
        self
    }

    /// The same refusal, naming the member it applied to.
    #[must_use]
    pub const fn at(mut self, field: &'static str) -> Self {
        self.field = Some(field);
        self
    }

    /// The `ControlErrorV1` record for this refusal, LF-terminated.
    #[must_use]
    pub fn frame(&self, request_id: &str, request_sha256: &str) -> Vec<u8> {
        record(&json!({
            "protocol": PROTOCOL,
            "version": PROTOCOL_VERSION,
            "kind": "error",
            "request_id": request_id,
            "request_sha256": request_sha256,
            "code": self.code.name(),
            // The effect is unknown only for `effect_unknown`; any other refusal may still carry a
            // readback (the narrower route a `resource_exhausted` names, B09) and had no effect.
            "effect": if self.code == ErrorCode::EffectUnknown { "unknown" } else { "none" },
            "retry": self.retry.name(),
            "readback": self.readback,
            "message": self.message,
            "details": {
                "field": self.field,
                "constraint": self.constraint,
                "current_generation": self.current_generation.map(|generation| generation.to_string()),
            },
        }))
    }
}

/// What an action did to durable state. `pending` is not representable here: it requires an
/// operation identity and a readback route that no composed action yet produces (RC03 §4).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResultEffect {
    /// Nothing durable changed.
    None,
    /// The action's durable boundary committed.
    Committed,
}

/// A result's envelope facts beyond its body.
#[derive(Clone, Debug, PartialEq)]
pub struct Outcome {
    /// What the action did to durable state.
    pub effect: ResultEffect,
    /// Whether this is the stored result of an exact replay rather than a new execution.
    pub replayed: bool,
    /// The resource generation the result observed.
    pub observed_generation: Option<String>,
    /// The read that finds this result again after a lost reply (a `ReadbackSelectorV1`).
    pub readback: Option<Value>,
    /// The action's result body.
    pub body: Value,
}

impl Outcome {
    /// A read: nothing committed, nothing to read back.
    #[must_use]
    pub fn read(body: Value) -> Self {
        Self {
            effect: ResultEffect::None,
            replayed: false,
            observed_generation: None,
            readback: None,
            body,
        }
    }
}

/// A `ControlResultV1`, LF-terminated.
#[must_use]
pub fn result_frame(request_id: &str, request_sha256: &str, outcome: &Outcome) -> Vec<u8> {
    record(&json!({
        "protocol": PROTOCOL,
        "version": PROTOCOL_VERSION,
        "kind": "result",
        "request_id": request_id,
        "request_sha256": request_sha256,
        "replayed": outcome.replayed,
        "effect": match outcome.effect {
            ResultEffect::None => "none",
            ResultEffect::Committed => "committed",
        },
        "operation_id": null,
        "observed_generation": outcome.observed_generation,
        "readback": outcome.readback,
        "body": outcome.body,
    }))
}

/// A `ControlResultV1` for an action that committed nothing, LF-terminated.
#[must_use]
pub fn read_result_frame(request_id: &str, request_sha256: &str, body: &Value) -> Vec<u8> {
    result_frame(request_id, request_sha256, &Outcome::read(body.clone()))
}

fn record(value: &Value) -> Vec<u8> {
    let mut bytes = value.to_string().into_bytes();
    bytes.push(b'\n');
    bytes
}

/// The widest page a caller may ask for (`PageInV1.limit`, RC03 §4).
pub const MAX_PAGE_LIMIT: u64 = 100;
/// The longest `after_key` (`PageCursorV1`, `ASCII[1..256]`).
const MAX_AFTER_KEY_BYTES: usize = 256;

/// `PageInV1` (RC03 §4): a page limit of 1..100 and the cursor that continues a listing, if any.
/// The one reader of a listing's page, for every action that pages.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PageIn {
    /// The most items the caller asked for.
    pub limit: u64,
    /// The continuation cursor, or `None` for a listing's first page.
    pub cursor: Option<PageCursor>,
}

impl PageIn {
    /// Read `page` (the body member).
    ///
    /// # Errors
    ///
    /// `invalid_argument` naming the member: `/body/page` unless an object of exactly `limit` and
    /// `cursor`; `/body/page/limit` outside 1..100; `/body/page/cursor` unless null or a
    /// `PageCursorV1` ([`PageCursor::parse`]).
    pub fn parse(page: Option<&Value>) -> Result<Self, Fault> {
        let Some(Value::Object(page)) = page else {
            return Err(Fault::invalid("/body/page", "PageInV1"));
        };
        if page.len() != 2 {
            return Err(Fault::invalid("/body/page", "exactly limit and cursor"));
        }
        let limit = page
            .get("limit")
            .and_then(Value::as_u64)
            .filter(|limit| (1..=MAX_PAGE_LIMIT).contains(limit))
            .ok_or(Fault::invalid("/body/page/limit", "integer 1..100"))?;
        let cursor = match page.get("cursor") {
            Some(Value::Null) => None,
            Some(Value::Object(cursor)) => Some(PageCursor::parse(cursor)?),
            _ => {
                return Err(Fault::invalid("/body/page/cursor", "null or PageCursorV1"));
            }
        };
        Ok(Self { limit, cursor })
    }
}

/// `PageCursorV1` (RC03 §4), read member by member. A cursor is a selector over one listing, never a
/// grant: the listing that issued it decides what its snapshot and `after_key` mean.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PageCursor {
    /// The snapshot the listing was issued under.
    pub snapshot_revision: u64,
    /// The opaque ordering key the next page starts after.
    pub after_key: String,
    /// The digest of the filter it was issued for.
    pub filter_sha256: String,
    /// When it stops naming the listing.
    pub expires_unix_ms: u64,
}

impl PageCursor {
    /// Read a `PageCursorV1` object.
    ///
    /// # Errors
    ///
    /// `invalid_argument` naming the member: `/body/page/cursor` unless exactly the four members;
    /// then each member's own rule.
    pub fn parse(cursor: &Map<String, Value>) -> Result<Self, Fault> {
        let text = |name: &str| cursor.get(name).and_then(Value::as_str);
        if cursor.len() != 4 {
            return Err(Fault::invalid(
                "/body/page/cursor",
                "exactly snapshot_revision, after_key, filter_sha256 and expires_unix_ms",
            ));
        }
        let snapshot_revision = text("snapshot_revision")
            .and_then(|value| parse_u64_decimal(value).ok())
            .ok_or(Fault::invalid(
                "/body/page/cursor/snapshot_revision",
                "U64Decimal",
            ))?;
        let after_key = text("after_key")
            .filter(|key| (1..=MAX_AFTER_KEY_BYTES).contains(&key.len()) && key.is_ascii())
            .ok_or(Fault::invalid(
                "/body/page/cursor/after_key",
                "ASCII of 1..256 bytes",
            ))?;
        let filter_sha256 = text("filter_sha256")
            .filter(|digest| Sha256Digest::parse(digest).is_ok())
            .ok_or(Fault::invalid("/body/page/cursor/filter_sha256", "Sha256"))?;
        let expires_unix_ms = text("expires_unix_ms")
            .and_then(|value| parse_u64_decimal(value).ok())
            .ok_or(Fault::invalid(
                "/body/page/cursor/expires_unix_ms",
                "U64Decimal",
            ))?;
        Ok(Self {
            snapshot_revision,
            after_key: after_key.to_owned(),
            filter_sha256: filter_sha256.to_owned(),
            expires_unix_ms,
        })
    }

    /// Whether it may resume a listing of `filter` at `now_unix_ms`. The snapshot is the issuing
    /// listing's to judge, before this.
    ///
    /// # Errors
    ///
    /// `resync_required` once it has expired; `invalid_argument` at `filter_sha256` for another filter.
    pub fn resumes(&self, filter: &str, now_unix_ms: u64) -> Result<(), Fault> {
        if self.expires_unix_ms <= now_unix_ms {
            return Err(Fault::resync("/body/page/cursor/expires_unix_ms"));
        }
        if self.filter_sha256 != filter {
            return Err(Fault::invalid(
                "/body/page/cursor/filter_sha256",
                "the filter this cursor was issued for",
            ));
        }
        Ok(())
    }
}

/// What an operator decides about an unresolved obligation (`task.resolve`, RC03 section 6;
/// contract-decisions.md:344). The one vocabulary for the wire parser and the store.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Disposition {
    /// Ask the obligation's owner to try again; closes nothing.
    Retry,
    /// Give the obligation up (a delivery), or stop the task through the stop door (an attempt).
    Abandon,
    /// Attest that an unknown external effect happened.
    AcknowledgeExternalEffect,
    /// Hold the task apart (`blocked`); closes nothing.
    Quarantine,
}

impl Disposition {
    /// Every disposition, in the contract's order.
    pub const ALL: [Self; 4] = [
        Self::Retry,
        Self::Abandon,
        Self::AcknowledgeExternalEffect,
        Self::Quarantine,
    ];

    /// The wire spelling.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Retry => "retry",
            Self::Abandon => "abandon",
            Self::AcknowledgeExternalEffect => "acknowledge_external_effect",
            Self::Quarantine => "quarantine",
        }
    }

    /// The disposition spelled `name`, if the vocabulary has one.
    #[must_use]
    pub fn parse(name: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|disposition| disposition.name() == name)
    }
}

/// `task.get`'s evidence view other than `none` (B09): `summary` names the outcome's evidence,
/// `refs` every reference the task holds.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EvidenceView {
    /// The outcome's evidence: bounded by construction.
    Summary,
    /// Every reference, each identity once, refused past the contract's 64.
    Refs,
}

/// The longest `media_type` or `schema_id` an `EvidenceRefV1` carries, in bytes.
pub const MAX_EVIDENCE_NAME_BYTES: usize = 128;

/// `EvidenceRefV1` (RC03 section 4): a reference to an artifact, never a path or URI. The ledger
/// checks `sha256` and `byte_length` against what it holds; `artifact_id` is recorded as given.
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct EvidenceRef {
    /// The artifact's identity as the caller names it.
    pub artifact_id: String,
    /// Its content digest.
    pub sha256: String,
    /// Its size in bytes.
    pub byte_length: u64,
    /// Its media type (ASCII 1..128).
    pub media_type: String,
    /// Its schema (ASCII 1..128).
    pub schema_id: String,
}

impl EvidenceRef {
    /// The one reader of an `EvidenceRefV1` value: exactly its five members, each well formed —
    /// for a reference the wire carries and for one the ledger stored (B09: a stored value this
    /// refuses is corruption).
    #[must_use]
    pub fn parse(item: &Value) -> Option<Self> {
        let Value::Object(members) = item else {
            return None;
        };
        let text = |name: &str| members.get(name).and_then(Value::as_str);
        let name = |name: &str| {
            text(name)
                .filter(|text| {
                    (1..=MAX_EVIDENCE_NAME_BYTES).contains(&text.len()) && text.is_ascii()
                })
                .map(str::to_owned)
        };
        (members.len() == 5).then_some(())?;
        Some(Self {
            artifact_id: text("artifact_id")
                .filter(|id| UuidV4::parse(id).is_ok())?
                .to_owned(),
            sha256: text("sha256")
                .filter(|digest| Sha256Digest::parse(digest).is_ok())?
                .to_owned(),
            byte_length: members
                .get("byte_length")
                .and_then(Value::as_u64)
                .and_then(|length| u32::try_from(length).ok())
                .map(u64::from)?,
            media_type: name("media_type")?,
            schema_id: name("schema_id")?,
        })
    }
}

/// Why a cancellation is asked for: `task.cancel`'s closed reason set (RC03 §6;
/// contract-decisions.md:343). The one vocabulary for every door that records a cancellation intent
/// -- the wire parser reads it, the store writes it -- so no door can record a reason outside it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CancelReason {
    /// An operator asked (also a roster disable with `request_cancel`).
    OperatorRequest,
    /// Other work supersedes the task.
    Superseded,
    /// The task's budget.
    Budget,
    /// The task's deadline.
    Deadline,
    /// Safety.
    Safety,
}

impl CancelReason {
    /// Every reason, in the contract's order.
    pub const ALL: [Self; 5] = [
        Self::OperatorRequest,
        Self::Superseded,
        Self::Budget,
        Self::Deadline,
        Self::Safety,
    ];

    /// The wire spelling.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::OperatorRequest => "operator_request",
            Self::Superseded => "superseded",
            Self::Budget => "budget",
            Self::Deadline => "deadline",
            Self::Safety => "safety",
        }
    }

    /// The reason spelled `name`, if the vocabulary has one.
    #[must_use]
    pub fn parse(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|reason| reason.name() == name)
    }
}

/// `ResourceKind`: what a generation precondition names.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResourceKind {
    Task,
    Thread,
    Roster,
    Service,
    Analysis,
}

impl ResourceKind {
    /// The wire spelling.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Task => "task",
            Self::Thread => "thread",
            Self::Roster => "roster",
            Self::Service => "service",
            Self::Analysis => "analysis",
        }
    }

    /// Every kind.
    pub const ALL: [Self; 5] = [
        Self::Task,
        Self::Thread,
        Self::Roster,
        Self::Service,
        Self::Analysis,
    ];

    fn parse(text: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.name() == text)
    }
}

/// `PreconditionV1`: the exact generation a mutation of existing state expects.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Precondition {
    /// The kind of resource.
    pub resource: ResourceKind,
    /// Its identity.
    pub id: String,
    /// The generation the caller last observed.
    pub generation: Generation,
}

/// A request whose common envelope satisfies RC03 §4. What the named action requires of it is
/// not yet checked: that is the catalogue's to decide.
#[derive(Clone, Debug, PartialEq)]
pub struct Envelope {
    /// The caller's request identity.
    pub request_id: String,
    /// The digest of the exact admitted bytes.
    pub request_sha256: String,
    /// The action named, not yet looked up.
    pub action: String,
    /// The action version named.
    pub action_version: u64,
    /// The idempotency key, when one was sent.
    pub idempotency_key: Option<String>,
    /// The deadline: inside the admitted window in [`Received::Admitted`], passed in
    /// [`Received::Expired`].
    pub deadline_unix_ms: u64,
    /// The server-side grant the request is made under.
    pub grant_id: String,
    /// The reviewed scope that grant must carry.
    pub scope_sha256: String,
    /// The generation precondition, when one was sent.
    pub precondition: Option<Precondition>,
    /// The action body, not yet materialized into its typed form.
    pub body: Map<String, Value>,
}

/// What receiving one frame payload produced.
#[derive(Clone, Debug, PartialEq)]
pub enum Received {
    /// No reply may be written; close the connection.
    Closed(FrameFault),
    /// A correlated refusal to write back.
    Refused {
        /// The request's identity.
        request_id: String,
        /// The digest of its exact bytes.
        request_sha256: String,
        /// Why.
        fault: Fault,
    },
    /// An envelope ready for the catalogue.
    Admitted(Envelope),
    /// A well-formed envelope whose deadline had passed at receipt. RC03 §6: it may be answered only
    /// as the readback of a request already recorded under its key; anything else is refused
    /// [`Fault::expired`] before dispatch. A variant of its own, so no consumer can dispatch one by
    /// matching on `Admitted`.
    Expired(Envelope),
}

const ENVELOPE_MEMBERS: [&str; 11] = [
    "protocol",
    "version",
    "kind",
    "request_id",
    "action",
    "action_version",
    "idempotency_key",
    "deadline_unix_ms",
    "authority",
    "precondition",
    "body",
];

/// Receive one frame payload (without its LF) at receiver wall time `now_unix_ms`.
///
/// Order: the frame rules; correlation (`request_id`); then protocol, version and kind; then
/// every member's shape and the deadline window. A reply is possible only once the request
/// identity was read from a safely admitted object.
#[must_use]
pub fn receive(payload: &[u8], now_unix_ms: u64) -> Received {
    let members = match admit_object(payload) {
        Ok(members) => members,
        Err(fault) => return Received::Closed(fault),
    };
    let Some(request_id) = members
        .get("request_id")
        .and_then(Value::as_str)
        .filter(|id| UuidV4::parse(id).is_ok())
    else {
        return Received::Closed(FrameFault::Uncorrelated);
    };
    let request_sha256 = request_sha256(payload);
    match envelope(&members, now_unix_ms) {
        Ok(parts) if parts.expired => {
            Received::Expired(parts.identified(request_id, request_sha256))
        }
        Ok(parts) => Received::Admitted(parts.identified(request_id, request_sha256)),
        Err(fault) => Received::Refused {
            request_id: request_id.to_owned(),
            request_sha256,
            fault,
        },
    }
}

/// An envelope's members before the identity it is correlated by is attached.
struct Parts {
    expired: bool,
    action: String,
    action_version: u64,
    idempotency_key: Option<String>,
    deadline_unix_ms: u64,
    grant_id: String,
    scope_sha256: String,
    precondition: Option<Precondition>,
    body: Map<String, Value>,
}

impl Parts {
    fn identified(self, request_id: &str, request_sha256: String) -> Envelope {
        Envelope {
            request_id: request_id.to_owned(),
            request_sha256,
            action: self.action,
            action_version: self.action_version,
            idempotency_key: self.idempotency_key,
            deadline_unix_ms: self.deadline_unix_ms,
            grant_id: self.grant_id,
            scope_sha256: self.scope_sha256,
            precondition: self.precondition,
            body: self.body,
        }
    }
}

fn envelope(members: &Map<String, Value>, now_unix_ms: u64) -> Result<Parts, Fault> {
    match members.get("protocol") {
        Some(Value::String(protocol)) if protocol == PROTOCOL => {}
        Some(Value::String(_)) => {
            return Err(Fault::of(
                ErrorCode::UnsupportedProtocol,
                Retry::Never,
                "this receiver speaks hee3.control only",
            )
            .at("/protocol"));
        }
        _ => return Err(Fault::invalid("/protocol", "required string")),
    }
    match members.get("version").map(version_number) {
        Some(Some(PROTOCOL_VERSION)) => {}
        Some(Some(_)) => {
            return Err(Fault::of(
                ErrorCode::UnsupportedVersion,
                Retry::Never,
                "this receiver speaks protocol version 1 only",
            )
            .at("/version"));
        }
        _ => return Err(Fault::invalid("/version", "integer 1..65535")),
    }
    if members.get("kind").and_then(Value::as_str) != Some("request") {
        return Err(Fault::invalid("/kind", "const request"));
    }
    for name in ENVELOPE_MEMBERS {
        if !members.contains_key(name) {
            return Err(Fault::invalid(pointer(name), "required member"));
        }
    }
    if members.len() != ENVELOPE_MEMBERS.len() {
        return Err(Fault::invalid("/", "unknown member"));
    }
    let action = members["action"]
        .as_str()
        .ok_or(Fault::invalid("/action", "ActionId string"))?;
    let action_version = version_number(&members["action_version"])
        .ok_or(Fault::invalid("/action_version", "integer 1..65535"))?;
    let idempotency_key = match &members["idempotency_key"] {
        Value::Null => None,
        Value::String(key) if UuidV4::parse(key).is_ok() => Some(key.clone()),
        _ => return Err(Fault::invalid("/idempotency_key", "null or UuidV4")),
    };
    let deadline_unix_ms = members["deadline_unix_ms"]
        .as_str()
        .and_then(|text| parse_u64_decimal(text).ok())
        .ok_or(Fault::invalid("/deadline_unix_ms", "U64Decimal"))?;
    let expired = deadline_unix_ms <= now_unix_ms;
    if !expired && deadline_unix_ms - now_unix_ms > MAX_DEADLINE_AHEAD_MS {
        return Err(Fault::invalid(
            "/deadline_unix_ms",
            "at most 60000 ms after receipt",
        ));
    }
    // An expired request is read whole, because an exact replay is answered from its record; one
    // that is malformed past its deadline is refused for the deadline, as the order above says.
    let tail = authority(&members["authority"]).and_then(|(grant_id, scope_sha256)| {
        let precondition = match &members["precondition"] {
            Value::Null => None,
            value => Some(precondition(value)?),
        };
        let Value::Object(body) = &members["body"] else {
            return Err(Fault::invalid("/body", "object"));
        };
        Ok((grant_id, scope_sha256, precondition, body))
    });
    let (grant_id, scope_sha256, precondition, body) = match tail {
        Ok(tail) => tail,
        Err(_) if expired => return Err(Fault::expired()),
        Err(fault) => return Err(fault),
    };
    Ok(Parts {
        expired,
        action: action.to_owned(),
        action_version,
        idempotency_key,
        deadline_unix_ms,
        grant_id,
        scope_sha256,
        precondition,
        body: body.clone(),
    })
}

/// RC03 §2: a version is a JSON integer in `1..=65535`; zero, strings and Booleans are not.
fn version_number(value: &Value) -> Option<u64> {
    value
        .as_u64()
        .filter(|version| (1..=65_535).contains(version))
}

fn pointer(name: &'static str) -> &'static str {
    match name {
        "protocol" => "/protocol",
        "version" => "/version",
        "kind" => "/kind",
        "request_id" => "/request_id",
        "action" => "/action",
        "action_version" => "/action_version",
        "idempotency_key" => "/idempotency_key",
        "deadline_unix_ms" => "/deadline_unix_ms",
        "authority" => "/authority",
        "precondition" => "/precondition",
        _ => "/body",
    }
}

fn authority(value: &Value) -> Result<(String, String), Fault> {
    let Value::Object(members) = value else {
        return Err(Fault::invalid("/authority", "object"));
    };
    if members.len() != 2 {
        return Err(Fault::invalid(
            "/authority",
            "exactly grant_id and scope_sha256",
        ));
    }
    let grant_id = members
        .get("grant_id")
        .and_then(Value::as_str)
        .filter(|id| UuidV4::parse(id).is_ok())
        .ok_or(Fault::invalid("/authority/grant_id", "UuidV4"))?;
    let scope = members
        .get("scope_sha256")
        .and_then(Value::as_str)
        .filter(|digest| Sha256Digest::parse(digest).is_ok())
        .ok_or(Fault::invalid("/authority/scope_sha256", "Sha256"))?;
    Ok((grant_id.to_owned(), scope.to_owned()))
}

fn precondition(value: &Value) -> Result<Precondition, Fault> {
    let Value::Object(members) = value else {
        return Err(Fault::invalid("/precondition", "null or PreconditionV1"));
    };
    if members.len() != 3 {
        return Err(Fault::invalid(
            "/precondition",
            "exactly resource, id and generation",
        ));
    }
    let resource = members
        .get("resource")
        .and_then(Value::as_str)
        .and_then(ResourceKind::parse)
        .ok_or(Fault::invalid("/precondition/resource", "ResourceKind"))?;
    let id = members
        .get("id")
        .and_then(Value::as_str)
        .filter(|id| UuidV4::parse(id).is_ok())
        .ok_or(Fault::invalid("/precondition/id", "UuidV4"))?;
    let generation = members
        .get("generation")
        .and_then(Value::as_str)
        .and_then(|text| text.parse::<Generation>().ok())
        .ok_or(Fault::invalid("/precondition/generation", "Generation"))?;
    Ok(Precondition {
        resource,
        id: id.to_owned(),
        generation,
    })
}

/// `BodyResult_health.recovery`: whether the coordinator's startup reconciliation left anything
/// for an operator.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Recovery {
    /// Every attempt reconciled; nothing outstanding.
    Complete,
    /// Reconciled, with obligations still open (an unknown outcome retained, cleanup owed,
    /// verification outstanding, a live attempt under observation).
    Pending,
    /// The engine cannot act: no ledger, a refused startup, or reconciliation mode.
    Blocked,
}

/// `BodyResult_health.database`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Database {
    /// The ledger is open for writing.
    Ready,
    /// The ledger is readable but inspection-only.
    Degraded,
    /// There is no usable ledger.
    Unavailable,
}

/// `BodyResult_health.socket`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Socket {
    /// This engine serves the control socket.
    Owned,
    /// It is draining connections before it stops.
    Draining,
}

impl Recovery {
    /// The wire spelling.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Complete => "complete",
            Self::Pending => "pending",
            Self::Blocked => "blocked",
        }
    }
}

impl Database {
    /// The wire spelling.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::Degraded => "degraded",
            Self::Unavailable => "unavailable",
        }
    }
}

impl Socket {
    /// The wire spelling.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Owned => "owned",
            Self::Draining => "draining",
        }
    }
}

/// One health observation. `ready` is not stored: it is derived, so a snapshot cannot claim
/// readiness its parts deny.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Health {
    /// Startup reconciliation's outcome.
    pub recovery: Recovery,
    /// The ledger's state.
    pub database: Database,
    /// The socket's state.
    pub socket: Socket,
    /// When this was observed, receiver wall time.
    pub checked_unix_ms: u64,
}

impl Health {
    /// Ready exactly when reconciliation is complete, the ledger writable and the socket owned.
    #[must_use]
    pub fn ready(&self) -> bool {
        self.recovery == Recovery::Complete
            && self.database == Database::Ready
            && self.socket == Socket::Owned
    }

    /// The `BodyResult_health` object.
    #[must_use]
    pub fn body(&self) -> Value {
        json!({
            "protocol_version": PROTOCOL_VERSION,
            "engine_version": env!("CARGO_PKG_VERSION"),
            "ready": self.ready(),
            "recovery": self.recovery.name(),
            "database": self.database.name(),
            "socket": self.socket.name(),
            "checked_unix_ms": self.checked_unix_ms.to_string(),
        })
    }
}
