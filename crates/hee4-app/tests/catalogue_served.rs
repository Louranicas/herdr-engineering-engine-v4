//! Every catalogued id is either served by a registered family or refused `unavailable` with
//! its scope's `because`; never `unknown_action`. `Registry::new` refuses a held owner, an
//! unknown id and an owner mismatch by typed name; a `Required(resource)` action without a
//! `precondition` is `invalid_argument` at `/precondition`.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::PathBuf;

use hee4_app::actions::{
    Answer, Engine, Family, Handler, Registry, RegistryFault, composed, dispatch_with, handle,
};
use hee4_app::dispatcher::Config;
use hee4_app::wire::{self, Code, Fault, Request};
use hee4_contracts::catalogue::{CATALOGUE, Owner, PreconditionRule, find};
use hee4_core::{Observations, Store, reconcile};
use serde_json::{Value, json};

type R<T> = Result<T, Box<dyn std::error::Error>>;

fn engine(name: &str) -> R<Engine> {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("served-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir)?;
    let store = Store::open(&dir.join("ledger.sqlite3"))?;
    let report = reconcile(&store, &Observations::worker_absent())?;
    assert!(report.complete);
    Ok(Engine::new(
        store,
        dir.join("ledger.sqlite3"),
        dir.join("work"),
        dir.join("rt"),
        Config {
            model: "m:1".into(),
            live: false,
        },
    )?)
}

#[expect(clippy::unnecessary_wraps, reason = "the signature is `Handler`'s")]
fn stub(_: &Engine, _: &Request) -> Result<Answer, Fault> {
    Ok(Answer::Frame(false, json!({})))
}

/// A test-only family that stubs every catalogued id of `owner` (read from `CATALOGUE`, never a
/// second id list); the handler table is leaked once because `Family` holds `&'static`.
fn stub_family(owner: Owner) -> Family {
    let handlers: Vec<(&'static str, Handler)> = CATALOGUE
        .iter()
        .filter(|e| e.owner == owner)
        .map(|e| (e.id, stub as Handler))
        .collect();
    Family {
        owner,
        handlers: Box::leak(handlers.into_boxed_slice()),
        on_serve_start: None,
    }
}

#[test]
fn every_catalogued_id_is_served_or_unavailable_by_scope() -> R<()> {
    let e = engine("all")?;
    let registry = composed()?;
    let mut unavailable = 0;
    for entry in &CATALOGUE {
        let line = wire::request("r", entry.id, Some("k"), json!({})).to_string();
        let reply = handle(&e, &line);
        assert_ne!(reply["code"], "unknown_action", "{}: {reply}", entry.id);
        if registry.get(entry.owner).is_none() {
            unavailable += 1;
            assert_eq!(reply["kind"], "error", "{}: {reply}", entry.id);
            assert_eq!(reply["code"], "unavailable", "{}: {reply}", entry.id);
            assert_eq!(reply["retry"], "after_condition", "{}", entry.id);
            assert_eq!(reply["field"], "/action", "{}", entry.id);
            assert_eq!(reply["because"], entry.scope.because(), "{}", entry.id);
        } else {
            assert_ne!(reply["code"], "unavailable", "{}: {reply}", entry.id);
            assert!(registry.handler(entry.id).is_some(), "{}", entry.id);
        }
    }
    let unregistered = CATALOGUE
        .iter()
        .filter(|a| registry.get(a.owner).is_none())
        .count();
    assert_eq!(unavailable, unregistered);
    assert!(unavailable > 0, "nothing in the catalogue is unserved");
    assert!(composed()?.get(Owner::Judge).is_none());
    assert!(composed()?.get(Owner::Deploy).is_none());
    Ok(())
}

#[test]
fn registry_refuses_held_owners_unknown_ids_and_mismatches_by_name() {
    for owner in [Owner::Judge, Owner::Deploy] {
        let held = Registry::new(&[Family {
            owner,
            handlers: &[],
            on_serve_start: None,
        }]);
        assert!(
            matches!(held, Err(RegistryFault::HeldOwner(o)) if o == owner),
            "{owner}: {held:?}"
        );
    }
    let unknown = Registry::new(&[Family {
        owner: Owner::Task,
        handlers: &[("task.explode", stub)],
        on_serve_start: None,
    }]);
    assert!(
        matches!(&unknown, Err(RegistryFault::UnknownId(id)) if id == "task.explode"),
        "{unknown:?}"
    );
    let mismatch = Registry::new(&[Family {
        owner: Owner::Roster,
        handlers: &[("tools.list", stub)],
        on_serve_start: None,
    }]);
    assert!(
        matches!(
            &mismatch,
            Err(RegistryFault::OwnerMismatch { id, family: Owner::Roster, catalogue: Owner::Actions })
                if id == "tools.list"
        ),
        "{mismatch:?}"
    );
    let twice = Registry::new(&[stub_family(Owner::App), stub_family(Owner::App)]);
    assert!(
        matches!(twice, Err(RegistryFault::DuplicateOwner(Owner::App))),
        "{twice:?}"
    );
}

/// A family that registers an owner with only some of that owner's catalogued ids is refused at
/// compose time, so a registered owner's id can never reach dispatch without a handler.
#[test]
fn registry_refuses_a_family_that_leaves_one_of_its_owners_ids_unserved() {
    let served = "task.get";
    assert_eq!(find(served).map(|e| e.owner), Some(Owner::Task));
    let first_unserved = CATALOGUE
        .iter()
        .find(|e| e.owner == Owner::Task && e.id != served)
        .map(|e| e.id)
        .expect("Task owns more than one id");
    let partial = Registry::new(&[Family {
        owner: Owner::Task,
        handlers: &[("task.get", stub)],
        on_serve_start: None,
    }]);
    assert!(
        matches!(
            &partial,
            Err(RegistryFault::MissingHandler { owner: Owner::Task, id }) if id == first_unserved
        ),
        "{partial:?}"
    );
    let full = composed().expect("composed");
    for entry in CATALOGUE.iter().filter(|e| full.get(e.owner).is_some()) {
        assert!(full.serve(entry).is_some(), "{}", entry.id);
    }
}

#[test]
fn a_required_precondition_action_without_one_is_refused_at_precondition() -> R<()> {
    let e = engine("precondition")?;
    let required = find("roster.disable").expect("catalogued");
    assert_eq!(required.owner, Owner::Roster);
    let PreconditionRule::Required(resource) = required.precondition else {
        panic!("{} is not Required", required.id);
    };
    let registry = Registry::new(&[stub_family(Owner::Roster)])?;
    let line = wire::request("r", required.id, Some("k"), json!({})).to_string();
    let req = wire::parse(&line).map_err(|(_, f)| f.message)?;
    let refused = dispatch_with(&e, &registry, &req);
    let fault = refused.expect_err("refused");
    assert_eq!(fault.code, Code::InvalidArgument);
    assert_eq!(fault.field, "/precondition");
    assert!(fault.message.contains(resource), "{}", fault.message);
    assert!(fault.message.contains("roster"), "{}", fault.message);

    let with = wire::request_with(
        "r",
        required.id,
        Some("k"),
        json!({}),
        Some(json!({"resource": resource, "id": "x", "generation": 1})),
    )
    .to_string();
    let req = wire::parse(&with).map_err(|(_, f)| f.message)?;
    assert_eq!(
        dispatch_with(&e, &registry, &req).ok(),
        Some(Answer::Frame(false, json!({})))
    );

    let mut malformed: Value = wire::request("r", required.id, Some("k"), json!({}));
    malformed["precondition"] = json!(5);
    let reply = handle(&e, &malformed.to_string());
    assert_eq!(reply["code"], "invalid_argument");
    assert_eq!(reply["field"], "/precondition");
    Ok(())
}
