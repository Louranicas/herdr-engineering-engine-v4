//! The roster family (K6, v4.1): `roster.list`, `roster.inspect`, `roster.update` and
//! `roster.disable` over K1's `store/roster.rs` verbs, plus the startup hook that composes the
//! native model's own record under `Owner::Deploy` (`Store::roster_compose_deploy`, action
//! `deploy.install`), never under the wire id `roster.update` (V4-62; the v3 AR SYS HIGH defect).
//! Parses the wire, calls K1, defines no contract type. Every operation key's action is the
//! request's own `action`, so no roster id is spelled into the ledger here either.
//!
//! Active attempts of a disabled record (INTERP until K1-attempts-ledger lands): the tasks in
//! `running`, `verifying` or `cancellation_requested` when the record is the model `route` would
//! select now. The `forbidden` (operator capability) path has no reachable site: no grants exist
//! (owner: the grants slice).

use hee4_contracts::bounds::MAX_VIEW_ITEMS;
use hee4_contracts::catalogue::Owner;
use hee4_contracts::{Event, Phase, TaskId};
use hee4_core::roster::{
    DisablePolicy, Locality, RosterCaps, RosterDefinition, RosterError, RosterFilter, RosterId,
    RosterKind,
};
use hee4_core::{OperationKey, StoreError};
use serde_json::{Value, json};

use super::page::{Cursor, PageIn, PageOut};
use super::registry::{Family, StartFault};
use super::{Answer, Engine, Reply, internal};
use crate::dispatcher;
use crate::wire::{Code, Fault, Precondition, Request};

/// The four `roster.*` handlers and the deploy-record hook: owner `Roster`.
pub const FAMILY: Family = Family {
    owner: Owner::Roster,
    handlers: &[
        ("roster.list", |engine, req| frame(list(engine, &req.body))),
        ("roster.inspect", |engine, req| {
            frame(inspect(engine, &req.body))
        }),
        ("roster.update", |engine, req| frame(update(engine, req))),
        ("roster.disable", |engine, req| frame(disable(engine, req))),
    ],
    on_serve_start: Some(on_serve_start),
};

/// A one-frame handler's reply as dispatch's `Answer`.
fn frame(reply: Reply) -> Result<Answer, Fault> {
    reply.map(|(replayed, body)| Answer::Frame(replayed, body))
}

/// The native model's declared row: the one literal, shared with `dispatcher::roster_from_store`'s
/// empty-table fallback (the figures that were the literal in `route`).
pub(crate) fn default_definition() -> RosterDefinition {
    RosterDefinition {
        kind: RosterKind::Model,
        caps: RosterCaps {
            ctx_tokens: 32_768,
            json_mode: true,
            tool_use: false,
            local: true,
        },
        cost_milli: 0,
        latency_ms: 0,
        quality: 0,
        capability: None,
        locality: Locality::Local,
    }
}

/// Compose `model:<cfg.model>` under `deploy.install`; replayed on every later start with the
/// same definition. Prints `roster deploy record=<id> replayed=<bool> operation=<id>`.
fn on_serve_start(engine: &Engine) -> Result<(), StartFault> {
    let op = engine
        .store()
        .roster_compose_deploy(&engine.cfg.model, &default_definition())
        .map_err(|e| StartFault::Io {
            owner: Owner::Roster,
            source: std::io::Error::other(e),
        })?;
    eprintln!(
        "roster deploy record={} replayed={} operation={}",
        op.subject.as_deref().unwrap_or_default(),
        op.replayed,
        op.operation_id
    );
    Ok(())
}

fn invalid(field: &'static str, message: impl Into<String>) -> Fault {
    Fault::new(Code::InvalidArgument, field, message)
}

/// `body.kinds`: an array of at most the distinct kinds, each `agent|model|runtime`.
fn kinds_of(v: Option<&Value>) -> Result<Vec<RosterKind>, Fault> {
    let Some(Value::Array(items)) = v else {
        return Err(invalid(
            "/body/kinds",
            "array of agent|model|runtime required (empty admits every kind)",
        ));
    };
    let mut kinds = Vec::with_capacity(items.len());
    for item in items {
        let kind = item
            .as_str()
            .and_then(RosterKind::parse)
            .ok_or_else(|| invalid("/body/kinds", "each kind is agent, model or runtime"))?;
        if kinds.contains(&kind) {
            return Err(invalid("/body/kinds", "a kind named twice"));
        }
        kinds.push(kind);
    }
    Ok(kinds)
}

