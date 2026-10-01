#!/usr/bin/env python3
"""Author the RC03 native control schema; this is not an engine validator.

Source: docs/contract-decisions.md, RC03 sections 2, 4 and 6. Run without
arguments to write the adjacent artifacts, or --check to compare exact bytes.
Only Python's standard library is needed. No transport or effects are started.

Per-action schema digests (decision D-4, RC03 section 4 compatibility tuple).
Beside the bundle this writes one standalone schema per action and kind --
control-v1.request.<action>.schema.json and control-v1.result.<action>.schema.json,
each the bundle's Request_<id> or Result_<id> definition at the root plus the
closure of every $defs entry it references -- and one shared error schema,
control-v1.error.schema.json (ControlErrorV1 and its closure): the error
taxonomy is one ErrorCodeV1, not per action. Every file is serialized like the
bundle (ASCII-escaped UTF-8, indent 2, sorted keys, one terminal LF). A
tools.inspect digest is "sha256:" + SHA-256 of that exact published file:
request_schema_sha256 names the action's request file, result_schema_sha256 its
result file, and error_schema_sha256 the shared error file for every action.
"""

import argparse
import json
from pathlib import Path


ACTIONS = (
    "tools.list", "tools.inspect", "task.preview", "task.submit", "task.get",
    "task.list", "task.cancel", "task.resolve", "thread.get", "thread.list",
    "roster.list", "roster.inspect", "roster.update", "roster.disable",
    "service.inspect", "service.probe", "service.action", "analysis.request",
    "analysis.get", "events.subscribe", "health",
)
READ_ACTIONS = (
    "tools.list", "tools.inspect", "task.get", "task.list", "thread.get",
    "thread.list", "roster.list", "roster.inspect", "service.inspect",
    "analysis.get", "health",
)
EFFECTFUL = {
    "task.submit", "task.cancel", "task.resolve", "roster.update",
    "roster.disable", "service.probe", "service.action", "analysis.request",
}
TASK_STATES = (
    "admitted", "queued", "running", "verifying", "repair_pending",
    "cancellation_requested", "blocked", "accepted", "failed", "cancelled",
    "abandoned", "effect_unknown",
)
THREAD_STATES = ("planned", "assigned", "running", "joining", "blocked", "settled")
TOPICS = ("task", "thread", "roster", "service", "analysis", "delivery", "recovery")
ERROR_CODES = (
    "invalid_frame", "unsupported_protocol", "unsupported_version", "unknown_action",
    "unsupported_action_version", "unauthenticated", "forbidden", "invalid_argument",
    "not_found", "conflict", "stale_generation", "deadline_exceeded", "cancelled",
    "resource_exhausted", "unavailable", "resync_required", "effect_unknown", "internal",
)


def ref(name):
    return {"$ref": "#/$defs/" + name}


def enum(*values):
    return {"type": "string", "enum": list(values)}


def const(value):
    # The explicit type also rejects Python/validator Boolean-number confusion.
    return {"type": "integer" if isinstance(value, int) else "string", "const": value}


def either(*schemas):
    return {"oneOf": list(schemas)}


def nullable(schema):
    return either({"type": "null"}, schema)


def record(properties, optional=(), comment=None):
    result = {
        "type": "object", "properties": properties,
        "required": [key for key in properties if key not in optional],
        "additionalProperties": False,
    }
    if comment:
        result["$comment"] = comment
    return result


def array(items, maximum, minimum=0, unique=False):
    result = {"type": "array", "items": items, "minItems": minimum, "maxItems": maximum}
    if unique:
        result["uniqueItems"] = True
    return result


def integer(minimum, maximum):
    return {"type": "integer", "minimum": minimum, "maximum": maximum}


def ascii_text(minimum, maximum):
    return {
        "type": "string", "minLength": minimum, "maxLength": maximum,
        "not": {"pattern": r"[^\u0000-\u007f]"},
        "$comment": "ASCII includes controls; RC03 does not select a printable-only subset.",
    }


def utf8_text(minimum, maximum):
    return {
        "type": "string", "minLength": minimum, "maxLength": maximum,
        "$comment": (
            f"Runtime must validate UTF-8 and {minimum}..{maximum} encoded bytes, "
            "reject unpaired surrogate code units, and avoid normalization. "
            "JSON Schema length counts Unicode code points, "
            "not bytes; maxLength is only a necessary upper bound."
        ),
    }


