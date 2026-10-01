//! The task module's control bodies: `task.submit`'s `TaskSpecV1`, `task.get`'s request and
//! `task.cancel`'s reason, read under the RC01 offline, zero-external-spend profile (RC03 §4; RC01
//! numeric policy).
//!
//! RC03 §1: "The receiving module owns its action body." These are the rules only the task module
//! can decide; the envelope, the grant and the catalogue door have already run. Every refusal names
//! its member and the rule, in static text.
//!
//! The RC01 overlay, stated once here and nowhere else:
//! * one admitted class, `rust-library-change/1` ("The first release supports
//!   `rust-library-change/1`"); free text cannot create a class;
//! * `privacy` must be `local_only` and `currency_microunits` `"0"`: the remote and paid profiles
//!   are reserved in the vocabulary and `unavailable` under RC01;
//! * no time limit is created here: the caller's `wall_ms` budget passes through, its verification
//!   share is [`crate::task::CLEANUP_RESERVE`] (RC01's policy, owned by the task loop guard, never
//!   more than the budget itself), and the ledger's own allocation rule decides what it admits;
//! * `tokens` must be `"0"`: the ledger has no token column yet (review D-C1), and an allocation the
//!   engine cannot enforce is not admitted;
//! * `parent` must be `null`: child allocations belong to cohort composition, not composed here.

use crate::contracts::control::{
    CancelReason, Disposition, ErrorCode, EvidenceRef, EvidenceView, Fault, PageIn, Retry,
    request_sha256,
};
use crate::contracts::{UuidV4, parse_u64_decimal};
use crate::recovery::TaskState;
use serde_json::{Map, Value, json};

/// The task classes the RC01 profile admits.
pub const ADMITTED_CLASSES: [&str; 1] = ["rust-library-change/1"];
const MAX_INTENT_BYTES: usize = 8192;
const MAX_CRITERION_BYTES: usize = 1024;
const MAX_CRITERIA: usize = 64;

/// The admission a valid `TaskSpecV1` asks for, in the ledger's terms.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Spec {
    /// The admitted class.
    pub task_class: &'static str,
    /// The acceptance criteria, in order.
    pub criteria: Vec<String>,
    /// The whole wall-time allocation.
    pub limit_ms: u64,
    /// The share before the verification reserve.
    pub work_ms: u64,
    /// The verification and cleanup reserve.
    pub verify_ms: u64,
    /// The installed workspace the task names (a validated `UuidV4` text; B14-P2c binds it at
    /// admission).
    pub workspace_id: String,
}

fn text<'a>(
    members: &'a Map<String, Value>,
    name: &str,
    field: &'static str,
    most: usize,
) -> Result<&'a str, Fault> {
    members
        .get(name)
        .and_then(Value::as_str)
        .filter(|value| (1..=most).contains(&value.len()))
        .ok_or(Fault::invalid(field, "UTF-8 within its byte bound"))
}

fn object<'a>(
    value: Option<&'a Value>,
    field: &'static str,
) -> Result<&'a Map<String, Value>, Fault> {
    match value {
        Some(Value::Object(members)) => Ok(members),
        _ => Err(Fault::invalid(field, "object")),
    }
}

fn exactly(members: &Map<String, Value>, names: &[&str], field: &'static str) -> Result<(), Fault> {
    if members.len() == names.len() && names.iter().all(|name| members.contains_key(*name)) {
        Ok(())
    } else {
        Err(Fault::invalid(field, "exactly its declared members"))
    }
}

fn unavailable(field: &'static str, constraint: &'static str) -> Fault {
    Fault::of(
        ErrorCode::Unavailable,
        Retry::AfterCondition,
        "the RC01 offline, zero-external-spend profile does not admit this",
    )
    .at(field)
    .because(constraint)
}

/// Read `task.submit`'s body.
///
/// # Errors
///
/// `invalid_argument` naming the member for a malformed spec; `unavailable` for a profile RC01
/// reserves but does not admit (remote privacy, currency, tokens, a parent allocation).
pub fn submission(body: &Map<String, Value>) -> Result<Spec, Fault> {
    exactly(body, &["spec"], "/body")?;
    spec(body.get("spec"))
}

