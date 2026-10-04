//! The four skeleton actions plus `health`, behind one `handle(line) -> reply` door.
//!
//! Every state change goes through `Store::admit` or `Store::apply`. A mutating action before
//! startup reconcile completed is refused `not_ready`, read from the ledger's own flag.

use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};
use std::time::Instant;

use hee4_contracts::{Brief, Event, Phase, Sha256Hex, TaskId};
use hee4_core::{OperationKey, Store, StoreError};
use serde_json::{Value, json};

use crate::wire::{self, Code, Fault, Request};

/// The engine one `serve` process holds: the ledger (one connection, one process) and paths.
#[derive(Debug)]
pub struct Engine {
    store: Mutex<Store>,
    work: PathBuf,
    started: Instant,
    principal: String,
}

impl Engine {
    /// Wrap an opened store. `work` is the work root (`<work>/<task_id>` per task).
    #[must_use]
    pub fn new(store: Store, work: PathBuf) -> Self {
        Self {
            store: Mutex::new(store),
            work,
            started: Instant::now(),
            principal: crate::process_uid()
                .map_or_else(|| "uid:unknown".into(), |u| format!("uid:{u}")),
        }
    }

    /// The ledger, locked for one call. A poisoned lock is still the same ledger: every write
    /// in it is its own committed transaction.
    pub fn store(&self) -> MutexGuard<'_, Store> {
        self.store
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// The work root.
    #[must_use]
    pub fn work(&self) -> &Path {
        &self.work
    }

    /// Where a task's admitted brief text lives (the ledger has no brief column; DC proposal).
    #[must_use]
    pub fn brief_path(&self, task: &TaskId) -> PathBuf {
        self.work.join("briefs").join(format!("{task}.brief"))
    }
}

/// Answer one request line with one reply frame.
#[must_use]
pub fn handle(engine: &Engine, line: &str) -> Value {
    match wire::parse(line) {
        Err((id, fault)) => wire::error(&id, &fault),
        Ok(req) => match dispatch(engine, &req) {
            Ok((replayed, body)) => wire::result(&req.request_id, replayed, body),
            Err(fault) => wire::error(&req.request_id, &fault),
        },
    }
}

type Reply = Result<(bool, Value), Fault>;

fn dispatch(engine: &Engine, req: &Request) -> Reply {
    let mutating = match req.action.as_str() {
        "health" | "task.get" | "task.list" => false,
        "task.submit" | "task.cancel" => true,
        _ => {
            return Err(Fault::new(
                Code::UnknownAction,
                "/action",
                "not in the skeleton catalogue",
            ));
        }
    };
    if req.action_version != 1 {
        return Err(Fault::new(
            Code::UnsupportedActionVersion,
            "/action_version",
            "only 1",
        ));
    }
    if mutating {
        if req.idempotency_key.is_none() {
            return Err(Fault::new(
                Code::InvalidArgument,
                "/idempotency_key",
                "required for a mutating action",
            ));
        }
        if !engine
            .store()
            .recovery_complete()
            .map_err(|e| internal(&e))?
        {
            return Err(Fault::new(
                Code::NotReady,
                "/action",
                "startup reconcile has not completed",
            ));
        }
    }
    match req.action.as_str() {
        "health" => health(engine),
        "task.submit" => submit(engine, req),
        "task.get" => get(engine, &req.body),
        "task.list" => list(engine),
        _ => cancel(engine, &req.body),
    }
}

fn internal(e: &StoreError) -> Fault {
    Fault::new(Code::Internal, "/", e.to_string())
}

fn health(engine: &Engine) -> Reply {
    let complete = engine
        .store()
        .recovery_complete()
        .map_err(|e| internal(&e))?;
    Ok((
        false,
        json!({
            "ok": complete,
            "head_sha": crate::HEAD,
            "recovery_complete": complete,
            "uptime_s": engine.started.elapsed().as_secs(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use hee4_core::{Observations, reconcile};

    const BRIEF: &str = "GOAL: g\nSCOPE: s\nCONTEXT: c\nACCEPTANCE: a\nVERIFY: /usr/bin/true\nTIMEBOX: 10s\nFORBIDDEN: f\nREPORT: r\nSTANDING: s\nRECON: r\nRESTATEMENT: run true\n";

    fn engine(name: &str) -> Result<Engine, Box<dyn std::error::Error>> {
        let dir = PathBuf::from(env!("OUT_DIR")).join(format!("actions-{name}"));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir)?;
        Ok(Engine::new(
            Store::open(&dir.join("ledger.sqlite3"))?,
            dir.join("work"),
        ))
    }

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
        let other = handle(&e, &submit_line("k1", &BRIEF.replace("run true", "other")));
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
            &BRIEF.replace("RESTATEMENT: run true", "RESTATEMENT:"),
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
}