def bounded_decimal(maximum):
    """A canonical ASCII decimal regex with an exact inclusive numeric ceiling."""
    ceiling = str(maximum)
    alternatives = ["0", f"[1-9][0-9]{{0,{len(ceiling) - 2}}}"]
    for index, character in enumerate(ceiling):
        low = 1 if index == 0 else 0
        high = int(character) - 1
        if high >= low:
            digit = str(low) if low == high else f"[{low}-{high}]"
            remaining = len(ceiling) - index - 1
            tail = f"[0-9]{{{remaining}}}" if remaining else ""
            alternatives.append(ceiling[:index] + digit + tail)
    alternatives.append(ceiling)
    return {
        "type": "string", "minLength": 1, "maxLength": len(ceiling),
        "pattern": "^(?:" + "|".join(alternatives) + ")$",
        # Some regex engines let $ match before a final newline.
        "not": {"pattern": "[^0-9]"},
        "$comment": f"Canonical decimal string, numeric range 0..{maximum} inclusive.",
    }


def action_name(prefix, action):
    return prefix + "_" + action.replace(".", "_")


def make_schema():
    definitions = {}

    def define(name, schema):
        if name in definitions:
            raise ValueError("Duplicate definition: " + name)
        definitions[name] = schema
        return ref(name)

    uuid = define("UuidV4", {
        "type": "string", "minLength": 36, "maxLength": 36,
        "pattern": r"^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$",
    })
    digest = define("Sha256", {
        "type": "string", "minLength": 71, "maxLength": 71,
        "pattern": r"^sha256:[0-9a-f]{64}$",
    })
    u64 = define("U64Decimal", bounded_decimal(2**64 - 1))
    define("U32Decimal", bounded_decimal(2**32 - 1))
    generation = define("Generation", {"allOf": [u64, {"not": {"const": "0"}}]})
    action_id = define("ActionId", enum(*ACTIONS))
    resource_kind = define("ResourceKind", enum(*TOPICS[:5]))
    error_code = define("ErrorCodeV1", enum(*ERROR_CODES))
    task_state = define("TaskStateV1", enum(*TASK_STATES))
    thread_state = define("ThreadStateV1", enum(*THREAD_STATES))
    event_topic = define("EventTopicV1", enum(*TOPICS))
    effect_class = define("ActionEffectV1", enum(
        "read", "planning", "durable", "cancel", "configuration", "probe",
        "lifecycle", "analysis", "stream",
    ))
    analysis_state = define("AnalysisStateV1", enum(
        "queued", "running", "validated", "failed", "cancelled", "unknown",
    ))
    boolean = {"type": "boolean"}
    null = {"type": "null"}
    version = const(1)
    page_cursor = define("PageCursorV1", record({
        "snapshot_revision": u64, "after_key": ascii_text(1, 256),
        "filter_sha256": digest, "expires_unix_ms": u64,
    }))
    page_in = define("PageInV1", record({
        "limit": integer(1, 100), "cursor": nullable(page_cursor),
    }))
    evidence = define("EvidenceRefV1", record({
        "artifact_id": uuid, "sha256": digest, "byte_length": integer(0, 2**32 - 1),
        "media_type": ascii_text(1, 128), "schema_id": ascii_text(1, 128),
    }))
    budget = define("BudgetV1", record({
        "mode": enum("hard", "conservative"), "wall_ms": u64,
        "tokens": u64, "currency_microunits": u64,
    }))
    spec = define("TaskSpecV1", record({
        "task_class": ascii_text(1, 64), "intent": utf8_text(1, 8192),
        "criteria": array(utf8_text(1, 1024), 64, 1),
        "privacy": enum("local_only", "remote_allowed"), "workspace_id": uuid,
        "budget": budget, "parent": nullable(record({
            "task_id": uuid, "allocation_id": uuid, "brief_revision": u64,
        })),
    }, comment="Runtime admits task_class, workspace, parent allocation and budget. RC01 overlay requires local_only privacy and zero currency; this vocabulary alone grants no remote capability."))
    task_head = define("TaskHeadV1", record({
        "task_id": uuid, "generation": generation, "state": task_state,
        "current_attempt_id": nullable(uuid), "unresolved_obligations": integer(0, 2**32 - 1),
    }))
    recipe = define("RecipeRefV1", record({
        "recipe_id": ascii_text(1, 128), "recipe_version": integer(1, 65535),
        "adapter_id": uuid, "actual_model_required": boolean,
    }))
    roster_definition = define("RosterDefinitionV1", record({
        "kind": enum("agent", "model", "service"), "display_name": utf8_text(1, 256),
        "owner_id": ascii_text(1, 128), "version": ascii_text(1, 128),
        "capabilities": array(ascii_text(1, 128), 128),
        "locality": enum("local", "remote", "hybrid"),
        "endpoint_ref": nullable(uuid), "limitations": utf8_text(0, 2048),
    }))
    roster_head = define("RosterHeadV1", record({
        "record_id": uuid, "record_version": u64, "definition": roster_definition,
        "disabled": boolean, "observation_cutoff_unix_ms": nullable(u64),
    }))
    health_observation = define("HealthObservationV1", record({
        "observation_id": uuid, "observed_unix_ms": u64,
        "state": enum("useful", "degraded", "unavailable", "unknown"),
        "owner_identity": ascii_text(1, 256), "latency_ms": nullable(u64),
        "evidence": array(evidence, 64),
    }))
    event_cursor = define("EventCursorV1", record({
        "epoch": uuid, "sequence": u64, "filter_sha256": digest,
        "visibility_revision": u64, "issued_unix_ms": u64, "expires_unix_ms": u64,
    }, comment="Runtime checks epoch, retention, expiry, principal visibility and the server-computed sorted filter digest; this selector supplies no authority."))

    def page_out(name, item):
        return define(name, record({
            "items": array(item, 100), "next_cursor": nullable(page_cursor),
            "snapshot_revision": u64,
        }, comment="Runtime also requires items.length <= the correlated request page.limit and complete snapshot/cursor/filter consistency; no silent truncation."))

    def key_selector(*actions):
        return record({"source_action": enum(*actions), "idempotency_key": uuid})

    request_bodies = {}
    result_bodies = {}

    def action(name, request, result):
        request_bodies[name] = define(action_name("BodyRequest", name), request)
        result_bodies[name] = define(action_name("BodyResult", name), result)

    tool_head = define("ToolHeadV1", record({
        "id": action_id, "version": version, "purpose": utf8_text(1, 256),
        "effect": effect_class,
    }))
    action("tools.list", record({"query": nullable(utf8_text(0, 256)), "page": page_in}), record({
        "catalogue_revision": u64, "page": page_out("PageOutToolsV1", tool_head),
    }))
    action("tools.inspect", record({"action": action_id, "version": version}), record({
        "action": action_id, "version": version, "purpose": utf8_text(1, 256),
        "effect": effect_class, "request_schema_sha256": digest,
        "result_schema_sha256": digest, "error_schema_sha256": digest,
        "max_request_bytes": const(1048576), "max_deadline_ms": const(60000),
        "readback_action": nullable(action_id),
    }))
    action("task.preview", record({
        "spec": spec, "brief_revision": u64, "catalogue_revision": u64,
    }), record({
        "eligible": array(recipe, 128), "exclusions": array(record({
            "recipe_id": ascii_text(1, 128),
            "code": enum("capability", "privacy", "budget", "stale", "unavailable"),
        }), 128), "cost_mode": enum("known", "bounded", "unknown"),
        "observations_cutoff_unix_ms": u64,
    }))
    action("task.submit", record({"spec": spec}), record({
        "task": {"allOf": [task_head, {"properties": {"generation": const("1")}}]},
        "engine_cursor": event_cursor,
    }))
    task_selector = define("TaskSelectorV1", either(
        record({"task_id": uuid}), key_selector("task.submit"),
    ))
    action("task.get", record({
        "selector": task_selector, "evidence": enum("none", "summary", "refs"),
    }), record({
        "task": task_head, "criteria_sha256": digest, "attempts": array(record({
            "attempt_id": uuid, "generation": generation,
            "state": enum("queued", "running", "settled", "unknown"),
            "effect": enum("none", "committed", "pending", "unknown"),
        }), 100), "cleanup": enum("none", "pending", "settled", "unknown"),
        "delivery": enum("none", "pending", "delivered", "unknown"),
        "evidence": array(evidence, 64), "cursor": event_cursor,
    }))
    action("task.list", record({
        "states": array(task_state, 12, unique=True), "task_class": nullable(ascii_text(1, 64)),
        "parent_task_id": nullable(uuid), "page": page_in,
    }), record({"page": page_out("PageOutTasksV1", task_head)}))
    action("task.cancel", record({
        "reason": enum("operator_request", "superseded", "budget", "deadline", "safety"),
        "note": nullable(utf8_text(0, 1024)),
    }), record({
        "task": task_head, "cancellation_obligation_id": uuid,
        "worker_settlement": enum("not_started", "pending", "settled", "unknown"),
    }))
    action("task.resolve", record({
        "obligation_id": uuid,
        "disposition": enum("retry", "abandon", "acknowledge_external_effect", "quarantine"),
        "reason": utf8_text(1, 2048), "evidence": array(evidence, 64),
    }), record({"task": task_head, "disposition_id": uuid, "obligation_state": enum("resolved", "pending")}))
    action("thread.get", record({"thread_id": uuid, "expected_brief_revision": nullable(u64)}), record({
        "thread_id": uuid, "task_id": uuid, "brief_revision": u64, "state": thread_state,
        "obligations": array(utf8_text(1, 512), 128), "children": array(task_head, 100),
        "artifacts": array(evidence, 64),
    }))
    thread_head = define("ThreadHeadV1", record({
        "thread_id": uuid, "task_id": uuid, "brief_revision": u64,
        "state": thread_state, "unresolved_obligations": integer(0, 2**32 - 1),
    }))
    action("thread.list", record({
        "task_id": nullable(uuid), "states": array(thread_state, 6, unique=True), "page": page_in,
    }), record({"page": page_out("PageOutThreadsV1", thread_head)}))
    action("roster.list", record({
        "kinds": array(enum("agent", "model", "service"), 3, 1, True),
        "capability": nullable(ascii_text(1, 128)),
        "locality": nullable(enum("local", "remote", "hybrid")),
        "include_disabled": boolean, "page": page_in,
    }), record({"page": page_out("PageOutRosterV1", roster_head)}))
    roster_selector = define("RosterSelectorV1", either(
        record({"record_id": uuid}), key_selector("roster.update", "roster.disable"),
    ))
    action("roster.inspect", record({"selector": roster_selector}), record({
        "record": roster_head, "last_operation": nullable(record({
            "operation_id": uuid, "source_action": enum("roster.update", "roster.disable"),
            "effect": enum("committed", "pending", "unknown"),
        })),
    }))
    action("roster.update", record({
        "record_id": nullable(uuid), "definition": roster_definition, "audit_reason": utf8_text(1, 1024),
    }), record({"record": roster_head, "operation_id": uuid, "change": enum("created", "updated")}))
    action("roster.disable", record({
        "record_id": uuid, "active_attempt_policy": enum("leave_running", "request_cancel"),
        "audit_reason": utf8_text(1, 1024),
    }), record({
        "record": roster_head, "operation_id": uuid, "active_attempts": array(uuid, 100),
        "cancellation_obligations": array(uuid, 100),
    }))
    service_operation_selector = define("ServiceOperationSelectorV1", either(
        record({"operation_id": uuid}), key_selector("service.probe", "service.action"),
    ))
    action("service.inspect", record({
        "service_id": uuid, "operation": nullable(service_operation_selector),
    }), record({
        "service_id": uuid, "owner_id": ascii_text(1, 128), "unit_id": nullable(ascii_text(1, 256)),
        "cached_health": nullable(health_observation), "operation": nullable(record({
            "operation_id": uuid, "source_action": enum("service.probe", "service.action"),
            "effect": enum("none", "committed", "pending", "unknown"),
            "owner_job_id": nullable(ascii_text(1, 256)),
        })),
    }))
    action("service.probe", record({
        "service_id": uuid, "probe_id": ascii_text(1, 128), "probe_version": version,
        "max_cost_microunits": u64, "network_scope": enum("none", "configured_allowlist"),
    }, comment="RC01 profile requires network_scope none and zero external spend; registered probe/owner/grant/cost bounds need runtime admission."), record({
        "operation_id": uuid, "service_id": uuid, "observation": health_observation,
        "cost_microunits": u64, "external_effect": enum("none", "bounded_probe"),
    }))
    action("service.action", record({
        "service_id": uuid, "unit_id": ascii_text(1, 256),
        "action": enum("start", "stop", "restart", "reload"), "expected_owner_sha256": digest,
    }), record({
        "operation_id": uuid, "service_id": uuid, "owner_job_id": nullable(ascii_text(1, 256)),
        "observed_state": enum("active", "inactive", "failed", "pending", "unknown"),
        "useful_health": nullable(health_observation),
    }))
    analysis_subject = define("AnalysisSubjectV1", record({
        "task_id": uuid, "attempt_id": uuid, "generation": generation,
    }))
    action("analysis.request", record({
        "subject": analysis_subject, "dataset": evidence, "cutoff_unix_ms": u64,
        "recipe_id": const("descriptive"), "recipe_version": version, "runtime_id": uuid,
        "limits": record({"wall_ms": u64, "memory_bytes": u64, "output_bytes": integer(1, 65536)}),
    }), record({
        "analysis_id": uuid, "task_id": uuid, "attempt_id": uuid, "generation": generation,
        "dataset_sha256": digest, "state": analysis_state,
    }))
    analysis_selector = define("AnalysisSelectorV1", either(
        record({"analysis_id": uuid}), key_selector("analysis.request"),
    ))
    action("analysis.get", record({"selector": analysis_selector}), record({
        "analysis_id": uuid, "task_id": uuid, "attempt_id": uuid, "generation": generation,
        "state": analysis_state, "dataset_sha256": digest, "report": nullable(evidence),
        "error_code": nullable(error_code),
    }))
    action("events.subscribe", record({
        "cursor": nullable(event_cursor), "topics": array(event_topic, 7, 1, True),
        "resource_ids": array(uuid, 100, unique=True), "bootstrap_limit": integer(1, 256),
    }), record({
        "subscription_id": uuid, "snapshot": array(ref("SnapshotHeadV1"), 256),
        "high_water_cursor": event_cursor,
    }, optional=("snapshot",), comment="Correlated request cursor null requires a complete visible snapshot with length <= bootstrap_limit. Snapshot is optional for resume; this structural schema cannot inspect the request."))
    action("health", record({}), record({
        "protocol_version": version, "engine_version": ascii_text(1, 128), "ready": boolean,
        "recovery": enum("complete", "pending", "blocked"),
        "database": enum("ready", "degraded", "unavailable"),
        "socket": enum("owned", "draining"), "checked_unix_ms": u64,
    }))

    readback = define("ReadbackSelectorV1", {"oneOf": [
        record({"action": const(name), "action_version": version, "body": request_bodies[name]})
        for name in READ_ACTIONS
    ]})
    define("SnapshotHeadV1", {"oneOf": [record({
        "resource": const(resource), "resource_id": uuid, "generation": generation,
        "readback": {"allOf": [readback, {"properties": {"action": const(action)}}]},
    }, comment="Runtime equates resource identity/generation and readback target. Delivery identifies its owning task; recovery identifies the engine instance and revision.") for resource, action in (
        ("task", "task.get"), ("delivery", "task.get"), ("thread", "thread.get"),
        ("roster", "roster.inspect"), ("service", "service.inspect"),
        ("analysis", "analysis.get"), ("recovery", "health"),
    )]})
    precondition = define("PreconditionV1", record({
        "resource": resource_kind, "id": uuid, "generation": generation,
    }))
    authority = define("AuthorityV1", record({"grant_id": uuid, "scope_sha256": digest}))

    def envelope(kind):
        return {"protocol": const("hee3.control"), "version": version, "kind": const(kind)}

    for name in ACTIONS:
        owner = name.split(".")[0]
        scoped_precondition = {"allOf": [precondition, {"properties": {"resource": const(owner)}}]}
        if owner not in TOPICS[:5]:
            expected_precondition = null
        else:
            expected_precondition = nullable(scoped_precondition)
        if name in {"task.cancel", "task.resolve", "roster.disable", "service.action"}:
            expected_precondition = scoped_precondition
        if name == "task.submit":
            expected_precondition = null
        request = record({
            **envelope("request"), "request_id": uuid, "action": const(name),
            "action_version": version, "idempotency_key": uuid if name in EFFECTFUL else nullable(uuid),
            "deadline_unix_ms": u64, "authority": authority,
            "precondition": expected_precondition, "body": request_bodies[name],
        }, comment="Runtime binds authenticated principal/grant, exact request bytes/digest, deadline <=60000ms ahead, visibility, idempotency custody and resource identity/generation equality before dispatch. UUIDs and schema validation grant no authority.")
        if name == "roster.update":
            request["allOf"] = [{
                "if": {"properties": {"body": {"properties": {"record_id": null}}}},
                "then": {"properties": {"precondition": null}},
                "else": {"properties": {"precondition": scoped_precondition}},
            }]
        define(action_name("Request", name), request)
        readback_action = {
            "task.submit": "task.get", "task.cancel": "task.get", "task.resolve": "task.get",
            "roster.update": "roster.inspect", "roster.disable": "roster.inspect",
            "service.probe": "service.inspect", "service.action": "service.inspect",
            "analysis.request": "analysis.get",
        }.get(name)
        result_readback = readback if readback_action is None else {
            "allOf": [readback, {"properties": {"action": const(readback_action)}}],
        }
        result = record({
            **envelope("result"), "request_id": uuid, "request_sha256": digest,
            "replayed": boolean, "effect": enum("none", "committed", "pending"),
            "operation_id": nullable(uuid), "observed_generation": nullable(generation),
            "readback": nullable(result_readback), "body": result_bodies[name],
        }, comment="Validate this per-action result with the retained request action, request_id and exact request_sha256. Runtime equates duplicated operation IDs/generations and validates readback correlation, effect truth and action-specific returned state. A typed result is not task/module acceptance.")
        result["allOf"] = [{
            "if": {"properties": {"effect": const("pending")}},
            "then": {"properties": {"operation_id": uuid, "readback": readback}},
        }]
        define(action_name("Result", name), result)
    define("ControlRequestV1", {"oneOf": [ref(action_name("Request", name)) for name in ACTIONS]})
    define("ControlResultV1", {
        "anyOf": [ref(action_name("Result", name)) for name in ACTIONS],
        "$comment": "STRUCTURAL UNION ONLY. RC03 result has no action discriminator. Empty list results overlap, so oneOf would reject valid replies. A retained request selects exactly one Result_<action> schema; generic anyOf cannot prove action compatibility.",
    })
    error = record({
        **envelope("error"), "request_id": uuid, "request_sha256": digest, "code": error_code,
        "effect": enum("none", "unknown"),
        "retry": enum("never", "same_exact_request", "after_readback", "after_condition"),
        "readback": nullable(readback), "message": utf8_text(0, 4096),
        "details": record({
            "field": nullable(ascii_text(1, 256)), "constraint": nullable(ascii_text(1, 256)),
            "current_generation": nullable(generation),
        }),
    }, comment="Runtime correlates request identity/digest and permitted visibility. Diagnostics are not control flow; no error implies rollback or settled effects.")
    error["allOf"] = [{
        "if": {"properties": {"code": const("effect_unknown")}},
        "then": {"properties": {"effect": const("unknown"), "retry": const("after_readback"), "readback": readback}},
    }, {
        "if": {"properties": {"retry": const("after_readback")}},
        "then": {"properties": {"readback": readback}},
    }]
    define("ControlErrorV1", error)
    event = record({
        **envelope("event"), "subscription_id": uuid, "event_id": uuid, "epoch": uuid,
        "sequence": u64, "previous_sequence": u64, "occurred_unix_ms": u64,
        "topic": event_topic, "subject": ref("SnapshotHeadV1"),
        "change": enum("created", "updated", "terminal", "deleted"), "cursor": event_cursor,
    }, comment="Runtime validates subscription/topic/subject identity, cursor epoch/sequence/filter/visibility, sequence > previous_sequence and previous_sequence == prior delivered cursor sequence; global gaps are legal. Requires retained-interval coverage; cannot prove continuity from one record.")
    event["allOf"] = [{
        "if": {"properties": {"topic": const(topic)}},
        "then": {"properties": {"subject": {"properties": {"resource": const(topic)}}}},
    } for topic in TOPICS]
    define("ControlEventV1", event)
    define("StreamStatusV1", record({
        **envelope("stream_status"), "subscription_id": uuid,
        "status": enum("resync_required", "queue_limit", "server_draining"),
        "last_delivered_cursor": nullable(event_cursor),
    }))
    if tuple(request_bodies) != ACTIONS or tuple(result_bodies) != ACTIONS:
        raise ValueError("Action catalogue coverage/order mismatch")
    return {
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "$id": "urn:hee3:control:1",
        "title": "HEE3-Control/1 structural schema candidate",
        "$comment": (
            "Source: docs/contract-decisions.md RC03 sections 2, 4 and 6. "
            "Candidate contract artifact, not implemented transport or module admission. "
            "Before schema validation, the typed receiver must enforce 1MiB frame payload, "
            "one compact UTF-8 object plus LF, no BOM/CRLF/outer whitespace, no duplicate "
            "keys/nonfinite numbers, nesting <=32 and bounded acquisition before allocation. "
            "JSON Schema cannot recover discarded duplicate keys or exact source bytes. "
            "Runtime validates UTF-8 byte lengths, numeric token representation, "
            "cross-field/cross-record equality, grant/custody, deadlines, idempotency, "
            "profile admission, evidence authenticity, queue limits and effect settlement. "
            "JSON Schema integer means numeric zero fractional part; exact token rejection "
            "of 1.0/1e0 where required belongs to typed parsing. Schema validity alone is "
            "not a compatible action tuple, verification result, accepted task or release. "
            "Per-action schema digests must bind immutable artifact bytes under an "
            "explicit publication convention; this bundle alone does not invent tuple digests."
        ),
        "oneOf": [ref(name) for name in (
            "ControlRequestV1", "ControlResultV1", "ControlErrorV1",
            "ControlEventV1", "StreamStatusV1",
        )],
        "$defs": definitions,
    }


