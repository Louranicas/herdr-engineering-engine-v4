//! The control frame: one compact JSON object per line, in and out, and the refusal names.
//!
//! Request: `{request_id, action, action_version, idempotency_key, precondition, body}` (other
//! envelope fields of the API Map are accepted and ignored by the skeleton). Reply:
//! `{kind:"result", request_id, replayed, body}` or `{kind:"error", request_id, code, retry,
//! message, field}` plus `because`, `current_generation`, `readback` when set and
//! `effect:"unknown"` for `effect_unknown`.

use hee4_contracts::bounds::MAX_TOKEN_BYTES;
use serde_json::{Value, json};

/// Every member an error frame may carry, in emission order: what `tools.inspect` digests as the
/// error schema descriptor.
pub const ERROR_MEMBERS: [&str; 10] = [
    "kind",
    "request_id",
    "code",
    "retry",
    "field",
    "message",
    "because",
    "current_generation",
    "readback",
    "effect",
];

/// Every refusal the socket answers with, one name each (Error and Refusal Map vocabulary;
/// `not_ready` is this slice's addition, see FLOW.md and the DC proposal).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Code {
    /// The line is not a JSON object, or a required member is absent or mistyped.
    InvalidArgument,
    /// The action is not in the catalogue.
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
    /// A request line was longer than the `socket.frame_bytes` budget; the connection closes after
    /// this frame.
    FrameTooLarge,
    /// The server already holds its cap of concurrent connections.
    TooManyConnections,
    /// The action is catalogued but its owner is not registered in this release (the registry
    /// miss in dispatch, `because` names the scope), or `service.*` cannot make its call
    /// (`because` names why).
    Unavailable,
    /// The precondition's generation is behind the resource's (`current_generation` is set).
    StaleGeneration,
    /// A page cursor from another boot or filter; start the listing again.
    ResyncRequired,
    /// A bound on work or storage was reached.
    ResourceExhausted,
    /// The effect could not be confirmed by readback (`readback` names the read that settles it).
    EffectUnknown,
}

impl Code {
    /// Every name, in declaration order (`ALL[c.ordinal()] == c`).
    pub const ALL: [Self; 17] = [
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
        Self::Unavailable,
        Self::StaleGeneration,
        Self::ResyncRequired,
        Self::ResourceExhausted,
        Self::EffectUnknown,
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
            Self::Unavailable => 12,
            Self::StaleGeneration => 13,
            Self::ResyncRequired => 14,
            Self::ResourceExhausted => 15,
            Self::EffectUnknown => 16,
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
            Self::Unavailable => "unavailable",
            Self::StaleGeneration => "stale_generation",
            Self::ResyncRequired => "resync_required",
            Self::ResourceExhausted => "resource_exhausted",
            Self::EffectUnknown => "effect_unknown",
        }
    }

    /// The retry class (Error map `Retry`).
    #[must_use]
    pub const fn retry(self) -> &'static str {
        match self {
            Self::NotReady
            | Self::NoRoute
            | Self::SlowConsumer
            | Self::TooManyConnections
            | Self::Unavailable => "after_condition",
            Self::Conflict | Self::EffectUnknown => "after_readback",
            Self::Internal | Self::ResourceExhausted => "same_exact_request",
            Self::InvalidArgument
            | Self::UnknownAction
            | Self::UnsupportedActionVersion
            | Self::NotFound
            | Self::Forbidden
            | Self::FrameTooLarge
            | Self::StaleGeneration
            | Self::ResyncRequired => "never",
        }
    }
}

/// A refusal: its one name, the JSON pointer it is about, a static message, and the optional
/// members a few names carry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fault {
    /// The name.
    pub code: Code,
    /// JSON pointer into the request.
    pub field: &'static str,
    /// Detail (never an echo of request bytes beyond a refusal's own text).
    pub message: String,
    /// Why the condition holds (`unavailable`: the scope; `resync_required`: what moved).
    pub because: Option<&'static str>,
    /// `stale_generation`: the resource's generation now.
    pub current_generation: Option<u64>,
    /// `effect_unknown`: the read that settles the effect.
    pub readback: Option<&'static str>,
}

impl Fault {
    /// A fault at `field`, with no optional member.
    #[must_use]
    pub fn new(code: Code, field: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            field,
            message: message.into(),
            because: None,
            current_generation: None,
            readback: None,
        }
    }

    /// With `because`.
    #[must_use]
    pub const fn with_because(mut self, because: &'static str) -> Self {
        self.because = Some(because);
        self
    }

    /// With `current_generation`.
    #[must_use]
    pub const fn with_generation(mut self, generation: u64) -> Self {
        self.current_generation = Some(generation);
        self
    }

    /// With `readback`.
    #[must_use]
    pub const fn with_readback(mut self, readback: &'static str) -> Self {
        self.readback = Some(readback);
        self
    }
}