/// `body.<key>`: `null` or a string, required.
fn optional_string(body: &Value, key: &str, field: &'static str) -> Result<Option<String>, Fault> {
    match body.get(key) {
        Some(Value::Null) => Ok(None),
        Some(Value::String(s)) => Ok(Some(s.clone())),
        _ => Err(invalid(field, "string or null required")),
    }
}

fn record_id_of(body: &Value) -> Result<RosterId, Fault> {
    let text = body
        .get("record_id")
        .and_then(Value::as_str)
        .ok_or_else(|| invalid("/body/record_id", "string required"))?;
    RosterId::parse(text).map_err(|e| invalid("/body/record_id", e.to_string()))
}

fn audit_reason_of(body: &Value) -> Result<&str, Fault> {
    body.get("audit_reason")
        .and_then(Value::as_str)
        .ok_or_else(|| invalid("/body/audit_reason", "string required"))
}

/// The caller's precondition against `id`: resource `roster`, the same id; its generation.
fn expected_generation(p: &Precondition, id: &RosterId) -> Result<u64, Fault> {
    if p.resource != "roster" {
        return Err(invalid("/precondition/resource", "roster"));
    }
    if p.id != id.as_str() {
        return Err(invalid(
            "/precondition/id",
            "the precondition's id is not the body's record_id",
        ));
    }
    Ok(p.generation)
}

fn operation_key(engine: &Engine, req: &Request) -> OperationKey {
    OperationKey {
        principal: engine.principal.clone(),
        action: req.action.clone(),
        version: req.action_version,
        idem_key: req.idempotency_key.clone().unwrap_or_default(),
    }
}

/// K1's refusal as the wire's: stale generation carries `current_generation`.
fn roster_fault(e: RosterError) -> Fault {
    match e {
        RosterError::Store(StoreError::Conflict(_)) => Fault::new(
            Code::Conflict,
            "/idempotency_key",
            "key recorded with other bytes",
        ),
        RosterError::Store(other) => internal(&other),
        RosterError::StaleGeneration { current, .. } => Fault::new(
            Code::StaleGeneration,
            "/precondition/generation",
            "the record's generation moved; read it again",
        )
        .with_generation(current),
        RosterError::NotFound(_) => Fault::new(Code::NotFound, "/body/record_id", "no such record"),
        RosterError::Id(e) => invalid("/body/record_id", e.to_string()),
        e @ (RosterError::KindImmutable { .. } | RosterError::KindPrefixMismatch { .. }) => {
            invalid("/body/definition/kind", e.to_string())
        }
    }
}

/// `{kinds, capability, locality, include_disabled, page}` (all five required) ->
/// `{page: {items: [RosterHeadV1], cursor}}`, keyset by id, cursor pinned to the boot and the
/// filter digest.
fn list(engine: &Engine, body: &Value) -> Reply {
    let kinds = kinds_of(body.get("kinds"))?;
    let capability = optional_string(body, "capability", "/body/capability")?;
    let locality = match optional_string(body, "locality", "/body/locality")? {
        None => None,
        Some(s) => Some(
            Locality::parse(&s)
                .ok_or_else(|| invalid("/body/locality", "local, remote or null"))?,
        ),
    };
    let include_disabled = body
        .get("include_disabled")
        .and_then(Value::as_bool)
        .ok_or_else(|| invalid("/body/include_disabled", "boolean required"))?;
    if !matches!(body.get("page"), Some(Value::Object(_))) {
        return Err(invalid("/body/page/limit", "page {limit, cursor} required"));
    }
    let page_in = PageIn::parse(body.get("page"))?;
    let filter = RosterFilter {
        kinds,
        capability,
        locality,
        include_disabled,
    };
    let digest = filter.digest();
    let boot = engine.boot();
    let after = match &page_in.cursor {
        Some(cursor) => {
            cursor.resumes(boot, &digest)?;
            Some(cursor.after_key.as_str())
        }
        None => None,
    };
    let mut heads = engine
        .store()
        .roster_list(&filter, after, page_in.limit.saturating_add(1))
        .map_err(|e| internal(&e))?;
    let more = heads.len() > page_in.limit;
    heads.truncate(page_in.limit);
    let cursor = more.then(|| heads.last()).flatten().map(|last| Cursor {
        after_key: last.id.as_str().to_owned(),
        boot,
        filter_sha256: digest,
    });
    let out = PageOut {
        items: heads
            .iter()
            .map(hee4_core::roster::RosterHead::to_json)
            .collect(),
        cursor,
    };
    Ok((false, json!({ "page": out.to_json() })))
}

