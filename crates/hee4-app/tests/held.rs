//! `judge.inspect` is held by construction. There is no handler module for the id anywhere in
//! `hee4-app`: it is catalogued under `Owner::Judge`, `Registry::new` refuses a `Judge` family
//! (`RegistryFault::HeldOwner`), so `composed()` has no entry for the owner and dispatch stops at
//! the registry miss, the one `unavailable` site. Egress is impossible because no code path from
//! the id reaches a handler, not because a grep says so; this test measures the refusal and that
//! the ledger is untouched.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::PathBuf;

use hee4_app::actions::{Engine, composed, handle};
use hee4_app::dispatcher::Config;
use hee4_app::wire;
use hee4_contracts::catalogue::{Owner, Scope, find};
use hee4_core::{Observations, Store, reconcile};
use serde_json::json;

type R<T> = Result<T, Box<dyn std::error::Error>>;

#[test]
fn judge_inspect_is_unavailable_by_name_and_touches_nothing() -> R<()> {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("held");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir)?;
    let store = Store::open(&dir.join("ledger.sqlite3"))?;
    reconcile(&store, &Observations::worker_absent())?;
    let e = Engine::new(
        store,
        dir.join("ledger.sqlite3"),
        dir.join("work"),
        dir.join("rt"),
        Config {
            model: "m:1".into(),
            live: false,
        },
    )?;
    let held = find("judge.inspect").expect("catalogued");
    assert_eq!(held.owner, Owner::Judge);
    assert_eq!(held.scope, Scope::Held);
    assert!(composed()?.get(Owner::Judge).is_none());

    let reply = handle(
        &e,
        &wire::request("r", "judge.inspect", None, json!({})).to_string(),
    );
    assert_eq!(reply["kind"], "error", "{reply}");
    assert_eq!(reply["code"], "unavailable");
    assert_eq!(reply["retry"], "after_condition");
    assert_eq!(reply["field"], "/action");
    assert_eq!(reply["because"], Scope::Held.because());
    let because = reply["because"].as_str().unwrap_or_default();
    assert!(because.contains("H-8"), "{because}");
    assert_ne!(because, Scope::V41.because());
    assert_ne!(because, Scope::V42.because());
    assert_ne!(because, Scope::V40.because());

    assert_eq!(e.store().task_ids()?.len(), 0);
    assert_eq!(e.store().event_count()?, 0);
    Ok(())
}