def serialize(schema):
    return (json.dumps(schema, ensure_ascii=True, indent=2, sort_keys=True) + "\n").encode("utf-8")


def artifact_bytes():
    return serialize(make_schema())


REFERENCE = "#/$defs/"


def references(value):
    """Every $defs name a schema fragment references, in no particular order."""
    if isinstance(value, dict):
        for key, member in value.items():
            if key == "$ref" and isinstance(member, str) and member.startswith(REFERENCE):
                yield member[len(REFERENCE):]
            else:
                yield from references(member)
    elif isinstance(value, list):
        for member in value:
            yield from references(member)


def standalone(bundle, root, identity, title):
    """The definition `root` as a self-contained schema: it, plus every $defs entry it reaches."""
    definitions = bundle["$defs"]
    reached, pending = set(), [root]
    while pending:
        name = pending.pop()
        if name in reached:
            continue
        reached.add(name)
        pending.extend(references(definitions[name]))
    reached.discard(root)
    schema = dict(definitions[root])
    for key in ("$schema", "$id", "title", "$defs"):
        if key in schema:
            raise ValueError("Definition " + root + " already carries " + key)
    schema.update({
        "$schema": bundle["$schema"], "$id": identity, "title": title,
        "$defs": {name: definitions[name] for name in sorted(reached)},
    })
    return schema