/// `{selector: {record_id} | {source_action, idempotency_key}}` -> `{record, last_operation}`.
/// By key: the caller's own operation row (`Store::operation_by_key` under this principal), then
/// its subject; by id: the record. Either way `last_operation` is the newest row about it.
fn inspect(engine: &Engine, body: &Value) -> Reply {
    let shape = || {
        invalid(
            "/body/selector",
            "exactly {record_id} or {source_action, idempotency_key}",
        )
    };
    let Some(Value::Object(selector)) = body.get("selector") else {
        return Err(shape());
    };
    let not_found = || Fault::new(Code::NotFound, "/body/selector", "no such record");
    let store = engine.store();
    let id = match (
        selector.get("record_id"),
        selector.get("source_action"),
        selector.get("idempotency_key"),
    ) {
        (Some(Value::String(id)), None, None) if selector.len() == 1 => {
            RosterId::parse(id).map_err(|e| invalid("/body/selector", e.to_string()))?
        }
        (None, Some(Value::String(action)), Some(Value::String(key))) if selector.len() == 2 => {
            let op = OperationKey {
                principal: engine.principal.clone(),
                action: action.clone(),
                version: 1,
                idem_key: key.clone(),
            };
            let row = store
                .operation_by_key(&op)
                .map_err(|e| internal(&e))?
                .ok_or_else(not_found)?;
            let subject = row.subject.ok_or_else(not_found)?;
            RosterId::parse(&subject).map_err(|_| not_found())?
        }
        _ => return Err(shape()),
    };
    let record = store
        .roster_get(&id)
        .map_err(|e| internal(&e))?
        .ok_or_else(not_found)?;
    let last = store
        .last_operation_for(id.as_str())
        .map_err(|e| internal(&e))?
        .map(|row| {
            json!({
                "operation_id": row.operation_id,
                "action": row.action,
                "idempotency_key": row.idem_key,
                "ts": row.ts,
            })
        });
    Ok((
        false,
        json!({ "record": record.to_json(), "last_operation": last }),
    ))
}

/// `{record_id, definition, audit_reason}` (+ optional `precondition {roster, id, generation}`)
/// -> `{record, operation_id, change}`. An empty or partial definition is a refusal at
/// `/body/definition`, never a no-op revision.
fn update(engine: &Engine, req: &Request) -> Reply {
    let body = &req.body;
    let id = record_id_of(body)?;
    let definition = match body.get("definition") {
        Some(v @ Value::Object(_)) => RosterDefinition::from_json(v)
            .map_err(|e| invalid("/body/definition", e.to_string()))?,
        _ => {
            return Err(invalid(
                "/body/definition",
                "object {kind, caps, cost_milli, latency_ms, quality, capability, locality} required",
            ));
        }
    };
    let audit_reason = audit_reason_of(body)?;
    let expected = match &req.precondition {
        None => None,
        Some(p) => Some(expected_generation(p, &id)?),
    };
    let op = operation_key(engine, req);
    let bytes =
        serde_json::to_vec(body).map_err(|e| Fault::new(Code::Internal, "/", e.to_string()))?;
    let operation = engine
        .store()
        .roster_update(&op, &bytes, &id, &definition, audit_reason, expected)
        .map_err(roster_fault)?;
    let mut reply = operation.result;
    reply["operation_id"] = json!(operation.operation_id);
    Ok((operation.replayed, reply))
}

