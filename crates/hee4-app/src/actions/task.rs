//! The skeleton's handlers, moved verbatim from `actions.rs`: `health` (owner `App`), the six
//! `task.*` (owner `Task`) and `events.subscribe` (owner `Notify`), each registered through a
//! [`Family`] table that `composed()` names. Every state change goes through `Store::admit` or
//! `Store::apply`; the mutating gates (`idempotency_key`, `not_ready`) are dispatch's, from the
//! catalogue's `Effect::mutates()`.

use std::fs;
use std::io::Write as _;
use std::path::Path;

use hee4_contracts::catalogue::Owner;
use hee4_contracts::{
    AbandonReason, Brief, BriefField, Event, Phase, QuarantineReason, RecoveryRule, Resolution,
    Sha256Hex, TaskId,
};
use hee4_core::{CursorVerdict, OperationKey, Store, StoreError};
use hee4_host::model::OllamaClient;
use hee4_worker::native::StepKind;
use serde_json::{Value, json};

use super::registry::Family;
use super::{Answer, Engine, Reply, internal};
use crate::dispatcher;
use crate::wire::{Code, Fault, Request};

/// A one-frame handler's reply as dispatch's `Answer`.
fn frame(reply: Reply) -> Result<Answer, Fault> {
    reply.map(|(replayed, body)| Answer::Frame(replayed, body))
}

/// `health`: owner `App`.
pub const HEALTH: Family = Family {
    owner: Owner::App,
    handlers: &[("health", |engine, _| frame(health(engine)))],
    on_serve_start: None,
};

/// The six `task.*` handlers: owner `Task`.
pub const FAMILY: Family = Family {
    owner: Owner::Task,
    handlers: &[
        ("task.submit", |engine, req| frame(submit(engine, req))),
        ("task.get", |engine, req| frame(get(engine, &req.body))),
        ("task.list", |engine, _| frame(list(engine))),
        ("task.cancel", |engine, req| {
            frame(cancel(engine, &req.body))
        }),
        ("task.preview", |engine, req| {
            frame(preview(engine, &req.body))
        }),
        ("task.resolve", |engine, req| {
            frame(resolve(engine, &req.body))
        }),
    ],
    on_serve_start: None,
};

/// `events.subscribe`: owner `Notify`; its handler turns the connection into a stream.
pub const EVENTS: Family = Family {
    owner: Owner::Notify,
    handlers: &[("events.subscribe", |engine, req| {
        subscribe(engine, &req.body)
    })],
    on_serve_start: None,
};

/// `since_seq`: absent or null is 0; else an unsigned integer that fits the ledger's `seq`.
fn since_seq_of(body: &Value) -> Result<i64, Fault> {
    match body.get("since_seq") {
        None | Some(Value::Null) => Ok(0),
        Some(v) => v
            .as_u64()
            .and_then(|n| i64::try_from(n).ok())
            .ok_or_else(|| {
                Fault::new(
                    Code::InvalidArgument,
                    "/body/since_seq",
                    "unsigned integer or null",
                )
            }),
    }
}

/// `epoch`: absent or null is a legacy cursor (unchecked); a string is checked; anything else
/// is `invalid_argument`.
fn epoch_of(body: &Value) -> Result<Option<&str>, Fault> {
    match body.get("epoch") {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) => Ok(Some(s.as_str())),
        Some(_) => Err(Fault::new(
            Code::InvalidArgument,
            "/body/epoch",
            "string or null",
        )),
    }
}

