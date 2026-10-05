//! The control catalogue as data (K0): the 22 action ids of the v4 release, each with its
//! owner, effect, scope, readback action and precondition rule. One home for the id list
//! (catalogue-data card §9 #2: no second list anywhere); pinned by [`revision`], a content
//! digest over canonical JSON.
//!
//! Types and data only: no I/O, no store. The owner registry (K6 `hee4-app`) decides which of
//! these ids is served; an id whose owner has no registry entry is refused `unavailable` with its
//! [`Scope::because`] text. [`Owner::Judge`] and [`Owner::Deploy`] are held: a registry refuses
//! them by construction.

use std::fmt;

use serde::Serialize;
use serde_json::{Value, json};

use crate::{Sha256Hex, canonical_json};

/// Who serves an action: the registry family an id is dispatched to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Owner {
    /// `health`.
    App,
    /// `tools.*`: the catalogue's own reads.
    Actions,
    /// `task.*`.
    Task,
    /// `events.subscribe`.
    Notify,
    /// `roster.*` (v4.1).
    Roster,
    /// `service.*` (v4.2).
    Service,
    /// `thread.*` (v4.2).
    Cohort,
    /// `analysis.*` (v4.2).
    Numerical,
    /// `judge.inspect`: held until H-8 (API Map, DC-24); never registered.
    Judge,
    /// Internal (API Map :89, :115): no catalogued id carries it; registering it is refused.
    Deploy,
}

impl Owner {
    /// The wire spelling (`snake_case`, the same as the `Serialize` form).
    #[must_use]
    pub const fn wire_name(self) -> &'static str {
        match self {
            Self::App => "app",
            Self::Actions => "actions",
            Self::Task => "task",
            Self::Notify => "notify",
            Self::Roster => "roster",
            Self::Service => "service",
            Self::Cohort => "cohort",
            Self::Numerical => "numerical",
            Self::Judge => "judge",
            Self::Deploy => "deploy",
        }
    }

    /// True for the owners no registry may carry: `Judge` (held, H-8) and `Deploy` (internal).
    #[must_use]
    pub const fn is_held(self) -> bool {
        matches!(self, Self::Judge | Self::Deploy)
    }
}

impl fmt::Display for Owner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.wire_name())
    }
}

/// What an action does to the engine (API Map "Effect" column).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Effect {
    /// One frame, no state change.
    Read,
    /// A stream of frames, no state change.
    ReadStream,
    /// Planning only: admits nothing.
    ReadOnlyPlanning,
    /// Creates a task row.
    DurableAdmission,
    /// Records a cancel intent.
    CancelIntent,
    /// Records an operator disposition.
    RecordDisposition,
    /// Changes configuration (roster).
    ConfigurationMutation,
    /// Runs a bounded probe and records it.
    BoundedProbe,
    /// Acts on a managed service's lifecycle.
    ManagedLifecycle,
    /// Runs a bounded numerical analysis.
    BoundedAnalysis,
}

impl Effect {
    /// True unless the effect is `Read`, `ReadStream` or `ReadOnlyPlanning` (API Map :29). What
    /// the idempotency key and `not_ready` gate and the K1 replay primitive derive from.
    #[must_use]
    pub const fn mutates(self) -> bool {
        !matches!(self, Self::Read | Self::ReadStream | Self::ReadOnlyPlanning)
    }

    /// The wire spelling (`snake_case`, the same as the `Serialize` form).
    #[must_use]
    pub const fn wire_name(self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::ReadStream => "read_stream",
            Self::ReadOnlyPlanning => "read_only_planning",
            Self::DurableAdmission => "durable_admission",
            Self::CancelIntent => "cancel_intent",
            Self::RecordDisposition => "record_disposition",
            Self::ConfigurationMutation => "configuration_mutation",
            Self::BoundedProbe => "bounded_probe",
            Self::ManagedLifecycle => "managed_lifecycle",
            Self::BoundedAnalysis => "bounded_analysis",
        }
    }
}