/// The tasks mid-attempt on `id` now (module doc INTERP): empty unless `id` is the model the
/// dispatcher selects over the stored roster, evaluated as the dispatcher does when live
/// (`route_as_dispatched`: no upstream call from a roster path).
fn active_attempts(engine: &Engine, id: &RosterId) -> Result<Vec<TaskId>, Fault> {
    let (roster, _) =
        dispatcher::roster_from_store(engine, &engine.cfg).map_err(|e| internal(&e))?;
    let selected = dispatcher::route_as_dispatched(&engine.cfg, &roster)
        .ok()
        .map(|s| s.model);
    if selected.as_deref() != id.model_name() {
        return Ok(Vec::new());
    }
    let store = engine.store();
    let mut active = Vec::new();
    for task in store.task_ids().map_err(|e| internal(&e))? {
        let mid_attempt = matches!(
            store.phase(&task).map_err(|e| internal(&e))?,
            Some(Phase::Running | Phase::Verifying | Phase::CancellationRequested)
        );
        if mid_attempt {
            active.push(task);
        }
    }
    Ok(active)
}

/// `{record_id, active_attempt_policy, audit_reason}` with the required `precondition` ->
/// `{record, operation_id, active_attempts, cancellation_obligations}`. With `request_cancel`
/// each listed task gets `Store::apply(Cancel)` after the disable commits; a refused edge is the
/// obligation's `refusal` text. A replay re-cancels nothing and reports each task's phase now.
fn disable(engine: &Engine, req: &Request) -> Reply {
    let body = &req.body;
    let id = record_id_of(body)?;
    let policy = body
        .get("active_attempt_policy")
        .and_then(Value::as_str)
        .and_then(DisablePolicy::parse)
        .ok_or_else(|| {
            invalid(
                "/body/active_attempt_policy",
                "let_finish or request_cancel",
            )
        })?;
    let audit_reason = audit_reason_of(body)?;
    // Dispatch refused the absent member already (`PreconditionRule::Required`); a `None` here
    // is a compose defect, not a second check.
    let precondition = req.precondition.as_ref().ok_or_else(|| {
        Fault::new(
            Code::Internal,
            "/precondition",
            "dispatch admitted roster.disable without its precondition",
        )
    })?;
    let expected = expected_generation(precondition, &id)?;
    let active = active_attempts(engine, &id)?;
    if active.len() > MAX_VIEW_ITEMS {
        return Err(Fault::new(
            Code::ResourceExhausted,
            "/",
            format!(
                "{} active attempts, at most {MAX_VIEW_ITEMS} in one reply",
                active.len()
            ),
        ));
    }
    let op = operation_key(engine, req);
    let bytes =
        serde_json::to_vec(body).map_err(|e| Fault::new(Code::Internal, "/", e.to_string()))?;
    let operation = engine
        .store()
        .roster_disable(&op, &bytes, &id, expected, policy, audit_reason, &active)
        .map_err(roster_fault)?;
    let listed: Vec<TaskId> = operation.result["active_attempts"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .filter_map(|s| s.parse().ok())
        .collect();
    let obligations: Vec<Value> = match policy {
        DisablePolicy::LetFinish => Vec::new(),
        DisablePolicy::RequestCancel => listed
            .iter()
            .map(|task| {
                let store = engine.store();
                let outcome = if operation.replayed {
                    store.phase(task).map(|p| p.map(|p| p.as_str().to_owned()))
                } else {
                    store
                        .apply(task, Event::Cancel)
                        .map(|p| Some(p.as_str().to_owned()))
                };
                match outcome {
                    Ok(Some(phase)) => json!({"task_id": task.as_str(), "phase_after": phase}),
                    Ok(None) => json!({"task_id": task.as_str(), "refusal": "no such task"}),
                    Err(e) => json!({"task_id": task.as_str(), "refusal": e.to_string()}),
                }
            })
            .collect(),
    };
    Ok((
        operation.replayed,
        json!({
            "record": operation.result["record"],
            "operation_id": operation.operation_id,
            "active_attempts": listed.iter().map(TaskId::as_str).collect::<Vec<_>>(),
            "cancellation_obligations": obligations,
        }),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::handle;
    use crate::actions::testing::{BRIEF, engine};
    use crate::wire;
    use hee4_core::{Observations, reconcile};

    type R = Result<(), Box<dyn std::error::Error>>;

    fn call(e: &Engine, action: &str, key: Option<&str>, body: Value) -> Value {
        handle(e, &wire::request("r", action, key, body).to_string())
    }

    fn call_with(
        e: &Engine,
        action: &str,
        key: Option<&str>,
        body: Value,
        precondition: Value,
    ) -> Value {
        handle(
            e,
            &wire::request_with("r", action, key, body, Some(precondition)).to_string(),
        )
    }

    fn list_body() -> Value {
        json!({"kinds": [], "capability": null, "locality": null, "include_disabled": false,
               "page": {"limit": 10, "cursor": null}})
    }

    fn ids_listed(reply: &Value) -> Vec<String> {
        reply["body"]["page"]["items"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|i| i["id"].as_str().map(str::to_owned))
            .collect()
    }

    fn ready(name: &str) -> Result<Engine, Box<dyn std::error::Error>> {
        let e = engine(name)?;
        reconcile(&e.store(), &Observations::worker_absent())?;
        Ok(e)
    }

    fn precondition(id: &str, generation: u64) -> Value {
        json!({"resource": "roster", "id": id, "generation": generation})
    }

    #[test]
    fn list_requires_all_five_members() -> R {
        let e = ready("roster-list")?;
        let ok = call(&e, "roster.list", None, list_body());
        assert_eq!(ok["kind"], "result", "{ok}");
        assert_eq!(ok["body"]["page"]["items"], json!([]));
        assert_eq!(ok["body"]["page"]["cursor"], Value::Null);
        for (member, field) in [
            ("kinds", "/body/kinds"),
            ("capability", "/body/capability"),
            ("locality", "/body/locality"),
            ("include_disabled", "/body/include_disabled"),
            ("page", "/body/page/limit"),
        ] {
            let mut body = list_body();
            body.as_object_mut().map(|o| o.remove(member));
            let reply = call(&e, "roster.list", None, body);
            assert_eq!(reply["code"], "invalid_argument", "{member}: {reply}");
            assert_eq!(reply["field"], field, "{member}: {reply}");
        }
        let mut dup = list_body();
        dup["kinds"] = json!(["model", "model"]);
        assert_eq!(call(&e, "roster.list", None, dup)["field"], "/body/kinds");
        let mut bad_locality = list_body();
        bad_locality["locality"] = json!("moon");
        assert_eq!(
            call(&e, "roster.list", None, bad_locality)["field"],
            "/body/locality"
        );
        let mut limit = list_body();
        limit["page"]["limit"] = json!(0);
        assert_eq!(
            call(&e, "roster.list", None, limit)["field"],
            "/body/page/limit"
        );
        let mut cursor = list_body();
        cursor["page"]["cursor"] = json!({"after_key": "a", "boot": 0,
            "filter_sha256": RosterFilter { kinds: vec![], capability: None, locality: None, include_disabled: false }.digest().to_string()});
        let resync = call(&e, "roster.list", None, cursor);
        assert_eq!(resync["code"], "resync_required");
        assert_eq!(resync["field"], "/body/page/cursor");
        Ok(())
    }

    #[test]
    fn inspect_unknown_is_not_found_at_selector() -> R {
        let e = ready("roster-inspect")?;
        let by_id = call(
            &e,
            "roster.inspect",
            None,
            json!({"selector": {"record_id": "model:none"}}),
        );
        assert_eq!(by_id["code"], "not_found", "{by_id}");
        assert_eq!(by_id["field"], "/body/selector");
        let by_key = call(
            &e,
            "roster.inspect",
            None,
            json!({"selector": {"source_action": "roster.update", "idempotency_key": "never"}}),
        );
        assert_eq!(by_key["code"], "not_found");
        for bad in [
            json!({}),
            json!({"selector": {}}),
            json!({"selector": {"record_id": "model:x", "source_action": "roster.update", "idempotency_key": "k"}}),
            json!({"selector": {"record_id": 5}}),
        ] {
            let reply = call(&e, "roster.inspect", None, bad);
            assert_eq!(reply["code"], "invalid_argument", "{reply}");
            assert_eq!(reply["field"], "/body/selector");
        }
        Ok(())
    }

    #[test]
    fn update_empty_definition_is_invalid_argument_at_body_definition() -> R {
        let e = ready("roster-update-empty")?;
        for definition in [json!({}), json!({"kind": "model"}), json!(null), json!("x")] {
            let reply = call(
                &e,
                "roster.update",
                Some("k"),
                json!({"record_id": "model:x", "definition": definition, "audit_reason": "r"}),
            );
            assert_eq!(reply["code"], "invalid_argument", "{reply}");
            assert_eq!(reply["field"], "/body/definition", "{reply}");
        }
        assert_eq!(
            e.store().roster_count()?,
            0,
            "a refused create writes nothing"
        );
        let bad_id = call(
            &e,
            "roster.update",
            Some("k"),
            json!({"record_id": "Model x", "definition": default_definition().to_json(), "audit_reason": "r"}),
        );
        assert_eq!(bad_id["field"], "/body/record_id");
        Ok(())
    }

    #[test]
    fn update_kind_change_and_id_kind_mismatch_are_refused_at_definition_kind() -> R {
        let e = ready("roster-update-kind")?;
        let model = default_definition().to_json();
        let created = call(
            &e,
            "roster.update",
            Some("k1"),
            json!({"record_id": "model:x", "definition": model, "audit_reason": "add"}),
        );
        assert_eq!(created["body"]["change"], "created", "{created}");
        let mut agent = model.clone();
        agent["kind"] = json!("agent");
        let moved = call(
            &e,
            "roster.update",
            Some("k2"),
            json!({"record_id": "model:x", "definition": agent, "audit_reason": "hide"}),
        );
        assert_eq!(moved["code"], "invalid_argument", "{moved}");
        assert_eq!(moved["field"], "/body/definition/kind", "{moved}");
        for (n, (record_id, definition)) in [("model:y", &agent), ("agent:z", &model)]
            .into_iter()
            .enumerate()
        {
            let odd = call(
                &e,
                "roster.update",
                Some(&format!("k3-{n}")),
                json!({"record_id": record_id, "definition": definition, "audit_reason": "odd"}),
            );
            assert_eq!(odd["code"], "invalid_argument", "{record_id}: {odd}");
            assert_eq!(odd["field"], "/body/definition/kind", "{record_id}: {odd}");
            let inspected = call(
                &e,
                "roster.inspect",
                None,
                json!({"selector": {"record_id": record_id}}),
            );
            assert_eq!(inspected["code"], "not_found", "{record_id}: {inspected}");
        }
        let (roster, exclusions) = dispatcher::roster_from_store(&e, &e.cfg)?;
        let routed: Vec<&str> = roster.models.iter().map(|m| m.name.as_str()).collect();
        assert_eq!(routed, vec!["x"]);
        assert_eq!(exclusions, Vec::<String>::new());
        Ok(())
    }

    #[test]
    fn update_stale_precondition_is_stale_generation_with_current_generation() -> R {
        let e = ready("roster-update-stale")?;
        let body = json!({"record_id": "model:x", "definition": default_definition().to_json(), "audit_reason": "add"});
        let created = call(&e, "roster.update", Some("k1"), body.clone());
        assert_eq!(created["kind"], "result", "{created}");
        assert_eq!(created["body"]["change"], "created");
        assert_eq!(created["body"]["record"]["generation"], 1);
        assert!(created["body"]["operation_id"].is_string());
        let replay = call(&e, "roster.update", Some("k1"), body.clone());
        assert_eq!(replay["replayed"], true);
        assert_eq!(replay["body"], created["body"]);
        let mut other = body.clone();
        other["audit_reason"] = json!("other");
        let conflict = call(&e, "roster.update", Some("k1"), other);
        assert_eq!(conflict["code"], "conflict");
        assert_eq!(conflict["field"], "/idempotency_key");
        let revised = call_with(
            &e,
            "roster.update",
            Some("k2"),
            body.clone(),
            precondition("model:x", 1),
        );
        assert_eq!(revised["body"]["change"], "revised", "{revised}");
        assert_eq!(revised["body"]["record"]["generation"], 2);
        let stale = call_with(
            &e,
            "roster.update",
            Some("k3"),
            body.clone(),
            precondition("model:x", 1),
        );
        assert_eq!(stale["code"], "stale_generation", "{stale}");
        assert_eq!(stale["retry"], "never");
        assert_eq!(stale["field"], "/precondition/generation");
        assert_eq!(stale["current_generation"], 2);
        let wrong_id = call_with(
            &e,
            "roster.update",
            Some("k4"),
            body,
            precondition("model:y", 2),
        );
        assert_eq!(wrong_id["field"], "/precondition/id");
        let by_key = call(
            &e,
            "roster.inspect",
            None,
            json!({"selector": {"source_action": "roster.update", "idempotency_key": "k2"}}),
        );
        assert_eq!(by_key["body"]["record"]["generation"], 2, "{by_key}");
        assert_eq!(by_key["body"]["last_operation"]["idempotency_key"], "k2");
        assert_eq!(
            by_key["body"]["last_operation"]["operation_id"],
            revised["body"]["operation_id"]
        );
        Ok(())
    }

    #[test]
    fn disable_without_precondition_is_refused_at_precondition() -> R {
        let e = ready("roster-disable-nopre")?;
        let reply = call(
            &e,
            "roster.disable",
            Some("k"),
            json!({"record_id": "model:x", "active_attempt_policy": "let_finish", "audit_reason": "r"}),
        );
        assert_eq!(reply["kind"], "error");
        assert_eq!(reply["code"], "invalid_argument", "{reply}");
        assert_eq!(reply["field"], "/precondition");
        Ok(())
    }

    #[test]
    fn disable_lets_list_hide_then_show_with_include_disabled() -> R {
        let e = ready("roster-disable-list")?;
        let create = json!({"record_id": "model:x", "definition": default_definition().to_json(), "audit_reason": "add"});
        assert_eq!(
            call(&e, "roster.update", Some("k1"), create)["kind"],
            "result"
        );
        assert_eq!(
            ids_listed(&call(&e, "roster.list", None, list_body())),
            ["model:x"]
        );
        let disable = json!({"record_id": "model:x", "active_attempt_policy": "let_finish", "audit_reason": "retire"});
        let bad_policy = call_with(
            &e,
            "roster.disable",
            Some("d0"),
            json!({"record_id": "model:x", "active_attempt_policy": "shrug", "audit_reason": "r"}),
            precondition("model:x", 1),
        );
        assert_eq!(bad_policy["field"], "/body/active_attempt_policy");
        let unknown = call_with(
            &e,
            "roster.disable",
            Some("d1"),
            json!({"record_id": "model:none", "active_attempt_policy": "let_finish", "audit_reason": "r"}),
            precondition("model:none", 1),
        );
        assert_eq!(unknown["code"], "not_found", "{unknown}");
        assert_eq!(unknown["field"], "/body/record_id");
        let done = call_with(
            &e,
            "roster.disable",
            Some("d2"),
            disable.clone(),
            precondition("model:x", 1),
        );
        assert_eq!(done["kind"], "result", "{done}");
        assert_eq!(done["body"]["record"]["disabled"], true);
        assert_eq!(done["body"]["record"]["generation"], 2);
        assert_eq!(done["body"]["active_attempts"], json!([]));
        assert_eq!(done["body"]["cancellation_obligations"], json!([]));
        let hidden = call(&e, "roster.list", None, list_body());
        assert!(ids_listed(&hidden).is_empty(), "{hidden}");
        let mut shown = list_body();
        shown["include_disabled"] = json!(true);
        let shown = call(&e, "roster.list", None, shown);
        assert_eq!(ids_listed(&shown), ["model:x"]);
        assert_eq!(shown["body"]["page"]["items"][0]["disabled"], true);
        let stale = call_with(
            &e,
            "roster.disable",
            Some("d3"),
            disable.clone(),
            precondition("model:x", 1),
        );
        assert_eq!(stale["code"], "stale_generation");
        assert_eq!(stale["current_generation"], 2);
        let replay = call_with(
            &e,
            "roster.disable",
            Some("d2"),
            disable,
            precondition("model:x", 1),
        );
        assert_eq!(replay["replayed"], true, "{replay}");
        assert_eq!(replay["body"]["record"], done["body"]["record"]);
        let preview = call(&e, "task.preview", None, json!({ "brief": BRIEF }));
        assert_eq!(
            preview["body"]["eligible"], false,
            "rows exist and none is eligible: no fallback to the declared model: {preview}"
        );
        assert_eq!(preview["body"]["exclusions"], json!(["model:x"]));
        Ok(())
    }

    #[test]
    fn disable_request_cancel_records_obligation_per_running_task() -> R {
        let e = ready("roster-disable-cancel")?;
        let hook = FAMILY.on_serve_start.ok_or("hook")?;
        hook(&e)?;
        let submit = call(&e, "task.submit", Some("s1"), json!({ "brief": BRIEF }));
        let task: TaskId = submit["body"]["task_id"]
            .as_str()
            .unwrap_or_default()
            .parse()?;
        e.store().apply(&task, Event::Dispatch)?;
        assert_eq!(e.store().phase(&task)?, Some(Phase::Running));
        let idle = call(
            &e,
            "task.submit",
            Some("s2"),
            json!({ "brief": BRIEF.replace("run true", "idle") }),
        );
        assert_eq!(idle["body"]["phase"], "admitted");
        let done = call_with(
            &e,
            "roster.disable",
            Some("d1"),
            json!({"record_id": "model:m:1", "active_attempt_policy": "request_cancel", "audit_reason": "retire"}),
            precondition("model:m:1", 1),
        );
        assert_eq!(done["kind"], "result", "{done}");
        assert_eq!(done["body"]["active_attempts"], json!([task.as_str()]));
        assert_eq!(
            done["body"]["cancellation_obligations"],
            json!([{"task_id": task.as_str(), "phase_after": "cancellation_requested"}])
        );
        assert_eq!(e.store().phase(&task)?, Some(Phase::CancellationRequested));
        let replay = call_with(
            &e,
            "roster.disable",
            Some("d1"),
            json!({"record_id": "model:m:1", "active_attempt_policy": "request_cancel", "audit_reason": "retire"}),
            precondition("model:m:1", 1),
        );
        assert_eq!(replay["replayed"], true, "{replay}");
        assert_eq!(
            replay["body"]["active_attempts"],
            done["body"]["active_attempts"]
        );
        let preview = call(&e, "task.preview", None, json!({ "brief": BRIEF }));
        assert_eq!(preview["body"]["eligible"], false, "{preview}");
        assert_eq!(preview["body"]["refusal"], "no_route");
        assert_eq!(preview["body"]["exclusions"], json!(["model:m:1"]));
        Ok(())
    }

    #[test]
    fn serve_start_hook_composes_deploy_record_under_deploy_install() -> R {
        let e = ready("roster-hook")?;
        let hook = FAMILY.on_serve_start.ok_or("hook")?;
        hook(&e)?;
        hook(&e)?;
        let listed = call(&e, "roster.list", None, list_body());
        assert_eq!(ids_listed(&listed), ["model:m:1"]);
        let inspected = call(
            &e,
            "roster.inspect",
            None,
            json!({"selector": {"record_id": "model:m:1"}}),
        );
        assert_eq!(
            inspected["body"]["last_operation"]["action"], "deploy.install",
            "{inspected}"
        );
        assert_eq!(
            inspected["body"]["record"]["generation"], 1,
            "a second start replays"
        );
        assert_eq!(
            inspected["body"]["record"]["definition"],
            default_definition().to_json()
        );
        let by_update_key = call(
            &e,
            "roster.inspect",
            None,
            json!({"selector": {"source_action": "roster.update", "idempotency_key": inspected["body"]["last_operation"]["idempotency_key"]}}),
        );
        assert_eq!(
            by_update_key["code"], "not_found",
            "no roster.update row exists"
        );
        let preview = call(&e, "task.preview", None, json!({ "brief": BRIEF }));
        assert_eq!(preview["body"], json!({"eligible": true, "model": "m:1"}));
        Ok(())
    }
}
