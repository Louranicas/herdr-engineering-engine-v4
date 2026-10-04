//! The control frame: one compact JSON object per line, in and out, and the refusal names.
//!
//! Request: `{request_id, action, action_version, idempotency_key, body}` (other envelope fields
//! of the API Map are accepted and ignored by the skeleton). Reply: `{kind:"result", request_id,
//! replayed, body}` or `{kind:"error", request_id, code, retry, message, field}`.

use serde_json::{Value, json};

/// The longest frame read, in bytes (Socket and IPC Map "Bound").
pub const MAX_FRAME_BYTES: usize = 1_048_576;

/// Every refusal the socket answers with, one name each (Error and Refusal Map vocabulary;
/// `not_ready` is this slice's addition, see FLOW.md and the DC proposal).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Code {
    /// The line is not a JSON object, or a required member is absent or mistyped.
    InvalidArgument,
    /// The action is not in the skeleton's catalogue.
    UnknownAction,
    /// The action exists, the version does not.
    UnsupportedActionVersion,
    /// A mutating action arrived before startup reconcile completed.
    NotReady,
    /// Same idempotency key, other bytes; or a transition the ledger refused.
    Conflict,
    /// The task does not exist.
    NotFound,
    /// The peer is not this process's uid (`SO_PEERCRED`).
    Forbidden,
    /// The ledger failed under the request.
    Internal,
    /// `task.preview`: `route::select` found no model for the brief (a result-body name).
    NoRoute,
    /// `events.subscribe`: the subscriber fell a full queue behind; the stream closes.
    SlowConsumer,
    /// A request line was longer than [`MAX_FRAME_BYTES`]; the connection closes after this frame.
    FrameTooLarge,
    /// The server already holds its cap of concurrent connections.
    TooManyConnections,
}

impl Code {
    /// Every name, in declaration order (`ALL[c.ordinal()] == c`).
    pub const ALL: [Self; 12] = [
        Self::InvalidArgument,
        Self::UnknownAction,
        Self::UnsupportedActionVersion,
        Self::NotReady,
        Self::Conflict,
        Self::NotFound,
        Self::Forbidden,
        Self::Internal,
        Self::NoRoute,
        Self::SlowConsumer,
        Self::FrameTooLarge,
        Self::TooManyConnections,
    ];

    /// Position in [`Code::ALL`]. No wildcard arm: a new variant does not compile until it has
    /// a position, and the table test then fails until `ALL` and FLOW.md name it.
    #[must_use]
    pub const fn ordinal(self) -> usize {
        match self {
            Self::InvalidArgument => 0,
            Self::UnknownAction => 1,
            Self::UnsupportedActionVersion => 2,
            Self::NotReady => 3,
            Self::Conflict => 4,
            Self::NotFound => 5,
            Self::Forbidden => 6,
            Self::Internal => 7,
            Self::NoRoute => 8,
            Self::SlowConsumer => 9,
            Self::FrameTooLarge => 10,
            Self::TooManyConnections => 11,
        }
    }

    /// The wire spelling.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::InvalidArgument => "invalid_argument",
            Self::UnknownAction => "unknown_action",
            Self::UnsupportedActionVersion => "unsupported_action_version",
            Self::NotReady => "not_ready",
            Self::Conflict => "conflict",
            Self::NotFound => "not_found",
            Self::Forbidden => "forbidden",
            Self::Internal => "internal",
            Self::NoRoute => "no_route",
            Self::SlowConsumer => "slow_consumer",
            Self::FrameTooLarge => "frame_too_large",
            Self::TooManyConnections => "too_many_connections",
        }
    }

    /// The retry class (Error map `Retry`).
    #[must_use]
    pub const fn retry(self) -> &'static str {
        match self {
            Self::NotReady | Self::NoRoute | Self::SlowConsumer | Self::TooManyConnections => {
                "after_condition"
            }
            Self::Conflict => "after_readback",
            Self::Internal => "same_exact_request",
            Self::InvalidArgument
            | Self::UnknownAction
            | Self::UnsupportedActionVersion
            | Self::NotFound
            | Self::Forbidden
            | Self::FrameTooLarge => "never",
        }
    }
}

/// A refusal: its one name, the JSON pointer it is about, and a static message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fault {
    /// The name.
    pub code: Code,
    /// JSON pointer into the request.
    pub field: &'static str,
    /// Detail (never an echo of request bytes beyond a refusal's own text).
    pub message: String,
}

impl Fault {
    /// A fault at `field`.
    #[must_use]
    pub fn new(code: Code, field: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            field,
            message: message.into(),
        }
    }
}