/// The release an action is served in. An id whose owner is not registered in the running
/// release is refused `unavailable` with [`Scope::because`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Scope {
    /// The walking skeleton.
    V40,
    /// The roster family.
    V41,
    /// The service, cohort and numerical families (service is composed in v4.0; cohort and
    /// numerical are not). The `because` text names no family: which families a binary composes
    /// is the registry's answer, never a constant (wave 9 found both old texts stale).
    V42,
    /// Held behind Luke's Engine Data Grant (H-8).
    Held,
}

impl Scope {
    /// The `because` text of the `unavailable` refusal: four distinct constants (Error map :66
    /// for v4.0; the Held one names H-8).
    #[must_use]
    pub const fn because(self) -> &'static str {
        match self {
            Self::V40 => "scope v4.0: owner not composed",
            Self::V41 => "scope v4.1: this action's family is not composed in this release",
            Self::V42 => "scope v4.2: this action's family is not composed in this release",
            Self::Held => "held: no registry entry until the Engine Data Grant (H-8)",
        }
    }

    /// The wire spelling (`snake_case`, the same as the `Serialize` form).
    #[must_use]
    pub const fn wire_name(self) -> &'static str {
        match self {
            Self::V40 => "v40",
            Self::V41 => "v41",
            Self::V42 => "v42",
            Self::Held => "held",
        }
    }
}

/// Whether a request must carry the `precondition` envelope member, and for which resource.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PreconditionRule {
    /// The envelope member is ignored.
    None,
    /// `precondition{resource: <this>, id, generation}` is required; absent is `invalid_argument`.
    Required(&'static str),
}

impl PreconditionRule {
    /// The JSON form used by [`revision`]: `null` or the resource name.
    #[must_use]
    pub fn to_json(self) -> Value {
        match self {
            Self::None => Value::Null,
            Self::Required(resource) => Value::String(resource.to_owned()),
        }
    }
}

/// One catalogue entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Action {
    /// The action id (`family.verb`, or `health`).
    pub id: &'static str,
    /// The contract version; 1 for every entry of this release.
    pub version: u32,
    /// One sentence, the first of the feature file's line 3.
    pub purpose: &'static str,
    /// What the action does.
    pub effect: Effect,
    /// Which registry family serves it.
    pub owner: Owner,
    /// The release it is served in.
    pub scope: Scope,
    /// The read a caller issues after a lost reply; `Some` exactly when the effect mutates.
    pub readback_action: Option<&'static str>,
    /// Whether the envelope's `precondition` is required.
    pub precondition: PreconditionRule,
    /// The request body's member names.
    pub request_fields: &'static [&'static str],
    /// The result body's member names.
    pub result_fields: &'static [&'static str],
}

impl Action {
    /// The entry as a JSON object (the shape [`revision`] digests and `tools.*` emit).
    #[must_use]
    pub fn to_json(&self) -> Value {
        json!({
            "id": self.id,
            "version": self.version,
            "purpose": self.purpose,
            "effect": self.effect.wire_name(),
            "owner": self.owner.wire_name(),
            "scope": self.scope.wire_name(),
            "readback_action": self.readback_action,
            "precondition": self.precondition.to_json(),
            "request_fields": self.request_fields,
            "result_fields": self.result_fields,
        })
    }
}

