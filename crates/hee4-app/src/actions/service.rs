//! The service family (K6, owner `Service`): `service.inspect`, `service.probe` and
//! `service.action`. The runner (K5, [`crate::service_runner`]) spawns busctl and returns values;
//! these handlers commit them through K1 (`Store::service_*`) and never touch SQL. Serve-start
//! work (the `service_facts` seed and the busctl digest pin) is this family's `on_serve_start`
//! hook, the single place it happens.

use std::sync::OnceLock;

use hee4_contracts::Sha256Hex;
use hee4_contracts::catalogue::Owner;
use hee4_core::service::{Expected, Seed, ServiceError, ServiceFact, StaleClaim};
use hee4_core::{OperationKey, StoreError};
use serde_json::{Value, json};

use super::registry::{Family, StartFault};
use super::{Answer, Engine, Reply, internal};
use crate::service_runner::{
    ActFault, BUSCTL_SHA256_ENV, BusctlRunner, ProbeBudget, ProbeFault, ProbeId, ServiceRunner,
    UnitAction,
};
use crate::wire::{Code, Fault, Request};

/// The managed services, seeded into `service_facts` at serve start: the one list of service
/// ids. `owner_id` is `deploy` for all three (the installer record's sense). Only `drive` is
/// actable: `service.action` on the engine itself or the model is refused server-side.
pub const SEEDS: &[Seed<'static>] = &[
    Seed {
        service_id: "self",
        owner_id: "deploy",
        unit_id: "hee4.service",
        actable: false,
    },
    Seed {
        service_id: "model",
        owner_id: "deploy",
        unit_id: "ollama.service",
        actable: false,
    },
    Seed {
        service_id: "drive",
        owner_id: "deploy",
        unit_id: "hee4-drive.service",
        actable: true,
    },
];

/// The `probe_version` this release serves.
pub const PROBE_VERSION: u64 = 1;

/// The runner the hook builds once per process; unset before `serve` starts.
static RUNNER: OnceLock<BusctlRunner> = OnceLock::new();

/// The service handlers: owner `Service`.
pub const FAMILY: Family = Family {
    owner: Owner::Service,
    handlers: &[
        ("service.inspect", |engine, req| frame(inspect(engine, req))),
        ("service.probe", |engine, req| {
            frame(probe(engine, req, started()))
        }),
        ("service.action", |engine, req| {
            frame(action(engine, req, started()))
        }),
    ],
    on_serve_start: Some(start),
};

fn frame(reply: Reply) -> Result<Answer, Fault> {
    reply.map(|(replayed, body)| Answer::Frame(replayed, body))
}

/// The runner `start` built, if `serve` ran the hook in this process.
fn started() -> Option<&'static dyn ServiceRunner> {
    RUNNER.get().map(|r| r as &dyn ServiceRunner)
}

/// The runner, consulted only after every request check passed: an engine built without
/// `serve` (a test) answers a well-formed request `unavailable` because the runner is absent.
fn runner_of(runner: Option<&dyn ServiceRunner>) -> Result<&dyn ServiceRunner, Fault> {
    runner.ok_or_else(|| {
        Fault::new(Code::Unavailable, "/", "the service runner is not started")
            .with_because("service runner not started")
    })
}

/// Seed `service_facts`, pin the busctl digest (env, else measured now), measure the busctl
/// version through the door, and print one `busctl_sha256=<hex> source=env|measured` line.
fn start(engine: &Engine) -> Result<(), StartFault> {
    let seeded = engine.store().service_seed(SEEDS).map_err(|e| {
        eprintln!("service family start: seed failed: {e}");
        StartFault::Refused {
            owner: Owner::Service,
            reason: "service_facts seed failed",
        }
    })?;
    let measured = BusctlRunner::measure_digest().ok();
    let env = std::env::var(BUSCTL_SHA256_ENV).ok();
    let (pin, source) = match &env {
        Some(text) => (text.parse::<Sha256Hex>().ok(), "env"),
        None => (measured, "measured"),
    };
    let version = match BusctlRunner::measure_version(engine.work()) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("service family start: busctl version unmeasured: {e}");
            "unmeasured".parse().map_err(|_| StartFault::Refused {
                owner: Owner::Service,
                reason: "busctl version token",
            })?
        }
    };
    let bus = BusctlRunner::bus_from_env();
    let show = |d: Option<Sha256Hex>| d.map_or_else(|| "unmeasured".to_owned(), |d| d.to_string());
    eprintln!(
        "service family start busctl_sha256={} source={source} measured={} busctl_version={version} bus={} seeded={seeded}",
        env.as_deref().map_or_else(|| show(pin), str::to_owned),
        show(measured),
        bus.as_deref()
            .map_or_else(|| "absent".to_owned(), |b| b.display().to_string()),
    );
    match engine.store().service_stale_claims() {
        Ok(stale) => stale
            .iter()
            .for_each(|c| report_stale("service family start", c)),
        Err(e) => eprintln!("service family start: stale claims unread: {e}"),
    }
    let _ = RUNNER.set(BusctlRunner::new(
        engine.work().to_path_buf(),
        bus,
        pin,
        version,
    ));
    Ok(())
}

