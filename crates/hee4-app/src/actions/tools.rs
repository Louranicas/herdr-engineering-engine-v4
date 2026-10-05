//! `tools.list` and `tools.inspect`, served from `hee4_contracts::catalogue` and nothing else:
//! no store access, no second id list. `tools.list` pages the catalogue by id through
//! `page.rs`; `tools.inspect` describes one entry, held ids included (a held id is catalogued,
//! not served). The schema digests are descriptor digests: SHA-256 over the canonical JSON of
//! `{action, version, fields}` for the request and result member lists, and of the error
//! frame's member list (`wire::ERROR_MEMBERS`).

use hee4_contracts::bounds::{MAX_DEADLINE_MS, MAX_QUERY_BYTES};
use hee4_contracts::catalogue::{self, Action, CATALOGUE, Owner};
use hee4_contracts::{Sha256Hex, canonical_json};
use serde_json::{Value, json};

use super::page::{Cursor, PageIn, PageOut, page};
use super::registry::Family;
use super::{Answer, Engine};
use crate::wire::{self, Code, Fault, Request};

/// The `Actions` family: the catalogue's two reads.
pub const FAMILY: Family = Family {
    owner: Owner::Actions,
    handlers: &[("tools.list", list), ("tools.inspect", inspect)],
    on_serve_start: None,
};

/// SHA-256 over the canonical JSON of `{action, version: 1, fields}`.
fn descriptor(action: &str, fields: &[&str]) -> String {
    let json = json!({"action": action, "version": 1, "fields": fields});
    Sha256Hex::digest(canonical_json(&json).as_bytes()).to_string()
}

fn item(action: &Action) -> Value {
    json!({
        "id": action.id,
        "version": action.version,
        "purpose": action.purpose,
        "effect": action.effect.wire_name(),
    })
}

/// `{query: null|string, page}` -> `{catalogue_revision, page: {items, cursor}}`.
fn list(engine: &Engine, req: &Request) -> Result<Answer, Fault> {
    let query = match req.body.get("query") {
        None | Some(Value::Null) => "",
        Some(Value::String(s)) => s.as_str(),
        Some(_) => {
            return Err(Fault::new(
                Code::InvalidArgument,
                "/body/query",
                "string or null",
            ));
        }
    };
    if query.len() > MAX_QUERY_BYTES {
        return Err(Fault::new(
            Code::InvalidArgument,
            "/body/query",
            format!("{} bytes, at most {MAX_QUERY_BYTES}", query.len()),
        ));
    }
    let page_in = PageIn::parse(req.body.get("page"))?;
    let filter = Sha256Hex::digest(query.as_bytes());
    let boot = engine.boot();
    let after = match &page_in.cursor {
        Some(cursor) => {
            cursor.resumes(boot, &filter)?;
            Some(cursor.after_key.as_str())
        }
        None => None,
    };
    let mut matched: Vec<&Action> = CATALOGUE
        .iter()
        .filter(|a| a.id.contains(query) || a.purpose.contains(query))
        .collect();
    matched.sort_by_key(|a| a.id);
    let (items, more) = page(&matched, |a| a.id, after, page_in.limit);
    let cursor = more.then(|| items.last()).flatten().map(|last| Cursor {
        after_key: last.id.to_owned(),
        boot,
        filter_sha256: filter,
    });
    let out = PageOut {
        items: items.iter().map(|a| item(a)).collect(),
        cursor,
    };
    Ok(Answer::Frame(
        false,
        json!({
            "catalogue_revision": catalogue::revision().to_string(),
            "page": out.to_json(),
        }),
    ))
}