/// The 22 catalogued actions (API Map A-01..A-22; `gates/features/<id>.md` line 3). The one
/// literal count lives in this type; tests compare the id set to the feature directory.
pub const CATALOGUE: [Action; 22] = [
    Action {
        id: "health",
        version: 1,
        purpose: "The readiness read: one frame that reports protocol and engine version, whether the engine is ready, the recovery state, the ledger state and socket custody.",
        effect: Effect::Read,
        owner: Owner::App,
        scope: Scope::V40,
        readback_action: None,
        precondition: PreconditionRule::None,
        request_fields: &[],
        result_fields: &["ok", "head_sha", "recovery_complete", "uptime_s"],
    },
    Action {
        id: "tools.list",
        version: 1,
        purpose: "A paged listing of the control catalogue: every action id with its version, purpose and effect, under one catalogue_revision.",
        effect: Effect::Read,
        owner: Owner::Actions,
        scope: Scope::V40,
        readback_action: None,
        precondition: PreconditionRule::None,
        request_fields: &["query", "page"],
        result_fields: &["catalogue_revision", "page"],
    },
    Action {
        id: "tools.inspect",
        version: 1,
        purpose: "One catalogue entry in full: purpose, effect, the SHA-256 of its request, result and error schemas, its byte and deadline bounds, and the read action a caller uses after a lost reply.",
        effect: Effect::Read,
        owner: Owner::Actions,
        scope: Scope::V40,
        readback_action: None,
        precondition: PreconditionRule::None,
        request_fields: &["action", "version"],
        result_fields: &[
            "action",
            "version",
            "purpose",
            "effect",
            "request_schema_sha256",
            "result_schema_sha256",
            "error_schema_sha256",
            "max_request_bytes",
            "max_deadline_ms",
            "readback_action",
            "scope",
            "served",
        ],
    },
    Action {
        id: "task.preview",
        version: 1,
        purpose: "Read-only planning: for a TaskSpecV1 at a given brief and catalogue revision, which recipes are eligible, which are excluded and why, the cost mode, and the observation cutoff the answer rests on.",
        effect: Effect::ReadOnlyPlanning,
        owner: Owner::Task,
        scope: Scope::V40,
        readback_action: None,
        precondition: PreconditionRule::None,
        request_fields: &["brief"],
        result_fields: &["eligible", "model", "refusal", "message"],
    },
    Action {
        id: "task.submit",
        version: 1,
        purpose: "Durable admission: the one way a task row is created.",
        effect: Effect::DurableAdmission,
        owner: Owner::Task,
        scope: Scope::V40,
        readback_action: Some("task.get"),
        precondition: PreconditionRule::None,
        request_fields: &["brief"],
        result_fields: &["task_id", "phase"],
    },
    Action {
        id: "task.get",
        version: 1,
        purpose: "One task in full: its head, criteria digest, attempts, cleanup, delivery count, and (on request) evidence, selected by id or by the submit key.",
        effect: Effect::Read,
        owner: Owner::Task,
        scope: Scope::V40,
        readback_action: None,
        precondition: PreconditionRule::None,
        request_fields: &["task_id"],
        result_fields: &["task_id", "phase", "events", "last_receipt_hash"],
    },
    Action {
        id: "task.list",
        version: 1,
        purpose: "Keyset-paged task heads filtered by state, class and parent, with a snapshot-pinned cursor.",
        effect: Effect::Read,
        owner: Owner::Task,
        scope: Scope::V40,
        readback_action: None,
        precondition: PreconditionRule::None,
        request_fields: &[],
        result_fields: &["tasks"],
    },
    Action {
        id: "task.cancel",
        version: 1,
        purpose: "A cancel intent against a task at a known generation: for a non-waiting task the state moves to cancellation_requested; for the two waiting variants (blocked, effect_unknown) the cancel field is set.",
        effect: Effect::CancelIntent,
        owner: Owner::Task,
        scope: Scope::V40,
        readback_action: Some("task.get"),
        precondition: PreconditionRule::None,
        request_fields: &["task_id"],
        result_fields: &["task_id", "phase"],
    },
    Action {
        id: "task.resolve",
        version: 1,
        purpose: "An operator records a disposition against a task obligation: quarantine moves any non-terminal task to blocked; abandon closes it through the stop door as cancelled (if a cancel was requested) or abandoned.",
        effect: Effect::RecordDisposition,
        owner: Owner::Task,
        scope: Scope::V40,
        readback_action: Some("task.get"),
        precondition: PreconditionRule::None,
        request_fields: &["task_id", "resolution", "reason"],
        result_fields: &["task_id", "phase"],
    },
    Action {
        id: "events.subscribe",
        version: 1,
        purpose: "The only streaming action: a subscriber gives a cursor and topics, receives a bootstrap frame, then one ControlEventV1 frame per outbox row until it is caught up, parks on the publish wake, and continues; each delivered frame advances outbox.delivered through acknowledge_delivery, its first production caller (UM-P11).",
        effect: Effect::ReadStream,
        owner: Owner::Notify,
        scope: Scope::V40,
        readback_action: None,
        precondition: PreconditionRule::None,
        request_fields: &["since_seq"],
        result_fields: &["since_seq", "stream"],
    },
    Action {
        id: "roster.list",
        version: 1,
        purpose: "A paged listing of roster records (the agents, models and runtimes the engine may dispatch to) by kind, capability and locality, with or without disabled ones.",
        effect: Effect::Read,
        owner: Owner::Roster,
        scope: Scope::V41,
        readback_action: None,
        precondition: PreconditionRule::None,
        request_fields: &[
            "kinds",
            "capability",
            "locality",
            "include_disabled",
            "page",
        ],
        result_fields: &["page"],
    },
    Action {
        id: "roster.inspect",
        version: 1,
        purpose: "One roster record with its last operation, selected by id or by the submit key of a roster.update, the mandatory readback for a lost create.",
        effect: Effect::Read,
        owner: Owner::Roster,
        scope: Scope::V41,
        readback_action: None,
        precondition: PreconditionRule::None,
        request_fields: &["selector"],
        result_fields: &["record", "last_operation"],
    },
    Action {
        id: "roster.update",
        version: 1,
        purpose: "Create or revise a roster record with a definition and an audit reason, recorded as a revision and an operation.",
        effect: Effect::ConfigurationMutation,
        owner: Owner::Roster,
        scope: Scope::V41,
        readback_action: Some("roster.inspect"),
        precondition: PreconditionRule::None,
        request_fields: &["record_id", "definition", "audit_reason"],
        result_fields: &["record", "operation_id", "change"],
    },
    Action {
        id: "roster.disable",
        version: 1,
        purpose: "Disable a roster record with a policy for its active attempts: either let them finish or request their cancellation, recorded as an operation.",
        effect: Effect::ConfigurationMutation,
        owner: Owner::Roster,
        scope: Scope::V41,
        readback_action: Some("roster.inspect"),
        precondition: PreconditionRule::Required("roster"),
        request_fields: &["record_id", "active_attempt_policy", "audit_reason"],
        result_fields: &[
            "record",
            "operation_id",
            "active_attempts",
            "cancellation_obligations",
        ],
    },
    Action {
        id: "service.inspect",
        version: 1,
        purpose: "One managed habitat service: its owner, its systemd unit, the cached health from the last probe, and optionally one operation by id or by key.",
        effect: Effect::Read,
        owner: Owner::Service,
        scope: Scope::V42,
        readback_action: None,
        precondition: PreconditionRule::None,
        request_fields: &["service_id", "operation"],
        result_fields: &[
            "service_id",
            "owner_id",
            "unit_id",
            "cached_health",
            "operation",
        ],
    },
    Action {
        id: "service.probe",
        version: 1,
        purpose: "Run one bounded probe against a managed service (a busctl get-property read over S-4, output bounded over S-11), validate it into an Observation, and commit it to service_facts as an operation.",
        effect: Effect::BoundedProbe,
        owner: Owner::Service,
        scope: Scope::V42,
        readback_action: Some("service.inspect"),
        precondition: PreconditionRule::None,
        request_fields: &[
            "service_id",
            "probe_id",
            "probe_version",
            "max_cost_microunits",
            "network_scope",
        ],
        result_fields: &[
            "operation_id",
            "service_id",
            "observation",
            "cost_microunits",
            "external_effect",
        ],
    },
    Action {
        id: "service.action",
        version: 1,
        purpose: "A managed lifecycle act on a service's unit (start, stop, ...) over S-4 D-Bus through K0h cgroup-io, guarded by the expected owner digest and a precondition, with the observed state and useful health read back.",
        effect: Effect::ManagedLifecycle,
        owner: Owner::Service,
        scope: Scope::V42,
        readback_action: Some("service.inspect"),
        precondition: PreconditionRule::Required("service"),
        request_fields: &["service_id", "unit_id", "action", "expected_owner_sha256"],
        result_fields: &[
            "operation_id",
            "service_id",
            "owner_job_id",
            "observed_state",
            "useful_health",
        ],
    },
    Action {
        id: "thread.get",
        version: 1,
        purpose: "One cohort thread: its task, brief revision, state, obligations, children and artifacts, read against an expected brief revision.",
        effect: Effect::Read,
        owner: Owner::Cohort,
        scope: Scope::V42,
        readback_action: None,
        precondition: PreconditionRule::None,
        request_fields: &["thread_id", "expected_brief_revision"],
        result_fields: &[
            "thread_id",
            "task_id",
            "brief_revision",
            "state",
            "obligations",
            "children",
            "artifacts",
        ],
    },
    Action {
        id: "thread.list",
        version: 1,
        purpose: "Paged thread heads under a task, filtered by state.",
        effect: Effect::Read,
        owner: Owner::Cohort,
        scope: Scope::V42,
        readback_action: None,
        precondition: PreconditionRule::None,
        request_fields: &["task_id", "states", "page"],
        result_fields: &["page"],
    },
    Action {
        id: "analysis.request",
        version: 1,
        purpose: "Request a bounded numerical analysis of a dataset by a pinned recipe and runtime: a julia child (S-7) under the K0h spawn door with wall, memory and output limits, recorded as an operation and an analysis row with its own state.",
        effect: Effect::BoundedAnalysis,
        owner: Owner::Numerical,
        scope: Scope::V42,
        readback_action: Some("analysis.get"),
        precondition: PreconditionRule::None,
        request_fields: &[
            "subject",
            "dataset",
            "cutoff_unix_ms",
            "recipe_id",
            "recipe_version",
            "runtime_id",
            "limits",
        ],
        result_fields: &[
            "analysis_id",
            "task_id",
            "attempt_id",
            "generation",
            "dataset_sha256",
            "state",
        ],
    },
    Action {
        id: "analysis.get",
        version: 1,
        purpose: "Read one analysis: its state, dataset digest, decoded output (once validated) or error code, selected by id or by the request key.",
        effect: Effect::Read,
        owner: Owner::Numerical,
        scope: Scope::V42,
        readback_action: None,
        precondition: PreconditionRule::None,
        request_fields: &["selector"],
        result_fields: &[
            "analysis_id",
            "state",
            "dataset_sha256",
            "report",
            "error_code",
        ],
    },
    Action {
        id: "judge.inspect",
        version: 1,
        purpose: "Read the judge admission state: question sets, admission state and shadow agreement, from the append-only judgments table (digests, not state).",
        effect: Effect::Read,
        owner: Owner::Judge,
        scope: Scope::Held,
        readback_action: None,
        precondition: PreconditionRule::None,
        request_fields: &[],
        result_fields: &[],
    },
];

/// The entry for `id`, or `None` for an id the catalogue does not carry. The one id lookup:
/// dispatch, `tools.inspect` and the CLI all go through it.
#[must_use]
pub fn find(id: &str) -> Option<&'static Action> {
    CATALOGUE.iter().find(|a| a.id == id)
}

/// The catalogue as a JSON array of [`Action::to_json`] objects, in catalogue order.
#[must_use]
pub fn to_json() -> Value {
    Value::Array(CATALOGUE.iter().map(Action::to_json).collect())
}

/// The content digest of the catalogue: SHA-256 over the canonical JSON of [`to_json`]. A
/// planted edit to any entry changes it (catalogue-data card §9 #1).
#[must_use]
pub fn revision() -> Sha256Hex {
    Sha256Hex::digest(canonical_json(&to_json()).as_bytes())
}