/// One line naming a stale act claim: at serve start (recovery) and when an action supersedes it.
fn report_stale(prefix: &str, stale: &StaleClaim) {
    eprintln!(
        "{prefix} stale_claim service_id={} operation_id={} generation={} age_ms={} reason={}",
        stale.claim.service_id,
        stale.claim.operation_id,
        stale.claim.generation,
        stale.age_ms,
        stale.reason.as_str()
    );
}

fn bad(field: &'static str, message: impl Into<String>) -> Fault {
    Fault::new(Code::InvalidArgument, field, message)
}

fn str_of<'a>(body: &'a Value, name: &str, field: &'static str) -> Result<&'a str, Fault> {
    body.get(name)
        .and_then(Value::as_str)
        .ok_or_else(|| bad(field, "string required"))
}

fn u64_of(body: &Value, name: &str, field: &'static str) -> Result<u64, Fault> {
    body.get(name)
        .and_then(Value::as_u64)
        .ok_or_else(|| bad(field, "unsigned integer required"))
}

fn op_key(engine: &Engine, req: &Request) -> OperationKey {
    OperationKey {
        principal: engine.principal.clone(),
        action: req.action.clone(),
        version: req.action_version,
        idem_key: req.idempotency_key.clone().unwrap_or_default(),
    }
}

fn body_bytes(body: &Value) -> Result<Vec<u8>, Fault> {
    serde_json::to_vec(body).map_err(|e| Fault::new(Code::Internal, "/", e.to_string()))
}

fn json_of<T: serde::Serialize>(v: &T) -> Result<Value, Fault> {
    serde_json::to_value(v).map_err(|e| Fault::new(Code::Internal, "/", e.to_string()))
}

fn key_conflict() -> Fault {
    Fault::new(
        Code::Conflict,
        "/idempotency_key",
        "key recorded with other bytes",
    )
}

fn store_fault(e: StoreError) -> Fault {
    match e {
        StoreError::Conflict(_) => key_conflict(),
        other => internal(&other),
    }
}

fn not_found() -> Fault {
    Fault::new(Code::NotFound, "/body/service_id", "no such service")
}

fn stale(current: u64) -> Fault {
    Fault::new(
        Code::StaleGeneration,
        "/precondition/generation",
        "the service has moved",
    )
    .with_generation(current)
}

fn owner_conflict() -> Fault {
    Fault::new(
        Code::Conflict,
        "/body/expected_owner_sha256",
        "owner digest mismatch",
    )
}

fn not_actable() -> Fault {
    Fault::new(
        Code::Forbidden,
        "/body/service_id",
        "this service is seeded not actable",
    )
    .with_because("service not actable")
}

fn claim_held() -> Fault {
    Fault::new(
        Code::Conflict,
        "/body/service_id",
        "another act holds this service's claim",
    )
    .with_because("service claim held")
}

fn service_fault(e: ServiceError) -> Fault {
    match e {
        ServiceError::ClaimHeld(_) => claim_held(),
        ServiceError::NotActable(_) => not_actable(),
        ServiceError::Store(e) => store_fault(e),
        ServiceError::UnknownService(_) => not_found(),
        ServiceError::StaleGeneration { current } => stale(current),
        ServiceError::OwnerMismatch => owner_conflict(),
    }
}

fn probe_fault(e: &ProbeFault) -> Fault {
    let unavailable =
        |because| Fault::new(Code::Unavailable, "/", e.to_string()).with_because(because);
    match e {
        ProbeFault::Digest { .. } => unavailable("busctl digest"),
        ProbeFault::HeadUnknown => unavailable("head unknown"),
        ProbeFault::NoBus => unavailable("user bus absent"),
        ProbeFault::StdoutOverBound { .. } => {
            Fault::new(Code::ResourceExhausted, "/", e.to_string())
        }
        ProbeFault::Door(_) | ProbeFault::Field(_) => {
            Fault::new(Code::Internal, "/", e.to_string())
        }
    }
}

fn fact_of(engine: &Engine, service_id: &str) -> Result<ServiceFact, Fault> {
    engine
        .store()
        .service_get(service_id)
        .map_err(store_fault)?
        .ok_or_else(not_found)
}