/// The envelope's `precondition`: the generation a mutating request was formed against.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Precondition {
    /// `task | thread | roster | service | analysis`.
    pub resource: String,
    /// The resource's id.
    pub id: String,
    /// The generation the caller read.
    pub generation: u64,
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
    /// Present when the action requires one (`catalogue::PreconditionRule::Required`).
    pub precondition: Option<Precondition>,
    /// The action's body.
    pub body: Value,
}

/// A token member: non-empty and at most [`MAX_TOKEN_BYTES`] bytes.
fn is_token(s: &str) -> bool {
    !s.is_empty() && s.len() <= MAX_TOKEN_BYTES
}

/// The refusal message for a token member.
fn token_refusal() -> String {
    format!("non-empty string of at most {MAX_TOKEN_BYTES} bytes")
}

/// `/precondition`: absent or null is `None`; an object with exactly `resource` (string), `id`
/// (string) and `generation` (unsigned integer) is `Some`; any other shape is refused at
/// `/precondition`, and a `resource` or `id` that is not a token at its own pointer.
fn precondition_of(v: Option<&Value>) -> Result<Option<Precondition>, (&'static str, String)> {
    let shape = |msg: &str| ("/precondition", msg.to_owned());
    let obj = match v {
        None | Some(Value::Null) => return Ok(None),
        Some(Value::Object(obj)) => obj,
        Some(_) => return Err(shape("object or null")),
    };
    if obj.len() != 3 {
        return Err(shape("exactly resource, id and generation"));
    }
    let (Some(Value::String(resource)), Some(Value::String(id)), Some(generation)) = (
        obj.get("resource"),
        obj.get("id"),
        obj.get("generation").and_then(Value::as_u64),
    ) else {
        return Err(shape(
            "resource: string, id: string, generation: unsigned integer",
        ));
    };
    if !is_token(resource) {
        return Err(("/precondition/resource", token_refusal()));
    }
    if !is_token(id) {
        return Err(("/precondition/id", token_refusal()));
    }
    Ok(Some(Precondition {
        resource: resource.clone(),
        id: id.clone(),
        generation,
    }))
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
        Some(Value::String(s)) if is_token(s) => Some(s.clone()),
        Some(_) => return Err(bad("/idempotency_key", &token_refusal())),
    };
    let precondition =
        precondition_of(obj.get("precondition")).map_err(|(field, msg)| bad(field, &msg))?;
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
        precondition,
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

/// An error frame: the six fixed members, then `because`, `current_generation` and `readback`
/// only when set, and `effect: "unknown"` only for [`Code::EffectUnknown`].
#[must_use]
pub fn error(request_id: &str, fault: &Fault) -> Value {
    let mut v = json!({
        "kind": "error",
        "request_id": request_id,
        "code": fault.code.name(),
        "retry": fault.code.retry(),
        "field": fault.field,
        "message": fault.message,
    });
    if let Some(because) = fault.because {
        v["because"] = json!(because);
    }
    if let Some(generation) = fault.current_generation {
        v["current_generation"] = json!(generation);
    }
    if let Some(readback) = fault.readback {
        v["readback"] = json!(readback);
    }
    if fault.code == Code::EffectUnknown {
        v["effect"] = json!("unknown");
    }
    v
}

/// A stream's close frame: the last line before the server closes the connection.
#[must_use]
pub fn close(code: Code, message: &str) -> Value {
    json!({"kind": "close", "code": code.name(), "retry": code.retry(), "message": message})
}

/// A request frame (the CLI's half), with no precondition.
#[must_use]
pub fn request(
    request_id: &str,
    action: &str,
    idempotency_key: Option<&str>,
    body: Value,
) -> Value {
    request_with(request_id, action, idempotency_key, body, None)
}

/// A request frame carrying `precondition` when given (the generic CLI's half).
#[must_use]
pub fn request_with(
    request_id: &str,
    action: &str,
    idempotency_key: Option<&str>,
    body: Value,
    precondition: Option<Value>,
) -> Value {
    let mut v = json!({
        "request_id": request_id,
        "action": action,
        "action_version": 1,
        "idempotency_key": idempotency_key,
    });
    if let Some(p) = precondition {
        v["precondition"] = p;
    }
    v["body"] = body;
    v
}

#[cfg(test)]
mod tests {
    use super::{
        Code, ERROR_MEMBERS, Fault, MAX_TOKEN_BYTES, Precondition, error, parse, request_with,
    };
    use serde_json::json;
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