/// `events.subscribe {since_seq, epoch}`: with an epoch, R13 runs through
/// `Store::cursor_check` before the ack and a stale cursor is `resync_required`, named by its
/// verdict (never a replay authorisation); without one the cursor is unchecked (legacy). The
/// ack carries the ledger epoch and the event high water so the next subscribe can be checked.
fn subscribe(engine: &Engine, body: &Value) -> Result<Answer, Fault> {
    let since_seq = since_seq_of(body)?;
    let epoch = epoch_of(body)?;
    let store = engine.store();
    let ledger_epoch = store.epoch().map_err(|e| internal(&e))?;
    let high_water = store.event_high_water().map_err(|e| internal(&e))?;
    if let Some(epoch) = epoch {
        let resync = |field: &'static str, because: &'static str, message: String| {
            Err(Fault::new(Code::ResyncRequired, field, message).with_because(because))
        };
        match store
            .cursor_check(epoch, since_seq.unsigned_abs())
            .map_err(|e| internal(&e))?
        {
            CursorVerdict::PriorEpochOfRestore => {
                return resync(
                    "/body/epoch",
                    "prior_epoch_of_restore",
                    format!("prior epoch of restore; the ledger epoch is {ledger_epoch}"),
                );
            }
            CursorVerdict::EpochChanged => {
                return resync(
                    "/body/epoch",
                    "epoch_changed",
                    format!("epoch changed; the ledger epoch is {ledger_epoch}"),
                );
            }
            CursorVerdict::FutureSequence => {
                return resync(
                    "/body/since_seq",
                    "future_sequence",
                    format!("future sequence; the event high water is {high_water}"),
                );
            }
            CursorVerdict::SnapshotOnly => {}
        }
    }
    let ack = json!({
        "since_seq": since_seq,
        "epoch": ledger_epoch,
        "high_water": high_water,
        "stream": "events",
    });
    Ok(Answer::Subscribe(ack, since_seq))
}

fn health(engine: &Engine) -> Reply {
    let store = engine.store();
    let complete = store.recovery_complete().map_err(|e| internal(&e))?;
    let schema_version = store.schema_version().map_err(|e| internal(&e))?;
    Ok((
        false,
        json!({
            "ok": complete,
            "head_sha": crate::HEAD,
            "recovery_complete": complete,
            "uptime_s": engine.started.elapsed().as_secs(),
            "schema_version": schema_version,
            "serve_cgroup": store.serve_cgroup(),
        }),
    ))
}

fn task_id_of(body: &Value) -> Result<TaskId, Fault> {
    body.get("task_id")
        .and_then(Value::as_str)
        .ok_or_else(|| Fault::new(Code::InvalidArgument, "/body/task_id", "string required"))?
        .parse()
        .map_err(|e: hee4_contracts::Refusal| {
            Fault::new(Code::InvalidArgument, "/body/task_id", e.to_string())
        })
}

fn submit(engine: &Engine, req: &Request) -> Reply {
    let text = req
        .body
        .get("brief")
        .and_then(Value::as_str)
        .ok_or_else(|| Fault::new(Code::InvalidArgument, "/body/brief", "string required"))?;
    let brief = Brief::parse(text)
        .map_err(|e| Fault::new(Code::InvalidArgument, "/body/brief", e.to_string()))?;
    brief
        .check_restatement()
        .map_err(|e| Fault::new(Code::InvalidArgument, "/body/brief", e.to_string()))?;
    // Admission is the one door for a vacuous VERIFY (V4-94): refused here, nothing is admitted
    // and no brief file is written; the dispatcher does not re-check.
    brief
        .check_verify()
        .map_err(|e| Fault::new(Code::InvalidArgument, "/body/brief", e.to_string()))?;
    let key = req.idempotency_key.clone().unwrap_or_default();
    let digest = Sha256Hex::digest(format!("{}\n{key}", engine.principal).as_bytes()).to_string();
    let task: TaskId = format!("t-{}", &digest[..24])
        .parse()
        .map_err(|e: hee4_contracts::Refusal| Fault::new(Code::Internal, "/", e.to_string()))?;
    let op = OperationKey {
        principal: engine.principal.clone(),
        action: req.action.clone(),
        version: req.action_version,
        idem_key: key,
    };
    let bytes = serde_json::to_vec(&req.body)
        .map_err(|e| Fault::new(Code::Internal, "/", e.to_string()))?;
    // The lock spans admit and the brief write, so the dispatcher never sees an admitted task
    // whose brief is not on disk yet.
    let store = engine.store();
    let admission = store
        .admit(
            &task,
            &op,
            &bytes,
            |t, p| json!({"task_id": t.as_str(), "phase": p.as_str()}),
        )
        .map_err(|e| match e {
            StoreError::Conflict(_) => Fault::new(
                Code::Conflict,
                "/idempotency_key",
                "key recorded with other bytes",
            ),
            other => internal(&other),
        })?;
    if !admission.replayed {
        // Crash between admit and this write: the dispatcher abandons the task, by name.
        write_brief(&engine.brief_path(&task), text)
            .map_err(|e| Fault::new(Code::Internal, "/", e.to_string()))?;
    }
    Ok((admission.replayed, admission.result))
}