/// A valid `task.preview` body: the spec, read by the one door submission reads it through, and
/// the catalogue revision the caller planned against (the receiver compares it with its own).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Preview {
    /// The spec, validated exactly as `task.submit` validates it.
    pub spec: Spec,
    /// The `catalogue_revision` the caller names.
    pub catalogue_revision: u64,
}

/// Read `task.preview`'s body (B07).
///
/// # Errors
///
/// `invalid_argument` naming the member: `/body` unless exactly `spec`, `brief_revision` and
/// `catalogue_revision`; every refusal of [`submission`]'s spec, at the same member with the same
/// code; `/body/brief_revision` unless the `U64Decimal` `"0"` — RC01 admits only `parent: null`, so
/// there is no brief (a composed parent's rule will be: equal to `spec.parent.brief_revision`, else
/// `resync_required`); `/body/catalogue_revision` unless a `U64Decimal`.
pub fn preview(body: &Map<String, Value>) -> Result<Preview, Fault> {
    exactly(
        body,
        &["spec", "brief_revision", "catalogue_revision"],
        "/body",
    )?;
    let spec = spec(body.get("spec"))?;
    let decimal = |name: &str, field: &'static str| {
        body.get(name)
            .and_then(Value::as_str)
            .and_then(|text| parse_u64_decimal(text).ok())
            .ok_or(Fault::invalid(field, "U64Decimal"))
    };
    if decimal("brief_revision", "/body/brief_revision")? != 0 {
        return Err(Fault::invalid(
            "/body/brief_revision",
            "0 when the spec names no parent",
        ));
    }
    Ok(Preview {
        spec,
        catalogue_revision: decimal("catalogue_revision", "/body/catalogue_revision")?,
    })
}

/// The one reader of a `TaskSpecV1` under the RC01 overlay, for every action that carries one.
fn spec(value: Option<&Value>) -> Result<Spec, Fault> {
    let spec = object(value, "/body/spec")?;
    exactly(
        spec,
        &[
            "task_class",
            "intent",
            "criteria",
            "privacy",
            "workspace_id",
            "budget",
            "parent",
        ],
        "/body/spec",
    )?;
    let class = spec
        .get("task_class")
        .and_then(Value::as_str)
        .filter(|class| (1..=64).contains(&class.len()) && class.is_ascii())
        .ok_or(Fault::invalid(
            "/body/spec/task_class",
            "ASCII of 1..64 bytes",
        ))?;
    let task_class = ADMITTED_CLASSES
        .into_iter()
        .find(|admitted| *admitted == class)
        .ok_or(Fault::invalid(
            "/body/spec/task_class",
            "a class in the admitted catalogue",
        ))?;
    text(spec, "intent", "/body/spec/intent", MAX_INTENT_BYTES)?;
    let Some(Value::Array(rows)) = spec.get("criteria") else {
        return Err(Fault::invalid(
            "/body/spec/criteria",
            "array of 1..64 criteria",
        ));
    };
    if !(1..=MAX_CRITERIA).contains(&rows.len()) {
        return Err(Fault::invalid(
            "/body/spec/criteria",
            "array of 1..64 criteria",
        ));
    }
    let criteria = rows
        .iter()
        .map(|row| {
            row.as_str()
                .filter(|criterion| (1..=MAX_CRITERION_BYTES).contains(&criterion.len()))
                .map(str::to_owned)
                .ok_or(Fault::invalid(
                    "/body/spec/criteria",
                    "each criterion UTF-8 of 1..1024 bytes",
                ))
        })
        .collect::<Result<Vec<_>, _>>()?;
    match spec.get("privacy").and_then(Value::as_str) {
        Some("local_only") => {}
        Some("remote_allowed") => {
            return Err(unavailable("/body/spec/privacy", "local_only under RC01"));
        }
        _ => {
            return Err(Fault::invalid(
                "/body/spec/privacy",
                "local_only or remote_allowed",
            ));
        }
    }
    let workspace_id = spec
        .get("workspace_id")
        .and_then(Value::as_str)
        .filter(|id| UuidV4::parse(id).is_ok())
        .ok_or(Fault::invalid("/body/spec/workspace_id", "UuidV4"))?
        .to_owned();
    let (limit_ms, work_ms, verify_ms) = budget(object(spec.get("budget"), "/body/spec/budget")?)?;
    match spec.get("parent") {
        Some(Value::Null) => {}
        Some(Value::Object(_)) => {
            return Err(unavailable(
                "/body/spec/parent",
                "child allocations are not composed behind this receiver",
            ));
        }
        _ => {
            return Err(Fault::invalid(
                "/body/spec/parent",
                "null or a parent allocation",
            ));
        }
    }
    Ok(Spec {
        task_class,
        criteria,
        limit_ms,
        work_ms,
        verify_ms,
        workspace_id,
    })
}