    /// One refusal the source emits: the code's wire name, its field and its `because` when
    /// each is a literal at the `Fault::new` site or at a caller of the one helper around it.
    #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
    struct Emission {
        code: String,
        field: Option<String>,
        because: Option<String>,
    }

    /// Every `.rs` file under `src/`, cut at its `#[cfg(test)]`, doc and line comments dropped;
    /// none without `CARGO_MANIFEST_DIR` (the caller's resync guard then fails).
    fn crate_sources() -> Vec<(String, String)> {
        let mut out = Vec::new();
        // Read at run time: a baked manifest path names a deleted export (no_baked_paths).
        let Some(root) = std::env::var_os("CARGO_MANIFEST_DIR") else {
            return out;
        };
        let mut dirs = vec![std::path::PathBuf::from(root).join("src")];
        while let Some(dir) = dirs.pop() {
            let Ok(entries) = std::fs::read_dir(&dir) else {
                continue;
            };
            for path in entries.filter_map(Result::ok).map(|e| e.path()) {
                if path.is_dir() {
                    dirs.push(path);
                } else if path.extension().is_some_and(|x| x == "rs")
                    && let Ok(text) = std::fs::read_to_string(&path)
                {
                    let live = text.split("#[cfg(test)]").next().unwrap_or("");
                    let code: Vec<&str> = live
                        .lines()
                        .filter(|l| !l.trim_start().starts_with("//"))
                        .collect();
                    out.push((path.display().to_string(), code.join("\n")));
                }
            }
        }
        out
    }

    /// The index of the `)` that closes the `(` at `open`, skipping string literals.
    fn close_of(s: &str, open: usize) -> Option<usize> {
        let (mut depth, mut in_str, mut escaped) = (0_i32, false, false);
        for (i, c) in s.char_indices().skip_while(|(i, _)| *i < open) {
            if in_str {
                match (escaped, c) {
                    (true, _) => escaped = false,
                    (false, '\\') => escaped = true,
                    (false, '"') => in_str = false,
                    _ => {}
                }
                continue;
            }
            match c {
                '"' => in_str = true,
                '(' | '[' | '{' => depth += 1,
                ')' | ']' | '}' => {
                    depth -= 1;
                    if depth == 0 {
                        return Some(i);
                    }
                }
                _ => {}
            }
        }
        None
    }

    /// `s` split at its top-level commas (string literals and brackets respected), trimmed.
    fn args_of(s: &str) -> Vec<String> {
        let (mut out, mut cur) = (Vec::new(), String::new());
        let (mut depth, mut in_str, mut escaped) = (0_i32, false, false);
        for c in s.chars() {
            if in_str {
                match (escaped, c) {
                    (true, _) => escaped = false,
                    (false, '\\') => escaped = true,
                    (false, '"') => in_str = false,
                    _ => {}
                }
            } else {
                match c {
                    '"' => in_str = true,
                    '(' | '[' | '{' | '<' => depth += 1,
                    ')' | ']' | '}' | '>' => depth -= 1,
                    ',' if depth == 0 => {
                        out.push(cur.trim().to_owned());
                        cur.clear();
                        continue;
                    }
                    _ => {}
                }
            }
            cur.push(c);
        }
        if !cur.trim().is_empty() {
            out.push(cur.trim().to_owned());
        }
        out
    }

    /// The text of a plain string literal `"..."` (no escapes, no format), else `None`.
    fn literal(arg: &str) -> Option<String> {
        let inner = arg.strip_prefix('"')?.strip_suffix('"')?;
        (!inner.contains(['"', '\\'])).then(|| inner.to_owned())
    }

    fn is_ident(s: &str) -> bool {
        !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
    }

    /// The helper (`let NAME = |params|` or `fn NAME(params)`) defined last before `at`:
    /// `(name, parameter names)`.
    fn helper_before(src: &str, at: usize) -> Option<(String, Vec<String>)> {
        let head = &src[..at];
        let closure = head.rfind(" = |").and_then(|eq| {
            let name = head[..eq].rsplit("let ").next()?.trim();
            let params = head[eq + 4..].split('|').next()?;
            Some((eq, name.to_owned(), params.to_owned()))
        });
        let func = head.rfind("fn ").and_then(|f| {
            let rest = &head[f + 3..];
            let paren = rest.find('(')?;
            let close = close_of(rest, paren)?;
            Some((
                f,
                rest[..paren].to_owned(),
                rest[paren + 1..close].to_owned(),
            ))
        });
        let (_, name, params) = match (closure, func) {
            (Some(c), Some(f)) => {
                if c.0 > f.0 {
                    c
                } else {
                    f
                }
            }
            (c, f) => c.or(f)?,
        };
        let names = args_of(&params)
            .iter()
            .map(|p| {
                p.split(':')
                    .next()
                    .unwrap_or("")
                    .trim()
                    .trim_start_matches("mut ")
                    .to_owned()
            })
            .collect();
        is_ident(name.trim()).then(|| (name.trim().to_owned(), names))
    }