fn write_brief(path: &Path, text: &str) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("tmp");
    let mut f = fs::File::create(&tmp)?;
    f.write_all(text.as_bytes())?;
    f.sync_all()?;
    fs::rename(&tmp, path)
}

fn phase_of(store: &Store, task: &TaskId) -> Result<Phase, Fault> {
    store
        .phase(task)
        .map_err(|e| internal(&e))?
        .ok_or_else(|| Fault::new(Code::NotFound, "/body/task_id", "no such task"))
}

fn get(engine: &Engine, body: &Value) -> Reply {
    let task = task_id_of(body)?;
    let store = engine.store();
    let phase = phase_of(&store, &task)?;
    let events = store.history(&task).map_err(|e| internal(&e))?.len();
    let head = store.chain_head(&task).map_err(|e| internal(&e))?;
    let last = (head != Sha256Hex::GENESIS).then(|| head.to_string());
    Ok((
        false,
        json!({"task_id": task.as_str(), "phase": phase.as_str(), "events": events, "last_receipt_hash": last}),
    ))
}

fn list(engine: &Engine) -> Reply {
    let store = engine.store();
    let mut tasks = Vec::new();
    for task in store.task_ids().map_err(|e| internal(&e))? {
        let phase = phase_of(&store, &task)?;
        tasks.push(json!({"task_id": task.as_str(), "phase": phase.as_str()}));
    }
    Ok((false, json!({ "tasks": tasks })))
}

fn cancel(engine: &Engine, body: &Value) -> Reply {
    let task = task_id_of(body)?;
    let store = engine.store();
    phase_of(&store, &task)?;
    let phase = store.apply(&task, Event::Cancel).map_err(|e| match e {
        StoreError::Refused(r) => Fault::new(Code::Conflict, "/body/task_id", r.to_string()),
        other => internal(&other),
    })?;
    Ok((
        false,
        json!({"task_id": task.as_str(), "phase": phase.as_str()}),
    ))
}

/// Parse, check the restatement and the VERIFY, then route, as dispatch would; admit nothing.
fn preview(engine: &Engine, body: &Value) -> Reply {
    let text = body
        .get("brief")
        .and_then(Value::as_str)
        .ok_or_else(|| Fault::new(Code::InvalidArgument, "/body/brief", "string required"))?;
    let not_eligible = |code: Code, message: String| {
        Ok((
            false,
            json!({"eligible": false, "refusal": code.name(), "message": message}),
        ))
    };
    let brief = match Brief::parse(text) {
        Ok(b) => b,
        Err(e) => return not_eligible(Code::InvalidArgument, e.to_string()),
    };
    if let Err(e) = brief.check_restatement() {
        return not_eligible(Code::InvalidArgument, e.to_string());
    }
    if let Err(e) = brief.check_verify() {
        return not_eligible(Code::InvalidArgument, e.to_string());
    }
    let wants_model = dispatcher::playbook(brief.get(BriefField::Verify))
        .iter()
        .any(|s| matches!(s.kind, StepKind::Generate { .. }));
    let client = OllamaClient::new(dispatcher::MODEL_URL);
    match dispatcher::route(&engine.cfg, &client, wants_model && engine.cfg.live) {
        Ok(sel) => Ok((false, json!({"eligible": true, "model": sel.model}))),
        Err(r) => not_eligible(Code::NoRoute, r.to_string()),
    }
}