fn budget(budget: &Map<String, Value>) -> Result<(u64, u64, u64), Fault> {
    exactly(
        budget,
        &["mode", "wall_ms", "tokens", "currency_microunits"],
        "/body/spec/budget",
    )?;
    if !matches!(
        budget.get("mode").and_then(Value::as_str),
        Some("hard" | "conservative")
    ) {
        return Err(Fault::invalid(
            "/body/spec/budget/mode",
            "hard or conservative",
        ));
    }
    let decimal = |name: &str, field: &'static str| {
        budget
            .get(name)
            .and_then(Value::as_str)
            .and_then(|text| parse_u64_decimal(text).ok())
            .ok_or(Fault::invalid(field, "U64Decimal"))
    };
    let wall = decimal("wall_ms", "/body/spec/budget/wall_ms")?;
    let tokens = decimal("tokens", "/body/spec/budget/tokens")?;
    let currency = decimal(
        "currency_microunits",
        "/body/spec/budget/currency_microunits",
    )?;
    if currency != 0 {
        return Err(unavailable(
            "/body/spec/budget/currency_microunits",
            "zero external spend under RC01",
        ));
    }
    if tokens != 0 {
        return Err(unavailable(
            "/body/spec/budget/tokens",
            "token budgets are not yet enforced by the ledger (D-C1)",
        ));
    }
    // The verification share is the task loop's existing reserve, never more than the budget.
    let reserve = u64::try_from(crate::task::CLEANUP_RESERVE.as_millis())
        .unwrap_or(u64::MAX)
        .min(wall);
    Ok((wall, wall - reserve, reserve))
}

/// `note` is bounded in UTF-8 bytes; the schema's `maxLength` counts code points, so the byte rule
/// is decided here.
const MAX_NOTE_BYTES: usize = 1024;

/// A valid `task.cancel` body. The task and the generation it expects are the precondition's, not
/// the body's.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Cancel {
    /// Why (the closed vocabulary, [`CancelReason`]).
    pub reason: CancelReason,
    /// The caller's note, at most 1,024 UTF-8 bytes.
    pub note: Option<String>,
}

/// Read `task.cancel`'s body.
///
/// # Errors
///
/// `invalid_argument` naming the member: `/body` unless the members are exactly `reason` and
/// `note`; `/body/reason` outside [`CancelReason`]; `/body/note` unless null or a string of at
/// most 1,024 bytes.
pub fn cancel(body: &Map<String, Value>) -> Result<Cancel, Fault> {
    exactly(body, &["reason", "note"], "/body")?;
    let reason = body
        .get("reason")
        .and_then(Value::as_str)
        .and_then(CancelReason::parse)
        .ok_or(Fault::invalid(
            "/body/reason",
            "operator_request, superseded, budget, deadline or safety",
        ))?;
    let note = match body.get("note") {
        Some(Value::Null) => None,
        Some(Value::String(note)) if note.len() <= MAX_NOTE_BYTES => Some(note.clone()),
        _ => {
            return Err(Fault::invalid(
                "/body/note",
                "null or UTF-8 within 1,024 bytes",
            ));
        }
    };
    Ok(Cancel { reason, note })
}