/// `service.inspect`: the committed facts, and one operation by key when asked. Never probes.
fn inspect(engine: &Engine, req: &Request) -> Reply {
    let body = &req.body;
    let service_id = str_of(body, "service_id", "/body/service_id")?;
    let selector = match body.get("operation") {
        None => {
            return Err(bad(
                "/body/operation",
                "required: null or {source_action, idempotency_key}",
            ));
        }
        Some(Value::Null) => None,
        Some(sel) => Some((
            str_of(sel, "source_action", "/body/operation")?,
            str_of(sel, "idempotency_key", "/body/operation")?,
        )),
    };
    let fact = fact_of(engine, service_id)?;
    let operation = match selector {
        None => Value::Null,
        Some((action, key)) => {
            let op = OperationKey {
                principal: engine.principal.clone(),
                action: action.to_owned(),
                version: 1,
                idem_key: key.to_owned(),
            };
            let row = engine
                .store()
                .operation_by_key(&op)
                .map_err(store_fault)?
                .filter(|row| row.subject.as_deref() == Some(service_id))
                .ok_or_else(|| {
                    Fault::new(
                        Code::NotFound,
                        "/body/operation",
                        "no such operation for this service",
                    )
                })?;
            json!({"operation_id": row.operation_id, "action": row.action,
                   "idempotency_key": row.idem_key, "ts": row.ts, "result": row.result})
        }
    };
    Ok((
        false,
        json!({
            "service_id": fact.service_id,
            "owner_id": fact.owner_id,
            "unit_id": fact.unit_id,
            "owner_sha256": fact.owner_sha256.to_string(),
            "generation": fact.generation,
            "cached_health": json_of(&fact.cached_health)?,
            "operation": operation,
        }),
    ))
}

/// The probe body, checked in field order.
fn probe_body(body: &Value) -> Result<(&str, ProbeId), Fault> {
    let service_id = str_of(body, "service_id", "/body/service_id")?;
    let probe_id = str_of(body, "probe_id", "/body/probe_id")?;
    let probe = ProbeId::parse(probe_id)
        .ok_or_else(|| bad("/body/probe_id", "one of active_state, sub_state, main_pid"))?;
    if u64_of(body, "probe_version", "/body/probe_version")? != PROBE_VERSION {
        return Err(Fault::new(
            Code::UnsupportedActionVersion,
            "/body/probe_version",
            "only 1",
        ));
    }
    if u64_of(body, "max_cost_microunits", "/body/max_cost_microunits")? != 0 {
        return Err(bad(
            "/body/max_cost_microunits",
            "the bound is 0: a probe costs nothing",
        ));
    }
    if str_of(body, "network_scope", "/body/network_scope")? != "none" {
        return Err(Fault::new(
            Code::Forbidden,
            "/body/network_scope",
            "only none under RC01",
        ));
    }
    Ok((service_id, probe))
}

/// `service.probe`: one bounded read through the runner, committed as an operation.
fn probe(engine: &Engine, req: &Request, runner: Option<&dyn ServiceRunner>) -> Reply {
    let (service_id, probe) = probe_body(&req.body)?;
    let op = op_key(engine, req);
    let bytes = body_bytes(&req.body)?;
    if let Some(stored) = engine
        .store()
        .service_replay(&op, &bytes)
        .map_err(store_fault)?
    {
        return Ok((true, stored.result));
    }
    let fact = fact_of(engine, service_id)?;
    let input = Sha256Hex::digest(&bytes);
    let observation = runner_of(runner)?
        .probe(&fact.unit_id, probe, &input, &ProbeBudget::DEFAULT)
        .map_err(|e| probe_fault(&e))?;
    let observed = json_of(&observation)?;
    let operation = engine
        .store()
        .service_probe_commit(&op, &bytes, service_id, &observation, |id| {
            json!({"operation_id": id, "service_id": service_id, "observation": observed,
                   "cost_microunits": 0, "external_effect": "none"})
        })
        .map_err(service_fault)?;
    Ok((operation.replayed, operation.result))
}

/// The action body, checked in field order.
fn action_body(body: &Value) -> Result<(&str, &str, UnitAction, Sha256Hex), Fault> {
    let service_id = str_of(body, "service_id", "/body/service_id")?;
    let unit_id = str_of(body, "unit_id", "/body/unit_id")?;
    let act = UnitAction::parse(str_of(body, "action", "/body/action")?)
        .ok_or_else(|| bad("/body/action", "one of start, stop, restart"))?;
    let owner = str_of(body, "expected_owner_sha256", "/body/expected_owner_sha256")?
        .parse::<Sha256Hex>()
        .map_err(|e| bad("/body/expected_owner_sha256", e.to_string()))?;
    Ok((service_id, unit_id, act, owner))
}

/// `service.action`: a CAS-guarded act through the runner, committed only after its read-back
/// settled. An unsettled read-back is `effect_unknown` and writes no operations row.
///
/// The act runs under the service's durable claim (`Store::service_claim`), taken after every
/// request check and before `runner.act`, released after the commit whatever the outcome. The
/// claim is in the ledger, so it holds across serves on one file and across a restart mid-act:
/// a request that finds it held is refused `conflict` ("service claim held") before it acts;
/// one that finds the generation moved is refused `stale_generation`. No in-process lock sits
/// in front of it: one guard, and every test of it exercises the cross-process path.
fn action(engine: &Engine, req: &Request, runner: Option<&dyn ServiceRunner>) -> Reply {
    let (service_id, unit_id, act, owner) = action_body(&req.body)?;
    let op = op_key(engine, req);
    let bytes = body_bytes(&req.body)?;
    if let Some(stored) = engine
        .store()
        .service_replay(&op, &bytes)
        .map_err(store_fault)?
    {
        return Ok((true, stored.result));
    }
    let fact = fact_of(engine, service_id)?;
    if unit_id != fact.unit_id {
        return Err(bad("/body/unit_id", "not this service's unit"));
    }
    if !fact.actable {
        return Err(not_actable());
    }
    let pre = req
        .precondition
        .as_ref()
        .ok_or_else(|| bad("/precondition", "required"))?;
    if pre.resource != "service" || pre.id != service_id {
        return Err(bad(
            "/precondition",
            "resource must be \"service\" and id the body's service_id",
        ));
    }
    if pre.generation != fact.generation {
        return Err(stale(fact.generation));
    }
    if owner != fact.owner_sha256 {
        return Err(owner_conflict());
    }
    let runner = runner_of(runner)?;
    if let Some(stale) = engine
        .store()
        .service_claim(service_id, pre.generation, &op)
        .map_err(service_fault)?
    {
        report_stale("service.action superseded", &stale);
    }
    let expected = Expected {
        generation: pre.generation,
        owner_sha256: owner,
    };
    let reply = act_and_commit(engine, runner, (&op, &bytes), &fact, act, expected);
    if let Err(e) = engine.store().service_release(service_id, &op) {
        eprintln!("service.action claim release failed service_id={service_id}: {e}");
    }
    reply
}