/// `{action, version}` -> the entry in full, with its catalogue `scope` (wire name) and
/// `served`: whether this binary's composed registry serves it (`Registry::serve`, the same
/// lookup whose miss is dispatch's `unavailable`). An unknown id is `unknown_action` at
/// `/body/action` (Error map F-2; DC proposal in FLOW.md); a known id at another version is
/// `unsupported_action_version` at `/body/version`.
fn inspect(engine: &Engine, req: &Request) -> Result<Answer, Fault> {
    let action = req
        .body
        .get("action")
        .and_then(Value::as_str)
        .ok_or_else(|| Fault::new(Code::InvalidArgument, "/body/action", "string required"))?;
    let version = req
        .body
        .get("version")
        .and_then(Value::as_u64)
        .ok_or_else(|| {
            Fault::new(
                Code::InvalidArgument,
                "/body/version",
                "unsigned integer required",
            )
        })?;
    let entry = catalogue::find(action)
        .ok_or_else(|| Fault::new(Code::UnknownAction, "/body/action", "not in the catalogue"))?;
    if version != u64::from(entry.version) {
        return Err(Fault::new(
            Code::UnsupportedActionVersion,
            "/body/version",
            format!("only {}", entry.version),
        ));
    }
    Ok(Answer::Frame(
        false,
        json!({
            "action": entry.id,
            "version": entry.version,
            "purpose": entry.purpose,
            "effect": entry.effect.wire_name(),
            "request_schema_sha256": descriptor(entry.id, entry.request_fields),
            "result_schema_sha256": descriptor(entry.id, entry.result_fields),
            "error_schema_sha256": descriptor(entry.id, &wire::ERROR_MEMBERS),
            "max_request_bytes": engine.budgets().socket.frame_bytes,
            "max_deadline_ms": MAX_DEADLINE_MS,
            "readback_action": entry.readback_action,
            "scope": entry.scope.wire_name(),
            "served": engine.registry().serve(entry).is_some(),
        }),
    ))
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use hee4_contracts::catalogue::{self, CATALOGUE, Effect, Owner};
    use serde_json::{Value, json};

    use crate::actions::testing::engine;
    use crate::actions::{Engine, handle};
    use crate::wire;

    fn call(e: &Engine, action: &str, body: Value) -> Value {
        handle(e, &wire::request("r", action, None, body).to_string())
    }

    #[test]
    fn list_with_limit_5_is_disjoint_and_complete_across_continuations()
    -> Result<(), Box<dyn std::error::Error>> {
        let e = engine("tools-list")?;
        let mut seen: Vec<String> = Vec::new();
        let mut cursor = Value::Null;
        let mut pages = 0;
        loop {
            let reply = call(
                &e,
                "tools.list",
                json!({"page": {"limit": 5, "cursor": cursor}}),
            );
            assert_eq!(reply["kind"], "result", "{reply}");
            assert_eq!(
                reply["body"]["catalogue_revision"],
                catalogue::revision().to_string()
            );
            let items = reply["body"]["page"]["items"]
                .as_array()
                .cloned()
                .unwrap_or_default();
            assert!(items.len() <= 5);
            for it in &items {
                seen.push(it["id"].as_str().unwrap_or_default().to_owned());
                assert!(it["effect"].is_string());
                assert!(it["purpose"].is_string());
                assert_eq!(it["version"], 1);
            }
            pages += 1;
            cursor = reply["body"]["page"]["cursor"].clone();
            if cursor.is_null() {
                break;
            }
            assert!(pages <= CATALOGUE.len());
        }
        let distinct: BTreeSet<&str> = seen.iter().map(String::as_str).collect();
        assert_eq!(
            distinct.len(),
            seen.len(),
            "a page repeated an id: {seen:?}"
        );
        let all: BTreeSet<&str> = CATALOGUE.iter().map(|a| a.id).collect();
        assert_eq!(distinct, all);
        assert_eq!(pages, CATALOGUE.len().div_ceil(5));
        Ok(())
    }

    #[test]
    fn empty_match_is_a_result_with_no_items_and_no_cursor()
    -> Result<(), Box<dyn std::error::Error>> {
        let e = engine("tools-empty")?;
        let reply = call(&e, "tools.list", json!({"query": "zzz-no-such-text"}));
        assert_eq!(reply["kind"], "result");
        assert_eq!(reply["body"]["page"]["items"], json!([]));
        assert_eq!(reply["body"]["page"]["cursor"], Value::Null);
        let filtered = call(&e, "tools.list", json!({"query": "task."}));
        let items = filtered["body"]["page"]["items"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        assert_ne!(items.len(), 0);
        assert!(items.iter().all(|it| {
            it["id"].as_str().is_some_and(|s| s.contains("task."))
                || it["purpose"].as_str().is_some_and(|s| s.contains("task."))
        }));
        Ok(())
    }

    #[test]
    fn list_refusals_name_their_pointer() -> Result<(), Box<dyn std::error::Error>> {
        let e = engine("tools-refuse")?;
        let long = "q".repeat(hee4_contracts::bounds::MAX_QUERY_BYTES + 1);
        let over = call(&e, "tools.list", json!({"query": long}));
        assert_eq!(over["code"], "invalid_argument");
        assert_eq!(over["field"], "/body/query");
        let message = over["message"].as_str().unwrap_or_default();
        assert!(
            message.contains("257") && message.contains("256"),
            "{message}"
        );
        let typed = call(&e, "tools.list", json!({"query": 5}));
        assert_eq!(typed["field"], "/body/query");
        let limit = call(&e, "tools.list", json!({"page": {"limit": 0}}));
        assert_eq!(limit["code"], "invalid_argument");
        assert_eq!(limit["field"], "/body/page/limit");
        let first = call(&e, "tools.list", json!({"page": {"limit": 3}}));
        let mut stale = first["body"]["page"]["cursor"].clone();
        stale["boot"] = json!(e.boot().wrapping_add(1));
        let resync = call(
            &e,
            "tools.list",
            json!({"page": {"limit": 3, "cursor": stale}}),
        );
        assert_eq!(resync["code"], "resync_required");
        assert_eq!(resync["because"], "epoch moved");
        let mut moved = first["body"]["page"]["cursor"].clone();
        moved["filter_sha256"] = json!(hee4_contracts::Sha256Hex::digest(b"other").to_string());
        let resync = call(
            &e,
            "tools.list",
            json!({"page": {"limit": 3, "cursor": moved}}),
        );
        assert_eq!(resync["because"], "filter moved");
        Ok(())
    }

    #[test]
    fn inspect_describes_an_entry_from_the_catalogue() -> Result<(), Box<dyn std::error::Error>> {
        let e = engine("tools-inspect")?;
        let admission = CATALOGUE
            .iter()
            .find(|a| a.effect == Effect::DurableAdmission)
            .map(|a| a.id)
            .unwrap_or_default();
        let reply = call(
            &e,
            "tools.inspect",
            json!({"action": admission, "version": 1}),
        );
        assert_eq!(reply["kind"], "result", "{reply}");
        assert_eq!(reply["body"]["effect"], "durable_admission");
        assert_eq!(reply["body"]["readback_action"], "task.get");
        assert_eq!(
            reply["body"]["max_request_bytes"],
            hee4_contracts::Budgets::DEFAULT.socket.frame_bytes
        );
        assert_eq!(
            reply["body"]["max_deadline_ms"],
            hee4_contracts::bounds::MAX_DEADLINE_MS
        );
        for digest in [
            "request_schema_sha256",
            "result_schema_sha256",
            "error_schema_sha256",
        ] {
            assert_eq!(reply["body"][digest].as_str().map(str::len), Some(64));
        }
        let app = CATALOGUE
            .iter()
            .find(|a| a.owner == Owner::App)
            .map(|a| a.id)
            .unwrap_or_default();
        let health = call(&e, "tools.inspect", json!({"action": app, "version": 1}));
        assert_eq!(health["body"]["readback_action"], Value::Null);
        let held = call(
            &e,
            "tools.inspect",
            json!({"action": "judge.inspect", "version": 1}),
        );
        assert_eq!(held["kind"], "result");
        assert_eq!(held["body"]["scope"], "held");
        assert_eq!(held["body"]["served"], false);
        let unknown = call(
            &e,
            "tools.inspect",
            json!({"action": "no.such.action", "version": 1}),
        );
        assert_eq!(unknown["code"], "unknown_action");
        assert_eq!(unknown["field"], "/body/action");
        let v2 = call(&e, "tools.inspect", json!({"action": app, "version": 2}));
        assert_eq!(v2["code"], "unsupported_action_version");
        assert_eq!(v2["field"], "/body/version");
        assert_eq!(
            call(&e, "tools.inspect", json!({"version": 1}))["field"],
            "/body/action"
        );
        assert_eq!(
            call(&e, "tools.inspect", json!({"action": app}))["field"],
            "/body/version"
        );
        Ok(())
    }

    #[test]
    fn inspect_says_scope_and_whether_this_binary_serves_it()
    -> Result<(), Box<dyn std::error::Error>> {
        let e = engine("tools-served")?;
        let composed = call(
            &e,
            "tools.inspect",
            json!({"action": "tools.inspect", "version": 1}),
        );
        assert_eq!(composed["kind"], "result", "{composed}");
        assert_eq!(composed["body"]["served"], true);
        assert_eq!(composed["body"]["scope"], "v40");
        let thread = call(
            &e,
            "tools.inspect",
            json!({"action": "thread.get", "version": 1}),
        );
        assert_eq!(thread["kind"], "result", "{thread}");
        assert_eq!(thread["body"]["served"], false);
        assert_eq!(thread["body"]["scope"], "v42");
        // `served` is the registry's own answer for every entry, and an unserved one invoked is
        // `unavailable` at `/action` with its scope's `because` (served handlers are not run here).
        for a in &CATALOGUE {
            let inspected = call(&e, "tools.inspect", json!({"action": a.id, "version": 1}));
            let served = inspected["body"]["served"].as_bool();
            assert_eq!(
                served,
                Some(e.registry().serve(a).is_some()),
                "{}: {inspected}",
                a.id
            );
            if served == Some(false) {
                let invoked = call(&e, a.id, json!({}));
                assert_eq!(invoked["code"], "unavailable", "{}: {invoked}", a.id);
                assert_eq!(invoked["field"], "/action");
                assert_eq!(invoked["because"], a.scope.because());
            }
            assert_eq!(inspected["body"]["scope"], a.scope.wire_name());
        }
        Ok(())
    }

    #[test]
    fn neither_action_touches_the_ledger() -> Result<(), Box<dyn std::error::Error>> {
        let e = engine("tools-ledger")?;
        let events_before = e.store().event_count()?;
        let tasks_before = e.store().task_ids()?;
        call(&e, "tools.list", json!({}));
        call(
            &e,
            "tools.inspect",
            json!({"action": "judge.inspect", "version": 1}),
        );
        assert_eq!(e.store().event_count()?, events_before);
        assert_eq!(e.store().task_ids()?, tasks_before);
        Ok(())
    }
}