    /// The argument lists of every call `name(..)` in `src` (not its definition).
    fn calls_of(src: &str, name: &str) -> Vec<Vec<String>> {
        let needle = format!("{name}(");
        let mut out = Vec::new();
        for (at, _) in src.match_indices(&needle) {
            let before = src[..at].chars().next_back();
            if before.is_some_and(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.')
                || src[..at].ends_with("fn ")
            {
                continue;
            }
            let open = at + name.len();
            if let Some(close) = close_of(src, open) {
                out.push(args_of(&src[open + 1..close]));
            }
        }
        out
    }

    /// Every refusal the crate's live source emits through `Fault::new`, with its field and
    /// `because` resolved to literals where the site or one helper level holds them.
    fn emissions() -> std::collections::BTreeSet<Emission> {
        let mut out = std::collections::BTreeSet::new();
        for (_, src) in crate_sources() {
            for (at, _) in src.match_indices("Fault::new(") {
                let open = at + "Fault::new".len();
                let Some(close) = close_of(&src, open) else {
                    continue;
                };
                let args = args_of(&src[open + 1..close]);
                let Some(code) = args
                    .first()
                    .and_then(|c| c.rsplit("Code::").next())
                    .filter(|c| is_ident(c))
                else {
                    continue;
                };
                let Some(variant) = Code::ALL.iter().find(|v| format!("{v:?}") == code) else {
                    continue;
                };
                let field = args.get(1).cloned().unwrap_or_default();
                let mut because = None;
                let mut rest = close + 1;
                loop {
                    let tail = src[rest..].trim_start();
                    let skipped = src.len() - rest - tail.len();
                    let Some(m) = tail.strip_prefix(".with_") else {
                        break;
                    };
                    let Some(paren) = m.find('(') else { break };
                    let start = rest + skipped + ".with_".len() + paren;
                    let Some(end) = close_of(&src, start) else {
                        break;
                    };
                    if m.starts_with("because(") {
                        because = Some(src[start + 1..end].trim().to_owned());
                    }
                    rest = end + 1;
                }
                let lit = |a: &str| literal(a);
                let field_lit = lit(&field);
                let because_lit = because.as_deref().and_then(lit);
                let needs =
                    |a: Option<&str>, l: &Option<String>| l.is_none() && a.is_some_and(is_ident);
                let helper = (needs(Some(&field), &field_lit)
                    || needs(because.as_deref(), &because_lit))
                .then(|| helper_before(&src, at))
                .flatten();
                let name = variant.name().to_owned();
                let Some((helper, params)) = helper else {
                    out.insert(Emission {
                        code: name,
                        field: field_lit,
                        because: because_lit,
                    });
                    continue;
                };
                let pos = |a: Option<&str>| a.and_then(|a| params.iter().position(|p| p == a));
                let (fi, bi) = (pos(Some(&field)), pos(because.as_deref()));
                for call in calls_of(&src, &helper) {
                    let from = |i: Option<usize>, own: &Option<String>| {
                        i.and_then(|i| call.get(i))
                            .and_then(|a| literal(a))
                            .or_else(|| own.clone())
                    };
                    out.insert(Emission {
                        code: name.clone(),
                        field: from(fi, &field_lit),
                        because: from(bi, &because_lit),
                    });
                }
            }
        }
        out
    }

    /// The FLOW.md "Refusal names" row of `code`, whole.
    fn flow_row(code: &str) -> Option<String> {
        let flow = include_str!("../FLOW.md");
        flow.lines()
            .find(|l| l.starts_with(&format!("| `{code}` |")))
            .map(str::to_owned)
    }

    /// Triple parity: every `(code, field, because)` the source emits (literal at the site or
    /// one helper level up) appears in that code's FLOW.md row: the field as `` `field` ``, the
    /// `because` text verbatim. A field or `because` the scan cannot resolve to a literal is
    /// not checked; the resync triple below proves the helper resolution runs.
    #[test]
    fn every_emitted_triple_appears_in_its_flow_row() {
        let found = emissions();
        assert!(
            found.contains(&Emission {
                code: "resync_required".into(),
                field: Some("/body/since_seq".into()),
                because: Some("future_sequence".into()),
            }),
            "the scan no longer resolves helper arguments: {found:?}"
        );
        let mut missing = Vec::new();
        for e in &found {
            let Some(row) = flow_row(&e.code) else {
                missing.push(format!("{}: no row", e.code));
                continue;
            };
            if let Some(f) = &e.field
                && !row.contains(&format!("`{f}`"))
            {
                missing.push(format!("{}: field `{f}`", e.code));
            }
            if let Some(b) = &e.because
                && !row.contains(b.as_str())
            {
                missing.push(format!("{}: because {b:?}", e.code));
            }
        }
        assert!(missing.is_empty(), "FLOW.md rows miss: {missing:#?}");
    }

    #[test]
    fn precondition_member_is_parsed_or_refused_by_shape() {
        let line = |p: serde_json::Value| {
            let mut v = request_with("r", "a", None, json!({}), None);
            v["precondition"] = p;
            v.to_string()
        };
        let ok = parse(&line(
            json!({"resource": "roster", "id": "x", "generation": 3}),
        ));
        assert_eq!(
            ok.map(|r| r.precondition),
            Ok(Some(Precondition {
                resource: "roster".into(),
                id: "x".into(),
                generation: 3
            }))
        );
        assert_eq!(parse(&line(json!(null))).map(|r| r.precondition), Ok(None));
        let absent = request_with("r", "a", None, json!({}), None).to_string();
        assert_eq!(parse(&absent).map(|r| r.precondition), Ok(None));
        for bad in [
            json!(5),
            json!("roster"),
            json!({"resource": "roster"}),
            json!({"resource": "roster", "id": "x", "generation": -1}),
            json!({"resource": "roster", "id": "x", "generation": 1, "extra": 0}),
        ] {
            let parsed = parse(&line(bad.clone()));
            assert!(
                matches!(&parsed, Err((id, f)) if id == "r" && f.code == Code::InvalidArgument && f.field == "/precondition"),
                "{bad}: {parsed:?}"
            );
        }
    }

    /// `idempotency_key`, `precondition.resource` and `precondition.id` are tokens: empty or
    /// over `MAX_TOKEN_BYTES` is `invalid_argument` at the member's own pointer; exactly
    /// `MAX_TOKEN_BYTES` is accepted.
    #[test]
    fn token_members_are_bounded_at_their_pointers() {
        let max = "k".repeat(MAX_TOKEN_BYTES);
        let over = "k".repeat(MAX_TOKEN_BYTES + 1);
        let refused_at = |line: String, at: &str| {
            let parsed = parse(&line);
            assert!(
                matches!(&parsed, Err((id, f)) if id == "r" && f.code == Code::InvalidArgument && f.field == at),
                "{at}: {parsed:?}"
            );
        };
        let keyed = |k: &str| request_with("r", "a", Some(k), json!({}), None).to_string();
        assert_eq!(
            parse(&keyed(&max)).map(|r| r.idempotency_key),
            Ok(Some(max.clone()))
        );
        refused_at(keyed(&over), "/idempotency_key");
        refused_at(keyed(""), "/idempotency_key");
        let pre = |resource: &str, id: &str| {
            request_with(
                "r",
                "a",
                None,
                json!({}),
                Some(json!({"resource": resource, "id": id, "generation": 1})),
            )
            .to_string()
        };
        assert!(parse(&pre(&max, &max)).is_ok());
        refused_at(pre("", "x"), "/precondition/resource");
        refused_at(pre(&over, "x"), "/precondition/resource");
        refused_at(pre("roster", ""), "/precondition/id");
        refused_at(pre("roster", &over), "/precondition/id");
    }

    #[test]
    fn error_frame_carries_optional_members_only_when_set() {
        let plain = error("r", &Fault::new(Code::NotFound, "/body/task_id", "no"));
        assert!(plain.get("because").is_none());
        assert!(plain.get("current_generation").is_none());
        assert!(plain.get("readback").is_none());
        assert!(plain.get("effect").is_none());
        let full = error(
            "r",
            &Fault::new(Code::EffectUnknown, "/", "unconfirmed")
                .with_because("why")
                .with_generation(7)
                .with_readback("service.inspect"),
        );
        assert_eq!(full["because"], "why");
        assert_eq!(full["current_generation"], 7);
        assert_eq!(full["readback"], "service.inspect");
        assert_eq!(full["effect"], "unknown");
        let keys: Vec<&str> = full
            .as_object()
            .map(|o| o.keys().map(String::as_str).collect())
            .unwrap_or_default();
        for k in keys {
            assert!(ERROR_MEMBERS.contains(&k), "{k} not in ERROR_MEMBERS");
        }
    }
}