/// The most states a `task.list` may select (`TaskStateV1[0..12]`: every state once).
const MAX_LIST_STATES: usize = 12;
/// The longest class a `task.list` may select by (`ASCII[1..64]`).
const MAX_CLASS_BYTES: usize = 64;

/// A valid `task.list` body (contract-decisions.md:342).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct List {
    /// The task states selected, each once; empty selects every state.
    pub states: Vec<TaskState>,
    /// The admitted class selected, if any.
    pub task_class: Option<String>,
    /// The parent selected, if any.
    pub parent_task_id: Option<String>,
    /// The page asked for.
    pub page: PageIn,
}

impl List {
    /// The digest a `task.list` cursor binds its filter by: SHA-256 over the server-produced compact
    /// JSON `{"parent_task_id":…,"states":[…],"task_class":…}`, keys in byte order, the states sorted
    /// by their names' bytes, no LF -- derived from the validated values, never from caller text
    /// (RC03 §4's rule for event filters, applied to this listing as `tools.list` applies it).
    #[must_use]
    pub fn filter_sha256(&self) -> String {
        let mut states: Vec<&str> = self.states.iter().map(|state| state.name()).collect();
        states.sort_unstable();
        let form = json!({
            "parent_task_id": self.parent_task_id,
            "states": states,
            "task_class": self.task_class,
        });
        request_sha256(form.to_string().as_bytes())
    }
}

/// Read `task.list`'s body.
///
/// # Errors
///
/// `invalid_argument` naming the member: `/body` unless exactly `states`, `task_class`,
/// `parent_task_id` and `page`; `/body/states` unless a duplicate-free array of at most 12 task
/// states; `/body/task_class` unless null or ASCII of 1..64 bytes; `/body/parent_task_id` unless
/// null or a `UuidV4`; the page by [`PageIn::parse`].
pub fn list(body: &Map<String, Value>) -> Result<List, Fault> {
    exactly(
        body,
        &["states", "task_class", "parent_task_id", "page"],
        "/body",
    )?;
    let refused = || {
        Fault::invalid(
            "/body/states",
            "a duplicate-free array of at most 12 task states",
        )
    };
    let Some(Value::Array(named)) = body.get("states") else {
        return Err(refused());
    };
    if named.len() > MAX_LIST_STATES {
        return Err(refused());
    }
    let mut states = Vec::with_capacity(named.len());
    for name in named {
        let state = name
            .as_str()
            .and_then(TaskState::parse)
            .filter(|state| !states.contains(state))
            .ok_or_else(refused)?;
        states.push(state);
    }
    let task_class = match body.get("task_class") {
        Some(Value::Null) => None,
        Some(Value::String(class))
            if (1..=MAX_CLASS_BYTES).contains(&class.len()) && class.is_ascii() =>
        {
            Some(class.clone())
        }
        _ => {
            return Err(Fault::invalid(
                "/body/task_class",
                "null or ASCII of 1..64 bytes",
            ));
        }
    };
    let parent_task_id = match body.get("parent_task_id") {
        Some(Value::Null) => None,
        Some(Value::String(parent)) if UuidV4::parse(parent).is_ok() => Some(parent.clone()),
        _ => return Err(Fault::invalid("/body/parent_task_id", "null or UuidV4")),
    };
    Ok(List {
        states,
        task_class,
        parent_task_id,
        page: PageIn::parse(body.get("page"))?,
    })
}

/// The longest disposition reason, in UTF-8 bytes (`UTF8[1..2048]`).
const MAX_REASON_BYTES: usize = 2048;
/// The most evidence references a request may carry (RC03: evidence arrays <= 64).
const MAX_EVIDENCE: usize = 64;

/// A valid `task.resolve` body (contract-decisions.md:344). The task and the generation it expects
/// are the precondition's.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Resolve {
    /// The obligation decided: an attempt, or a delivery event.
    pub obligation_id: String,
    /// What was decided.
    pub disposition: Disposition,
    /// Why, as the operator states it (1..2048 UTF-8 bytes).
    pub reason: String,
    /// What the operator relies on: references to artifacts the ledger holds.
    pub evidence: Vec<EvidenceRef>,
}