/// The claimed half of `service.action`: `runner.act`, then the CAS commit.
fn act_and_commit(
    engine: &Engine,
    runner: &dyn ServiceRunner,
    (op, bytes): (&OperationKey, &[u8]),
    fact: &ServiceFact,
    act: UnitAction,
    expected: Expected,
) -> Reply {
    let service_id = fact.service_id.as_str();
    let input = Sha256Hex::digest(bytes);
    let outcome = runner
        .act(&fact.unit_id, act, &input, &ProbeBudget::DEFAULT)
        .map_err(|e| match &e {
            ActFault::Probe(p) => probe_fault(p),
            ActFault::UnitAbsent(_) => Fault::new(Code::NotFound, "/body/unit_id", e.to_string()),
            ActFault::ManagerRefused(_) => {
                Fault::new(Code::Unavailable, "/body/unit_id", e.to_string())
                    .with_because("manager refused")
            }
            ActFault::EffectUnknown { settling_read, .. } => {
                Fault::new(Code::EffectUnknown, "/", e.to_string()).with_readback(settling_read)
            }
        })?;
    let health = json_of(&outcome.health)?;
    let operation = engine
        .store()
        .service_action_commit(op, bytes, service_id, expected, &outcome.health, |id| {
            json!({"operation_id": id, "service_id": service_id,
                       "owner_job_id": outcome.owner_job_id,
                       "observed_state": outcome.observed_state, "useful_health": health})
        })
        .map_err(service_fault)?;
    Ok((operation.replayed, operation.result))
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use hee4_contracts::{Evidence, Observation, Outcome, ToolId};
    use hee4_core::{Observations, reconcile};

    use super::*;
    use crate::actions::testing::engine;
    use crate::actions::{dispatch_with, handle};
    use crate::service_runner::{ActionOutcome, PROBE_STDOUT_MAX, SETTLING_READ};
    use crate::wire;

    type R = Result<(), Box<dyn std::error::Error>>;

    struct Stub {
        probe: Result<Observation, ProbeFault>,
        act: Result<ActionOutcome, ActFault>,
        calls: AtomicUsize,
        /// How long `act` takes, so concurrent requests overlap inside it.
        act_ms: u64,
    }

    impl ServiceRunner for Stub {
        fn probe(
            &self,
            _: &str,
            _: ProbeId,
            _: &Sha256Hex,
            _: &ProbeBudget,
        ) -> Result<Observation, ProbeFault> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            self.probe.clone()
        }
        fn act(
            &self,
            _: &str,
            _: UnitAction,
            _: &Sha256Hex,
            _: &ProbeBudget,
        ) -> Result<ActionOutcome, ActFault> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            std::thread::sleep(std::time::Duration::from_millis(self.act_ms));
            self.act.clone()
        }
    }

    fn obs(state: &str) -> Result<Observation, Box<dyn std::error::Error>> {
        Ok(Observation {
            source: "service.probe".parse()?,
            input_sha256: Sha256Hex::digest(b"{}"),
            tool: ToolId {
                name: "busctl".parse()?,
                version: "systemd-261".parse()?,
            },
            head_sha: "a".repeat(40).parse()?,
            outcome: Outcome::Pass,
            evidence: vec![Evidence {
                label: "active_state".parse()?,
                sha256: Sha256Hex::digest(state.as_bytes()),
            }],
            advisory: false,
            elapsed_ms: 2,
            budget_ms: 5000,
        })
    }

    fn stub(probe: Result<Observation, ProbeFault>, act: Result<ActionOutcome, ActFault>) -> Stub {
        Stub {
            probe,
            act,
            calls: AtomicUsize::new(0),
            act_ms: 0,
        }
    }

    fn ok_stub() -> Result<Stub, Box<dyn std::error::Error>> {
        Ok(stub(
            Ok(obs("active")?),
            Ok(ActionOutcome {
                owner_job_id: "/org/freedesktop/systemd1/job/7".into(),
                observed_state: "inactive".into(),
                health: obs("inactive")?,
            }),
        ))
    }

    fn ready(name: &str) -> Result<Engine, Box<dyn std::error::Error>> {
        let e = engine(name)?;
        reconcile(&e.store(), &Observations::worker_absent())?;
        e.store().service_seed(SEEDS)?;
        Ok(e)
    }

    fn req(
        action: &str,
        key: Option<&str>,
        body: Value,
        pre: Option<Value>,
    ) -> Result<Request, Box<dyn std::error::Error>> {
        let line = wire::request_with("r", action, key, body, pre).to_string();
        wire::parse(&line).map_err(|(_, f)| f.message.into())
    }

    /// The error frame a reply would carry.
    fn err(reply: Reply) -> Value {
        match reply {
            Ok(v) => json!({"unexpected": v.1}),
            Err(f) => wire::error("r", &f),
        }
    }

    fn probe_body(over: Value) -> Value {
        let mut b = json!({"service_id": "drive", "probe_id": "active_state", "probe_version": 1,
                           "max_cost_microunits": 0, "network_scope": "none"});
        if let (Some(b), Value::Object(o)) = (b.as_object_mut(), over) {
            b.extend(o);
        }
        b
    }

    fn action_body(over: Value) -> Value {
        let mut b = json!({"service_id": "drive", "unit_id": "hee4-drive.service", "action": "stop",
                           "expected_owner_sha256": Sha256Hex::digest(b"deploy").to_string()});
        if let (Some(b), Value::Object(o)) = (b.as_object_mut(), over) {
            b.extend(o);
        }
        b
    }

    fn pre(generation: u64) -> Value {
        json!({"resource": "service", "id": "drive", "generation": generation})
    }

    fn inspect_drive(e: &Engine, operation: &Value) -> Result<Value, Box<dyn std::error::Error>> {
        let r = req(
            "service.inspect",
            None,
            json!({"service_id": "drive", "operation": operation}),
            None,
        )?;
        Ok(inspect(e, &r).map_err(|f| f.message)?.1)
    }

    #[test]
    fn service_inspect_reads_the_seeded_facts() -> R {
        let e = ready("svc-inspect")?;
        let b = inspect_drive(&e, &Value::Null)?;
        assert_eq!(b["unit_id"], "hee4-drive.service");
        assert_eq!(b["generation"], 1);
        assert_eq!(b["owner_sha256"], Sha256Hex::digest(b"deploy").to_string());
        assert_eq!(b["cached_health"], Value::Null);
        assert_eq!(b["operation"], Value::Null);
        let none = req(
            "service.inspect",
            None,
            json!({"service_id": "nope", "operation": null}),
            None,
        )?;
        let f = err(inspect(&e, &none));
        assert_eq!(
            (f["code"].as_str(), f["field"].as_str()),
            (Some("not_found"), Some("/body/service_id"))
        );
        let missing = req(
            "service.inspect",
            None,
            json!({"service_id": "drive"}),
            None,
        )?;
        assert_eq!(err(inspect(&e, &missing))["field"], "/body/operation");
        let unknown_op = req(
            "service.inspect",
            None,
            json!({"service_id": "drive", "operation": {"source_action": "service.probe", "idempotency_key": "x"}}),
            None,
        )?;
        let f = err(inspect(&e, &unknown_op));
        assert_eq!(
            (f["code"].as_str(), f["field"].as_str()),
            (Some("not_found"), Some("/body/operation"))
        );
        Ok(())
    }

    #[test]
    fn service_probe_commits_replays_and_reads_back() -> R {
        let e = ready("svc-probe")?;
        let s = ok_stub()?;
        let r = req("service.probe", Some("p1"), probe_body(json!({})), None)?;
        let (replayed, b) = probe(&e, &r, Some(&s)).map_err(|f| f.message)?;
        assert!(!replayed);
        assert_eq!(b["external_effect"], "none");
        assert_eq!(b["cost_microunits"], 0);
        assert_eq!(b["observation"]["evidence"][0]["label"], "active_state");
        let (again, b2) = probe(&e, &r, Some(&s)).map_err(|f| f.message)?;
        assert!(again);
        assert_eq!(b2["operation_id"], b["operation_id"]);
        assert_eq!(
            s.calls.load(Ordering::SeqCst),
            1,
            "a replay never re-probes"
        );
        let read = inspect_drive(
            &e,
            &json!({"source_action": "service.probe", "idempotency_key": "p1"}),
        )?;
        assert_eq!(read["cached_health"], b["observation"]);
        assert_eq!(read["operation"]["operation_id"], b["operation_id"]);
        Ok(())
    }

    #[test]
    fn service_probe_refusals_by_code_and_field() -> R {
        let e = ready("svc-probe-refuse")?;
        let s = ok_stub()?;
        let cases = [
            (
                json!({"probe_version": 2}),
                "unsupported_action_version",
                "/body/probe_version",
            ),
            (
                json!({"network_scope": "loopback"}),
                "forbidden",
                "/body/network_scope",
            ),
            (
                json!({"probe_id": "cpu"}),
                "invalid_argument",
                "/body/probe_id",
            ),
            (
                json!({"max_cost_microunits": 5}),
                "invalid_argument",
                "/body/max_cost_microunits",
            ),
            (
                json!({"service_id": "nope"}),
                "not_found",
                "/body/service_id",
            ),
        ];
        for (i, (over, code, field)) in cases.into_iter().enumerate() {
            let r = req(
                "service.probe",
                Some(&format!("k{i}")),
                probe_body(over),
                None,
            )?;
            let f = err(probe(&e, &r, Some(&s)));
            assert_eq!(
                (f["code"].as_str(), f["field"].as_str()),
                (Some(code), Some(field)),
                "{f}"
            );
        }
        assert_eq!(s.calls.load(Ordering::SeqCst), 0);
        let digest = stub(
            Err(ProbeFault::Digest {
                expected: "0".repeat(64),
                found: "a".repeat(64),
            }),
            Err(ActFault::Probe(ProbeFault::HeadUnknown)),
        );
        let f = err(probe(
            &e,
            &req("service.probe", Some("d"), probe_body(json!({})), None)?,
            Some(&digest),
        ));
        assert_eq!(
            (f["code"].as_str(), f["because"].as_str()),
            (Some("unavailable"), Some("busctl digest"))
        );
        let big = stub(
            Err(ProbeFault::StdoutOverBound {
                bytes: 5000,
                bound: PROBE_STDOUT_MAX,
            }),
            Err(ActFault::Probe(ProbeFault::HeadUnknown)),
        );
        let f = err(probe(
            &e,
            &req("service.probe", Some("b"), probe_body(json!({})), None)?,
            Some(&big),
        ));
        assert_eq!(f["code"], "resource_exhausted");
        let msg = f["message"].as_str().unwrap_or_default();
        assert!(msg.contains("5000") && msg.contains("4096"), "{msg}");
        Ok(())
    }

    #[test]
    fn service_action_cas_readback_and_replay() -> R {
        let e = ready("svc-action")?;
        let s = ok_stub()?;
        let r = req(
            "service.action",
            Some("a1"),
            action_body(json!({})),
            Some(pre(1)),
        )?;
        let (replayed, b) = action(&e, &r, Some(&s)).map_err(|f| f.message)?;
        assert!(!replayed);
        assert_eq!(b["observed_state"], "inactive");
        assert_eq!(b["owner_job_id"], "/org/freedesktop/systemd1/job/7");
        assert_eq!(inspect_drive(&e, &Value::Null)?["generation"], 2);
        let (again, b2) = action(&e, &r, Some(&s)).map_err(|f| f.message)?;
        assert!(again);
        assert_eq!(b2, b);
        assert_eq!(s.calls.load(Ordering::SeqCst), 1);
        let stale = err(action(
            &e,
            &req(
                "service.action",
                Some("a2"),
                action_body(json!({})),
                Some(pre(1)),
            )?,
            Some(&s),
        ));
        assert_eq!(
            (stale["code"].as_str(), stale["field"].as_str()),
            (Some("stale_generation"), Some("/precondition/generation"))
        );
        assert_eq!(stale["current_generation"], 2);
        Ok(())
    }

    /// A second serve's engine over `e`'s ledger file: its own `Store` connection, as another
    /// `hee4 serve` process on the same ledger has.
    fn second_serve(e: &Engine) -> Result<Engine, Box<dyn std::error::Error>> {
        let ledger = e.store().path().to_path_buf();
        let dir = ledger.parent().ok_or("ledger has no directory")?;
        Ok(Engine::new(
            hee4_core::Store::open(&ledger)?,
            ledger.clone(),
            dir.join("work-2"),
            dir.join("rt-2"),
            crate::actions::Config {
                model: "m:1".into(),
                live: false,
            },
        )?)
    }

    /// Whether `reply` is the loser's refusal by name: the claim was held, or the generation
    /// had moved by the time it looked.
    fn refused_by_name(reply: &Value) -> bool {
        (reply["code"] == "conflict" && reply["because"] == "service claim held")
            || (reply["code"] == "stale_generation" && reply["current_generation"] == 2)
    }

    /// Two concurrent requests on one service at one generation, run on two threads against
    /// a slow stub: `keys` names each request's idempotency key; with `two_serves` the second
    /// request goes through a second engine over the same ledger file. Returns the replies, as
    /// error frames or `{"replayed": .., "body": ..}`, and how many times the stub acted.
    fn race(
        name: &str,
        keys: [&str; 2],
        two_serves: bool,
    ) -> Result<(Vec<Value>, usize), Box<dyn std::error::Error>> {
        let e = ready(name)?;
        let other = if two_serves {
            Some(second_serve(&e)?)
        } else {
            None
        };
        let serves = [&e, other.as_ref().unwrap_or(&e)];
        let mut s = ok_stub()?;
        s.act_ms = 150;
        let reqs = keys
            .iter()
            .map(|k| {
                req(
                    "service.action",
                    Some(k),
                    action_body(json!({})),
                    Some(pre(1)),
                )
            })
            .collect::<Result<Vec<_>, _>>()?;
        let replies = std::thread::scope(|scope| {
            let handles: Vec<_> = reqs
                .iter()
                .zip(serves)
                .map(|(r, e)| {
                    let s = &s;
                    scope.spawn(move || match action(e, r, Some(s)) {
                        Ok((replayed, body)) => json!({"replayed": replayed, "body": body}),
                        Err(f) => wire::error("r", &f),
                    })
                })
                .collect();
            handles
                .into_iter()
                .map(|h| h.join().unwrap_or_else(|_| json!({"panicked": true})))
                .collect::<Vec<_>>()
        });
        assert_eq!(
            e.store().service_claim_get("drive")?,
            None,
            "every claim is released after the race: {replies:?}"
        );
        Ok((replies, s.calls.load(Ordering::SeqCst)))
    }

    #[test]
    fn service_action_concurrent_keys_act_once_and_the_loser_is_refused_by_name() -> R {
        let (replies, calls) = race("svc-race-keys", ["c1", "c2"], false)?;
        assert_eq!(
            calls, 1,
            "only one request reaches the manager: {replies:?}"
        );
        let won = replies.iter().filter(|r| r["replayed"] == false).count();
        let refused = replies.iter().filter(|r| refused_by_name(r)).count();
        assert_eq!((won, refused), (1, 1), "{replies:?}");
        Ok(())
    }

    #[test]
    fn service_action_concurrent_same_key_acts_once() -> R {
        let (replies, calls) = race("svc-race-same", ["c1", "c1"], false)?;
        assert_eq!(calls, 1, "a same-key race acts once: {replies:?}");
        let won = replies
            .iter()
            .find(|r| r["replayed"] == false)
            .ok_or("no request acted")?;
        let loser = replies
            .iter()
            .find(|r| r["replayed"] != false)
            .ok_or("no loser")?;
        let replayed = loser["replayed"] == true && loser["body"] == won["body"];
        let held = loser["code"] == "conflict" && loser["because"] == "service claim held";
        assert!(replayed || held, "{replies:?}");
        Ok(())
    }

    #[test]
    fn two_serves_on_one_ledger_act_once_and_the_loser_is_refused_by_name() -> R {
        let (replies, calls) = race("svc-race-serves", ["c1", "c2"], true)?;
        assert_eq!(
            calls, 1,
            "two serves on one ledger reach the manager once: {replies:?}"
        );
        let won = replies.iter().filter(|r| r["replayed"] == false).count();
        let refused = replies.iter().filter(|r| refused_by_name(r)).count();
        assert_eq!((won, refused), (1, 1), "{replies:?}");
        Ok(())
    }

    #[test]
    fn a_claim_left_by_a_serve_that_died_mid_act_refuses_before_acting() -> R {
        let e = ready("svc-died")?;
        let s = ok_stub()?;
        let dead = OperationKey {
            principal: e.principal.clone(),
            action: "service.action".into(),
            version: 1,
            idem_key: "died".into(),
        };
        e.store().service_claim("drive", 1, &dead)?;
        let restarted = second_serve(&e)?;
        let r = req(
            "service.action",
            Some("after"),
            action_body(json!({})),
            Some(pre(1)),
        )?;
        let f = err(action(&restarted, &r, Some(&s)));
        assert_eq!(
            (
                f["code"].as_str(),
                f["field"].as_str(),
                f["because"].as_str()
            ),
            (
                Some("conflict"),
                Some("/body/service_id"),
                Some("service claim held")
            ),
            "{f}"
        );
        assert_eq!(s.calls.load(Ordering::SeqCst), 0, "the loser never acts");
        assert_eq!(
            restarted
                .store()
                .operation_by_key(&op_key(&restarted, &r))?,
            None
        );
        assert!(
            restarted.store().service_claim_get("drive")?.is_some(),
            "a refused request leaves the holder's claim in place"
        );
        Ok(())
    }

    #[test]
    fn service_action_on_self_or_model_is_forbidden_server_side() -> R {
        let e = ready("svc-held")?;
        let s = ok_stub()?;
        for (id, unit) in [("self", "hee4.service"), ("model", "ollama.service")] {
            let r = req(
                "service.action",
                Some(&format!("held-{id}")),
                action_body(json!({"service_id": id, "unit_id": unit})),
                Some(json!({"resource": "service", "id": id, "generation": 1})),
            )?;
            let f = err(action(&e, &r, Some(&s)));
            assert_eq!(
                (
                    f["code"].as_str(),
                    f["field"].as_str(),
                    f["because"].as_str()
                ),
                (
                    Some("forbidden"),
                    Some("/body/service_id"),
                    Some("service not actable")
                ),
                "{f}"
            );
        }
        assert_eq!(
            s.calls.load(Ordering::SeqCst),
            0,
            "a held service never reaches the manager"
        );
        Ok(())
    }

    #[test]
    fn service_action_refusals_by_code_and_field() -> R {
        let e = ready("svc-action-refuse")?;
        let s = ok_stub()?;
        let wrong = json!({"expected_owner_sha256": Sha256Hex::digest(b"other").to_string()});
        let other_pre = Some(json!({"resource": "task", "id": "drive", "generation": 1}));
        let cases = [
            (
                json!({"service_id": "nope"}),
                Some(pre(1)),
                "not_found",
                "/body/service_id",
            ),
            (
                json!({"unit_id": "ollama.service"}),
                Some(pre(1)),
                "invalid_argument",
                "/body/unit_id",
            ),
            (json!({}), other_pre, "invalid_argument", "/precondition"),
            (
                json!({}),
                Some(pre(3)),
                "stale_generation",
                "/precondition/generation",
            ),
            (
                wrong,
                Some(pre(1)),
                "conflict",
                "/body/expected_owner_sha256",
            ),
            (
                json!({"action": "reload"}),
                Some(pre(1)),
                "invalid_argument",
                "/body/action",
            ),
        ];
        for (i, (over, p, code, field)) in cases.into_iter().enumerate() {
            let r = req(
                "service.action",
                Some(&format!("k{i}")),
                action_body(over),
                p,
            )?;
            let f = err(action(&e, &r, Some(&s)));
            assert_eq!(
                (f["code"].as_str(), f["field"].as_str()),
                (Some(code), Some(field)),
                "{f}"
            );
        }
        assert_eq!(s.calls.load(Ordering::SeqCst), 0);
        let no_pre =
            wire::request("r", "service.action", Some("np"), action_body(json!({}))).to_string();
        let parsed = wire::parse(&no_pre).map_err(|(_, f)| f.message)?;
        let f = match dispatch_with(&e, e.registry(), &parsed) {
            Ok(_) => json!({}),
            Err(f) => wire::error("r", &f),
        };
        assert_eq!(
            (f["code"].as_str(), f["field"].as_str()),
            (Some("invalid_argument"), Some("/precondition"))
        );
        let digest = stub(
            Ok(obs("active")?),
            Err(ActFault::Probe(ProbeFault::Digest {
                expected: "x".into(),
                found: "y".into(),
            })),
        );
        let f = err(action(
            &e,
            &req(
                "service.action",
                Some("dg"),
                action_body(json!({})),
                Some(pre(1)),
            )?,
            Some(&digest),
        ));
        assert_eq!(
            (f["code"].as_str(), f["because"].as_str()),
            (Some("unavailable"), Some("busctl digest"))
        );
        Ok(())
    }

    #[test]
    fn service_action_manager_refusal_is_typed_and_writes_no_operation() -> R {
        let e = ready("svc-refused")?;
        for (i, (fault, code, because)) in [
            (
                ActFault::UnitAbsent("Unit hee4-drive.service not loaded.".into()),
                "not_found",
                None,
            ),
            (
                ActFault::ManagerRefused("Unit hee4-drive.service is masked.".into()),
                "unavailable",
                Some("manager refused"),
            ),
        ]
        .into_iter()
        .enumerate()
        {
            let s = stub(Ok(obs("active")?), Err(fault));
            let r = req(
                "service.action",
                Some(&format!("m{i}")),
                action_body(json!({})),
                Some(pre(1)),
            )?;
            let f = err(action(&e, &r, Some(&s)));
            assert_eq!(
                (
                    f["code"].as_str(),
                    f["field"].as_str(),
                    f["because"].as_str()
                ),
                (Some(code), Some("/body/unit_id"), because),
                "{f}"
            );
            assert_eq!(e.store().operation_by_key(&op_key(&e, &r))?, None);
        }
        assert_eq!(inspect_drive(&e, &Value::Null)?["generation"], 1);
        Ok(())
    }

    #[test]
    fn service_action_effect_unknown_writes_no_operation() -> R {
        let e = ready("svc-unknown")?;
        let s = stub(
            Ok(obs("active")?),
            Err(ActFault::EffectUnknown {
                settling_read: SETTLING_READ,
                detail: "ActiveState read deactivating".into(),
            }),
        );
        let r = req(
            "service.action",
            Some("u1"),
            action_body(json!({})),
            Some(pre(1)),
        )?;
        let f = err(action(&e, &r, Some(&s)));
        assert_eq!(f["code"], "effect_unknown");
        assert_eq!(f["retry"], "after_readback");
        assert_eq!(f["effect"], "unknown");
        assert_eq!(f["readback"], "service.inspect");
        assert_eq!(e.store().operation_by_key(&op_key(&e, &r))?, None);
        assert_eq!(inspect_drive(&e, &Value::Null)?["generation"], 1);
        Ok(())
    }

    #[test]
    fn service_without_a_started_runner_is_unavailable_by_name() -> R {
        let e = ready("svc-unstarted")?;
        if RUNNER.get().is_some() {
            println!("UNMEASURED: the runner is set in this process; unstarted path skipped");
            return Ok(());
        }
        let line =
            wire::request("r", "service.probe", Some("x"), probe_body(json!({}))).to_string();
        let f = handle(&e, &line);
        assert_eq!(
            (f["code"].as_str(), f["because"].as_str()),
            (Some("unavailable"), Some("service runner not started"))
        );
        Ok(())
    }
}