def published():
    """Every artifact this generator owns, as {file name: exact bytes}."""
    bundle = make_schema()
    artifacts = {"control-v1.schema.json": serialize(bundle)}
    for action in ACTIONS:
        for kind, prefix in (("request", "Request"), ("result", "Result")):
            artifacts[f"control-v1.{kind}.{action}.schema.json"] = serialize(standalone(
                bundle, action_name(prefix, action), f"urn:hee3:control:1:{kind}:{action}",
                f"HEE3-Control/1 {action} v1 {kind} schema",
            ))
    artifacts["control-v1.error.schema.json"] = serialize(standalone(
        bundle, "ControlErrorV1", "urn:hee3:control:1:error", "HEE3-Control/1 error schema",
    ))
    return artifacts


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="compare adjacent artifacts without writing")
    arguments = parser.parse_args()
    directory = Path(__file__).parent
    artifacts = published()
    if arguments.check:
        differing = [name for name, generated in artifacts.items()
                     if not (directory / name).is_file() or (directory / name).read_bytes() != generated]
        if differing:
            parser.exit(1, "differs from its authoring generator: " + ", ".join(differing) + "\n")
        print(f"PASS deterministic control schema bytes files={len(artifacts)}")
    else:
        for name, generated in artifacts.items():
            (directory / name).write_bytes(generated)
        print(f"Wrote {len(artifacts)} files")


if __name__ == "__main__":
    main()