/// Read `task.resolve`'s body.
///
/// # Errors
///
/// `invalid_argument` naming the member: `/body` unless exactly `obligation_id`, `disposition`,
/// `reason` and `evidence`; `/body/obligation_id` unless a `UuidV4`; `/body/disposition` outside
/// [`Disposition`]; `/body/reason` unless 1..2048 UTF-8 bytes; `/body/evidence` unless an array of
/// at most 64 `EvidenceRefV1`, each exactly its five members, well formed.
pub fn resolve(body: &Map<String, Value>) -> Result<Resolve, Fault> {
    exactly(
        body,
        &["obligation_id", "disposition", "reason", "evidence"],
        "/body",
    )?;
    let obligation_id = body
        .get("obligation_id")
        .and_then(Value::as_str)
        .filter(|id| UuidV4::parse(id).is_ok())
        .ok_or(Fault::invalid("/body/obligation_id", "UuidV4"))?
        .to_owned();
    let disposition = body
        .get("disposition")
        .and_then(Value::as_str)
        .and_then(Disposition::parse)
        .ok_or(Fault::invalid(
            "/body/disposition",
            "retry, abandon, acknowledge_external_effect or quarantine",
        ))?;
    let reason = body
        .get("reason")
        .and_then(Value::as_str)
        .filter(|reason| (1..=MAX_REASON_BYTES).contains(&reason.len()))
        .ok_or(Fault::invalid("/body/reason", "UTF-8 of 1..2048 bytes"))?
        .to_owned();
    let refused = || Fault::invalid("/body/evidence", "at most 64 EvidenceRefV1");
    let Some(Value::Array(named)) = body.get("evidence") else {
        return Err(refused());
    };
    if named.len() > MAX_EVIDENCE {
        return Err(refused());
    }
    let evidence = named
        .iter()
        .map(|item| EvidenceRef::parse(item).ok_or_else(refused))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Resolve {
        obligation_id,
        disposition,
        reason,
        evidence,
    })
}

/// How `task.get` names its task.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Selector {
    /// By identity.
    Task(String),
    /// By the admission's idempotency key: the readback a lost `task.submit` reply needs.
    SubmitKey(String),
}

/// Read `task.get`'s body.
///
/// # Errors
///
/// `invalid_argument` naming the member. The evidence view is handed on: `none` reads no evidence,
/// `summary` and `refs` the views B09 composes.
pub fn get(body: &Map<String, Value>) -> Result<(Selector, Option<EvidenceView>), Fault> {
    exactly(body, &["selector", "evidence"], "/body")?;
    let view = match body.get("evidence").and_then(Value::as_str) {
        Some("none") => None,
        Some("summary") => Some(EvidenceView::Summary),
        Some("refs") => Some(EvidenceView::Refs),
        _ => return Err(Fault::invalid("/body/evidence", "none, summary or refs")),
    };
    Ok((selector(body)?, view))
}

fn selector(body: &Map<String, Value>) -> Result<Selector, Fault> {
    let selector = object(body.get("selector"), "/body/selector")?;
    let uuid = |name: &str, field: &'static str| {
        selector
            .get(name)
            .and_then(Value::as_str)
            .filter(|id| UuidV4::parse(id).is_ok())
            .map(str::to_owned)
            .ok_or(Fault::invalid(field, "UuidV4"))
    };
    if selector.len() == 1 && selector.contains_key("task_id") {
        return Ok(Selector::Task(uuid("task_id", "/body/selector/task_id")?));
    }
    exactly(
        selector,
        &["source_action", "idempotency_key"],
        "/body/selector",
    )?;
    if selector.get("source_action").and_then(Value::as_str) != Some("task.submit") {
        return Err(Fault::invalid(
            "/body/selector/source_action",
            "const task.submit",
        ));
    }
    Ok(Selector::SubmitKey(uuid(
        "idempotency_key",
        "/body/selector/idempotency_key",
    )?))
}