/// The wire spellings of `task.resolve`'s `reason`, per resolution. Nothing else is accepted.
const ABANDON_REASONS: [(&str, AbandonReason); 8] = [
    ("brief_unreadable", AbandonReason::BriefUnreadable),
    (
        "route_refused",
        AbandonReason::RouteRefused { floor_unmet: false },
    ),
    (
        "route_refused_floor_unmet",
        AbandonReason::RouteRefused { floor_unmet: true },
    ),
    ("namespace_refused", AbandonReason::NamespaceRefused),
    ("work_dir_unavailable", AbandonReason::WorkDirUnavailable),
    ("head_unknown", AbandonReason::HeadUnknown),
    ("no_permit", AbandonReason::NoPermit),
    ("attempt_failed", AbandonReason::AttemptFailed),
];

/// The abandon reason when the operator names none.
const DEFAULT_ABANDON: AbandonReason = AbandonReason::AttemptFailed;

/// The quarantine reason when the operator names none.
const DEFAULT_QUARANTINE: QuarantineReason = QuarantineReason::EffectUnknownPermanent {
    rule: RecoveryRule::R10EffectAmbiguity,
};

/// The wire spelling of a quarantine reason: `effect_unknown_permanent_r01` .. `_r14`.
fn quarantine_reason(wire: &str) -> Option<QuarantineReason> {
    RecoveryRule::ALL
        .iter()
        .enumerate()
        .find(|(i, _)| wire == format!("effect_unknown_permanent_r{:02}", i + 1))
        .map(|(_, rule)| QuarantineReason::EffectUnknownPermanent { rule: *rule })
}

/// The typed `Resolution` for the wire's `resolution` and optional `reason`. An unknown
/// resolution, an unknown reason, or a reason of the other resolution is `invalid_argument`.
fn resolution_of(body: &Value) -> Result<Resolution, Fault> {
    let bad_reason = || {
        Fault::new(
            Code::InvalidArgument,
            "/body/reason",
            "a reason this resolution defines",
        )
    };
    let reason = match body.get("reason") {
        None | Some(Value::Null) => None,
        Some(Value::String(r)) => Some(r.as_str()),
        Some(_) => return Err(bad_reason()),
    };
    match body.get("resolution").and_then(Value::as_str) {
        Some("quarantine") => reason
            .map_or(Ok(DEFAULT_QUARANTINE), |r| {
                quarantine_reason(r).ok_or_else(bad_reason)
            })
            .map(Resolution::Quarantine),
        Some("abandon") => reason
            .map_or(Ok(DEFAULT_ABANDON), |r| {
                ABANDON_REASONS
                    .iter()
                    .find(|(wire, _)| *wire == r)
                    .map(|(_, reason)| *reason)
                    .ok_or_else(bad_reason)
            })
            .map(Resolution::Abandon),
        _ => Err(Fault::new(
            Code::InvalidArgument,
            "/body/resolution",
            "\"quarantine\" or \"abandon\"",
        )),
    }
}