/// A parsed request.
#[derive(Debug, Clone, PartialEq)]
pub struct Request {
    /// Correlation id, echoed.
    pub request_id: String,
    /// The action name.
    pub action: String,
    /// The action's contract version.
    pub action_version: u32,
    /// Present for a mutating action.
    pub idempotency_key: Option<String>,
    /// The action's body.
    pub body: Value,
}

/// Parse one line. A line that is not a JSON object is refused `invalid_argument` at `/`.
///
/// # Errors
/// [`Code::InvalidArgument`] naming the first bad member.
pub fn parse(line: &str) -> Result<Request, (String, Fault)> {
    let none = String::new();
    let Ok(Value::Object(mut obj)) = serde_json::from_str::<Value>(line) else {
        return Err((
            none,
            Fault::new(Code::InvalidArgument, "/", "not a JSON object"),
        ));
    };
    let request_id = match obj.get("request_id") {
        Some(Value::String(s)) if !s.is_empty() => s.clone(),
        _ => {
            return Err((
                none,
                Fault::new(Code::InvalidArgument, "/request_id", "string required"),
            ));
        }
    };
    let bad = |field, msg: &str| {
        (
            request_id.clone(),
            Fault::new(Code::InvalidArgument, field, msg),
        )
    };
    let Some(Value::String(action)) = obj.get("action").cloned() else {
        return Err(bad("/action", "string required"));
    };
    let action_version = obj
        .get("action_version")
        .and_then(Value::as_u64)
        .and_then(|v| u32::try_from(v).ok())
        .ok_or_else(|| bad("/action_version", "unsigned integer required"))?;
    let idempotency_key = match obj.get("idempotency_key") {
        None | Some(Value::Null) => None,
        Some(Value::String(s)) if !s.is_empty() => Some(s.clone()),
        Some(_) => return Err(bad("/idempotency_key", "non-empty string or null")),
    };
    let body = match obj.remove("body") {
        None | Some(Value::Null) => json!({}),
        Some(b @ Value::Object(_)) => b,
        Some(_) => return Err(bad("/body", "object required")),
    };
    Ok(Request {
        request_id,
        action,
        action_version,
        idempotency_key,
        body,
    })
}

/// A result frame.
#[must_use]
pub fn result(request_id: &str, replayed: bool, body: Value) -> Value {
    let mut v = json!({"kind": "result", "request_id": request_id, "replayed": replayed});
    v["body"] = body;
    v
}

/// An error frame.
#[must_use]
pub fn error(request_id: &str, fault: &Fault) -> Value {
    json!({
        "kind": "error",
        "request_id": request_id,
        "code": fault.code.name(),
        "retry": fault.code.retry(),
        "field": fault.field,
        "message": fault.message,
    })
}

/// A stream's close frame: the last line before the server closes the connection.
#[must_use]
pub fn close(code: Code, message: &str) -> Value {
    json!({"kind": "close", "code": code.name(), "retry": code.retry(), "message": message})
}

/// A request frame (the CLI's half).
#[must_use]
pub fn request(
    request_id: &str,
    action: &str,
    idempotency_key: Option<&str>,
    body: Value,
) -> Value {
    let mut v = json!({
        "request_id": request_id,
        "action": action,
        "action_version": 1,
        "idempotency_key": idempotency_key,
    });
    v["body"] = body;
    v
}

#[cfg(test)]
mod tests {
    use super::Code;
    use std::collections::BTreeSet;

    /// The names in FLOW.md's "Refusal names" table, first column.
    fn flow_table() -> Vec<String> {
        let flow = include_str!("../FLOW.md");
        let section = flow
            .split("## Refusal names")
            .nth(1)
            .and_then(|s| s.split("\n## ").next())
            .unwrap_or("");
        section
            .lines()
            .filter_map(|l| l.strip_prefix("| `"))
            .filter_map(|l| l.split('`').next())
            .map(str::to_owned)
            .collect()
    }

    #[test]
    fn every_emittable_refusal_has_one_row_in_flow() {
        for (i, c) in Code::ALL.iter().enumerate() {
            assert_eq!(c.ordinal(), i, "{c:?} out of place in Code::ALL");
        }
        let code: BTreeSet<String> = Code::ALL.iter().map(|c| c.name().to_owned()).collect();
        let table = flow_table();
        let rows: BTreeSet<String> = table.iter().cloned().collect();
        assert_eq!(rows.len(), table.len(), "a name appears twice: {table:?}");
        assert_eq!(code.len(), Code::ALL.len(), "two variants share a name");
        assert_eq!(rows, code);
    }
}