/// `Store::apply(Resolve(..))`; the typed reason rides in the event.
fn resolve(engine: &Engine, body: &Value) -> Reply {
    let task = task_id_of(body)?;
    let resolution = resolution_of(body)?;
    let store = engine.store();
    phase_of(&store, &task)?;
    let phase = store
        .apply(&task, Event::Resolve(resolution))
        .map_err(|e| match e {
            StoreError::Refused(r) => Fault::new(Code::Conflict, "/body/task_id", r.to_string()),
            other => internal(&other),
        })?;
    eprintln!("resolve task={task} resolution={resolution:?}");
    Ok((
        false,
        json!({"task_id": task.as_str(), "phase": phase.as_str()}),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::testing::{BRIEF, engine};
    use crate::actions::{Outcome, answer, handle};
    use crate::wire;
    use hee4_core::{Observations, reconcile};

    fn submit_line(key: &str, brief: &str) -> String {
        wire::request("r1", "task.submit", Some(key), json!({ "brief": brief })).to_string()
    }

    #[test]
    fn submit_before_reconcile_is_not_ready_then_admitted_after()
    -> Result<(), Box<dyn std::error::Error>> {
        let e = engine("not-ready")?;
        let reply = handle(&e, &submit_line("k1", BRIEF));
        assert_eq!(reply["code"], "not_ready");
        assert_eq!(e.store().task_ids()?.len(), 0);
        let report = reconcile(&e.store(), &Observations::worker_absent())?;
        assert!(report.complete);
        let reply = handle(&e, &submit_line("k1", BRIEF));
        assert_eq!(reply["kind"], "result");
        assert_eq!(reply["body"]["phase"], "admitted");
        assert_eq!(reply["replayed"], false);
        let again = handle(&e, &submit_line("k1", BRIEF));
        assert_eq!(again["replayed"], true);
        assert_eq!(again["body"], reply["body"]);
        let other = handle(
            &e,
            &submit_line("k1", &BRIEF.replace("run test in the namespace", "other")),
        );
        assert_eq!(other["code"], "conflict");
        Ok(())
    }

    #[test]
    fn one_name_per_wire_refusal() -> Result<(), Box<dyn std::error::Error>> {
        let e = engine("names")?;
        reconcile(&e.store(), &Observations::worker_absent())?;
        assert_eq!(handle(&e, "{not json")["code"], "invalid_argument");
        assert_eq!(handle(&e, "[1]")["code"], "invalid_argument");
        let unknown = wire::request("r", "task.explode", None, json!({})).to_string();
        assert_eq!(handle(&e, &unknown)["code"], "unknown_action");
        let missing =
            wire::request("r", "task.get", None, json!({"task_id": "t-none"})).to_string();
        assert_eq!(handle(&e, &missing)["code"], "not_found");
        let no_key = wire::request("r", "task.submit", None, json!({ "brief": BRIEF })).to_string();
        assert_eq!(handle(&e, &no_key)["code"], "invalid_argument");
        let empty = submit_line(
            "k2",
            &BRIEF.replace("RESTATEMENT: run test in the namespace", "RESTATEMENT:"),
        );
        assert_eq!(handle(&e, &empty)["field"], "/body/brief");
        let health = handle(
            &e,
            &wire::request("r", "health", None, json!({})).to_string(),
        );
        assert_eq!(health["body"]["recovery_complete"], true);
        Ok(())
    }

    #[test]
    fn health_carries_schema_version_and_serve_cgroup_from_the_store()
    -> Result<(), Box<dyn std::error::Error>> {
        let e = engine("health-fields")?;
        let health = handle(
            &e,
            &wire::request("r", "health", None, json!({})).to_string(),
        );
        let body = &health["body"];
        assert_eq!(body["ok"], false);
        assert_eq!(body["recovery_complete"], false);
        assert_eq!(body["head_sha"], crate::HEAD);
        assert!(body["uptime_s"].is_u64(), "{body}");
        assert_eq!(body["schema_version"], json!(e.store().schema_version()?));
        assert!(
            body["schema_version"].as_i64().is_some_and(|v| v >= 2),
            "{body}"
        );
        assert_eq!(body["serve_cgroup"], json!(e.store().serve_cgroup()));
        assert!(
            body["serve_cgroup"].as_str().is_some_and(|s| !s.is_empty()),
            "{body}"
        );
        Ok(())
    }

    /// The VERIFY lines `check_verify` refuses at admission (V4-94); the parse refusal of a brief
    /// with no VERIFY field comes first and keeps its own message.
    #[test]
    fn vacuous_verify_is_refused_at_submit_and_preview() -> Result<(), Box<dyn std::error::Error>> {
        let e = engine("vacuous")?;
        reconcile(&e.store(), &Observations::worker_absent())?;
        let briefs_dir = e.work().join("briefs");
        for (i, verify) in ["sh: true", "/usr/bin/true", "sh: echo ok", "model: hi", ""]
            .iter()
            .enumerate()
        {
            let text = BRIEF.replace(
                "VERIFY: /usr/bin/test -d /usr",
                &format!("VERIFY: {verify}"),
            );
            let refused = handle(&e, &submit_line(&format!("v{i}"), &text));
            assert_eq!(refused["kind"], "error", "{verify:?}: {refused}");
            assert_eq!(refused["code"], "invalid_argument", "{verify:?}: {refused}");
            assert_eq!(refused["retry"], "never", "{verify:?}: {refused}");
            assert_eq!(refused["field"], "/body/brief", "{verify:?}: {refused}");
            let message = refused["message"].as_str().unwrap_or_default();
            assert!(message.contains("VERIFY"), "{verify:?}: {refused}");
            let preview = handle(&e, &line("task.preview", None, json!({ "brief": text })));
            assert_eq!(preview["body"]["eligible"], false, "{verify:?}: {preview}");
            assert_eq!(
                preview["body"]["refusal"], "invalid_argument",
                "{verify:?}: {preview}"
            );
            let message = preview["body"]["message"].as_str().unwrap_or_default();
            assert!(message.contains("VERIFY"), "{verify:?}: {preview}");
        }
        assert_eq!(e.store().task_ids()?.len(), 0);
        assert!(!briefs_dir.exists(), "{}", briefs_dir.display());

        // Order: parse -> restatement -> verify. No VERIFY field at all is the parse refusal.
        let no_verify = BRIEF.replace("VERIFY: /usr/bin/test -d /usr\n", "");
        let refused = handle(&e, &submit_line("nv", &no_verify));
        assert_eq!(refused["code"], "invalid_argument", "{refused}");
        assert_eq!(refused["field"], "/body/brief", "{refused}");
        let message = refused["message"].as_str().unwrap_or_default();
        assert!(message.contains("missing"), "{refused}");
        assert!(
            !message.contains("VERIFY is") && !message.contains("vacuous"),
            "{refused}"
        );
        // An empty RESTATEMENT with a vacuous VERIFY is named as the restatement, not VERIFY.
        let both = BRIEF
            .replace("VERIFY: /usr/bin/test -d /usr", "VERIFY: sh: true")
            .replace("RESTATEMENT: run test in the namespace", "RESTATEMENT:");
        let refused = handle(&e, &submit_line("both", &both));
        let message = refused["message"].as_str().unwrap_or_default();
        assert!(!message.contains("VERIFY"), "{refused}");
        assert_eq!(e.store().task_ids()?.len(), 0);

        let admitted = handle(&e, &submit_line("ok", BRIEF));
        assert_eq!(admitted["kind"], "result", "{admitted}");
        assert_eq!(admitted["body"]["phase"], "admitted", "{admitted}");
        assert_eq!(e.store().task_ids()?.len(), 1);
        Ok(())
    }

    #[test]
    fn get_list_cancel_read_the_ledger() -> Result<(), Box<dyn std::error::Error>> {
        let e = engine("glc")?;
        reconcile(&e.store(), &Observations::worker_absent())?;
        let id = handle(&e, &submit_line("k3", BRIEF))["body"]["task_id"].clone();
        let get = handle(
            &e,
            &wire::request("r", "task.get", None, json!({ "task_id": id })).to_string(),
        );
        assert_eq!(get["body"]["events"], 1);
        assert_eq!(get["body"]["last_receipt_hash"], Value::Null);
        let list = handle(
            &e,
            &wire::request("r", "task.list", None, json!({})).to_string(),
        );
        assert_eq!(list["body"]["tasks"][0]["phase"], "admitted");
        let c = wire::request("r", "task.cancel", Some("c1"), json!({ "task_id": id })).to_string();
        assert_eq!(handle(&e, &c)["body"]["phase"], "cancellation_requested");
        assert_eq!(handle(&e, &c)["body"]["phase"], "cancellation_requested");
        Ok(())
    }

    fn line(action: &str, key: Option<&str>, body: Value) -> String {
        wire::request("r", action, key, body).to_string()
    }

    #[test]
    fn preview_admits_nothing_and_names_its_refusal() -> Result<(), Box<dyn std::error::Error>> {
        let e = engine("preview")?;
        let ok = handle(&e, &line("task.preview", None, json!({ "brief": BRIEF })));
        assert_eq!(ok["body"], json!({"eligible": true, "model": "m:1"}));
        let bad = BRIEF.replace("RESTATEMENT: run test in the namespace", "RESTATEMENT:");
        let no = handle(&e, &line("task.preview", None, json!({ "brief": bad })));
        assert_eq!(no["body"]["eligible"], false);
        assert_eq!(no["body"]["refusal"], "invalid_argument");
        assert_eq!(e.store().task_ids()?.len(), 0);
        Ok(())
    }

    #[test]
    fn resolve_goes_through_apply() -> Result<(), Box<dyn std::error::Error>> {
        let e = engine("resolve")?;
        reconcile(&e.store(), &Observations::worker_absent())?;
        let id = handle(&e, &submit_line("k4", BRIEF))["body"]["task_id"].clone();
        let q = json!({"task_id": id, "resolution": "quarantine", "reason": "effect_unknown_permanent_r10"});
        let r = handle(&e, &line("task.resolve", Some("q1"), q.clone()));
        assert_eq!(r["body"]["phase"], "blocked");
        let no_key = handle(&e, &line("task.resolve", None, q));
        assert_eq!(no_key["field"], "/idempotency_key");
        let bad = json!({"task_id": id, "resolution": "retry", "reason": "attempt_failed"});
        assert_eq!(
            handle(&e, &line("task.resolve", Some("q2"), bad))["field"],
            "/body/resolution"
        );
        let a = json!({"task_id": id, "resolution": "abandon", "reason": "attempt_failed"});
        let r = handle(&e, &line("task.resolve", Some("a1"), a.clone()));
        assert_eq!(r["body"]["phase"], "abandoned");
        let again = handle(&e, &line("task.resolve", Some("a2"), a));
        assert_eq!(again["code"], "conflict");
        Ok(())
    }

    #[test]
    fn resolve_reason_table_is_typed() {
        let r = |res: &str, reason: Option<&str>| {
            let mut b = json!({"resolution": res});
            if let Some(x) = reason {
                b["reason"] = json!(x);
            }
            resolution_of(&b)
        };
        for (wire, want) in ABANDON_REASONS {
            assert_eq!(
                r("abandon", Some(wire)).ok(),
                Some(Resolution::Abandon(want))
            );
        }
        assert_eq!(
            r("abandon", None).ok(),
            Some(Resolution::Abandon(DEFAULT_ABANDON))
        );
        assert_eq!(
            r("quarantine", None).ok(),
            Some(Resolution::Quarantine(DEFAULT_QUARANTINE))
        );
        for (i, rule) in RecoveryRule::ALL.iter().enumerate() {
            let wire = format!("effect_unknown_permanent_r{:02}", i + 1);
            assert_eq!(
                r("quarantine", Some(&wire)).ok(),
                Some(Resolution::Quarantine(
                    QuarantineReason::EffectUnknownPermanent { rule: *rule }
                ))
            );
        }
        for bad in [
            r("abandon", Some("operator")),
            r("abandon", Some("")),
            r("abandon", Some("effect_unknown_permanent_r01")),
            r("quarantine", Some("attempt_failed")),
            r("retry", None),
        ] {
            assert!(bad.is_err());
        }
    }

    #[test]
    fn subscribe_parses_since_seq() -> Result<(), Box<dyn std::error::Error>> {
        let e = engine("subscribe")?;
        let ok = answer(&e, &line("events.subscribe", None, json!({"since_seq": 7})));
        assert!(matches!(ok, Outcome::Subscribe { since_seq: 7, .. }));
        let Outcome::Subscribe { ack, .. } = ok else {
            return Err("not a subscription".into());
        };
        assert_eq!(ack["kind"], "result", "{ack}");
        assert_eq!(ack["body"]["since_seq"], 7, "{ack}");
        assert_eq!(ack["body"]["stream"], "events", "{ack}");
        assert_eq!(ack["body"]["epoch"], json!(e.store().epoch()?), "{ack}");
        assert_eq!(
            ack["body"]["high_water"],
            json!(e.store().event_high_water()?),
            "{ack}"
        );
        let bad = handle(
            &e,
            &line("events.subscribe", None, json!({"since_seq": -1})),
        );
        assert_eq!(bad["code"], "invalid_argument");
        assert_eq!(bad["field"], "/body/since_seq");
        let bad_epoch = handle(
            &e,
            &line(
                "events.subscribe",
                None,
                json!({"since_seq": 0, "epoch": 7}),
            ),
        );
        assert_eq!(bad_epoch["code"], "invalid_argument", "{bad_epoch}");
        assert_eq!(bad_epoch["field"], "/body/epoch", "{bad_epoch}");
        Ok(())
    }

    /// R13 through `Store::cursor_check`: each verdict is one `resync_required` by name, at the
    /// member it faults; a null epoch is unchecked; none authorises a replay.
    #[test]
    fn subscribe_epoch_cursor_is_checked_by_verdict() -> Result<(), Box<dyn std::error::Error>> {
        let e = engine("cursor")?;
        reconcile(&e.store(), &Observations::worker_absent())?;
        assert_eq!(handle(&e, &submit_line("c1", BRIEF))["kind"], "result");
        let epoch = e.store().epoch()?;
        let high_water = e.store().event_high_water()?;
        assert!(high_water >= 1);
        let sub = |body: Value| handle(&e, &line("events.subscribe", None, body));

        let foreign = sub(json!({"since_seq": 0, "epoch": "not-this-ledger"}));
        assert_eq!(foreign["code"], "resync_required", "{foreign}");
        assert_eq!(foreign["field"], "/body/epoch", "{foreign}");
        assert_eq!(foreign["because"], "epoch_changed", "{foreign}");
        assert_eq!(foreign["retry"], "never", "{foreign}");

        let future = sub(json!({"since_seq": high_water + 1000, "epoch": epoch}));
        assert_eq!(future["code"], "resync_required", "{future}");
        assert_eq!(future["field"], "/body/since_seq", "{future}");
        assert_eq!(future["because"], "future_sequence", "{future}");

        let boundary = answer(
            &e,
            &line(
                "events.subscribe",
                None,
                json!({"since_seq": high_water, "epoch": epoch}),
            ),
        );
        assert!(
            matches!(&boundary, Outcome::Subscribe { since_seq, .. } if u64::try_from(*since_seq).ok() == Some(high_water)),
            "{boundary:?}"
        );

        // A null epoch is the legacy cursor: unchecked even past the high water.
        let legacy = answer(
            &e,
            &line(
                "events.subscribe",
                None,
                json!({"since_seq": high_water + 1000}),
            ),
        );
        assert!(matches!(legacy, Outcome::Subscribe { .. }), "{legacy:?}");

        e.store().mark_restored_from(&epoch)?;
        let prior = sub(json!({"since_seq": 0, "epoch": epoch}));
        assert_eq!(prior["code"], "resync_required", "{prior}");
        assert_eq!(prior["field"], "/body/epoch", "{prior}");
        assert_eq!(prior["because"], "prior_epoch_of_restore", "{prior}");
        Ok(())
    }
}
