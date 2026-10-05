//! Behaviour of the ledger doors: `open`, `apply`, `admit`, `record_observation`,
//! `append_receipt`, `reconcile`.

use std::error::Error;
use std::path::PathBuf;

use hee4_contracts::{
    Decision, Event, GitSha, Observation, ObservationId, Outcome, Phase, Receipt, ReceiptBody,
    RecoveryRule, Refusal, Settlement, Sha256Hex, TaskId, ToolId, Verdict,
};
use hee4_core::{
    AttemptOutcome, AttemptState, Cleanup, Observations, OperationKey, Store, StoreError, reconcile,
};
use serde_json::json;

type R = Result<(), Box<dyn Error>>;

fn db(name: &str) -> Result<PathBuf, Box<dyn Error>> {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("hee4-core-store");
    std::fs::create_dir_all(&dir)?;
    let path = dir.join(format!("{name}.sqlite"));
    for suffix in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{}{suffix}", path.display()));
    }
    Ok(path)
}

/// A fresh store whose (empty) reconcile has completed, so `Dispatch` is open.
fn ready(name: &str) -> Result<Store, Box<dyn Error>> {
    let store = Store::open(&db(name)?)?;
    assert!(reconcile(&store, &Observations::default())?.complete);
    Ok(store)
}

fn tid(s: &str) -> Result<TaskId, Box<dyn Error>> {
    Ok(s.parse()?)
}

fn op(key: &str) -> OperationKey {
    OperationKey {
        principal: "luke".into(),
        action: "task.submit".into(),
        version: 1,
        idem_key: key.into(),
    }
}

fn observation() -> Result<Observation, Box<dyn Error>> {
    Ok(Observation {
        source: "cargo-test".parse()?,
        input_sha256: Sha256Hex::digest(b"input"),
        tool: ToolId {
            name: "cargo".parse()?,
            version: "1.99".parse()?,
        },
        head_sha: "a".repeat(40).parse::<GitSha>()?,
        outcome: Outcome::Pass,
        evidence: vec![],
        advisory: false,
        elapsed_ms: 1,
        budget_ms: 10,
    })
}

#[test]
fn open_sets_wal_full_fk_and_schema() -> R {
    let path = db("open")?;
    let store = Store::open(&path)?;
    store.apply(&"t1".parse()?, Event::Admit)?;
    assert_eq!(store.pragmas()?, ("wal".into(), 2, 1));
    assert_eq!(store.integrity_check()?, "ok");
    assert!(!store.recovery_complete()?, "open resets the gate");
    drop(store);
    let again = Store::open(&path)?;
    assert_eq!(again.event_count()?, 1, "reopen keeps the ledger");
    assert_eq!(again.phase(&"t1".parse()?)?, Some(Phase::Admitted));
    Ok(())
}

#[test]
fn apply_writes_event_and_cache_together() -> R {
    let store = ready("apply")?;
    let t = tid("t1")?;
    assert_eq!(store.apply(&t, Event::Admit)?, Phase::Admitted);
    assert_eq!(store.apply(&t, Event::Dispatch)?, Phase::Running);
    assert_eq!(store.event_count()?, 2);
    let row = store.cached(&t)?.ok_or("no row")?;
    assert_eq!(
        (row.phase.as_str(), row.cancel, row.generation),
        ("running", false, 1)
    );
    assert_eq!(store.phase(&t)?, Some(Phase::Running));
    let attempts = store.attempts(&t)?;
    assert_eq!(attempts.len(), 1, "one attempts row per Dispatch");
    assert_eq!(attempts[0].id.to_string(), "a-t1-1");
    assert_eq!(
        (attempts[0].state, attempts[0].dispatch_seq),
        (AttemptState::Running, 2)
    );
    Ok(())
}

#[test]
fn refused_edge_writes_nothing() -> R {
    let store = ready("refused")?;
    let t = tid("t1")?;
    store.apply(&t, Event::Admit)?;
    let before = store.event_count()?;
    let err = store.apply(&t, Event::Accept);
    assert!(
        matches!(err, Err(StoreError::Refused(Refusal::Illegal { .. }))),
        "{err:?}"
    );
    let err = store.apply(&tid("ghost")?, Event::Dispatch);
    assert!(
        matches!(err, Err(StoreError::Refused(Refusal::NotAdmitted { .. }))),
        "{err:?}"
    );
    assert_eq!(store.event_count()?, before);
    assert_eq!(store.cached(&tid("ghost")?)?, None);
    assert_eq!(store.cached(&t)?.ok_or("row")?.phase, "admitted");
    Ok(())
}

#[test]
fn dispatch_is_refused_until_reconcile_completes() -> R {
    let path = db("gate")?;
    let store = Store::open(&path)?;
    let t = tid("t1")?;
    store.apply(&t, Event::Admit)?;
    assert!(matches!(
        store.apply(&t, Event::Dispatch),
        Err(StoreError::RecoveryIncomplete)
    ));
    assert_eq!(store.event_count()?, 1);
    assert!(reconcile(&store, &Observations::default())?.complete);
    assert!(store.recovery_complete()?);
    assert_eq!(store.apply(&t, Event::Dispatch)?, Phase::Running);
    Ok(())
}

#[test]
fn admit_replays_same_bytes_and_conflicts_on_other_bytes() -> R {
    let store = ready("admit")?;
    let t = tid("t1")?;
    let first = store.admit(
        &t,
        &op("k1"),
        b"spec-A",
        |id, p| json!({"id": id.as_str(), "state": p.as_str()}),
    )?;
    assert!(!first.replayed);
    assert_eq!(first.result, json!({"id": "t1", "state": "admitted"}));
    let replay = store.admit(&tid("t2")?, &op("k1"), b"spec-A", |_, _| json!("never"))?;
    assert_eq!(
        (replay.replayed, replay.task_id.as_str(), &replay.result),
        (true, "t1", &first.result)
    );
    let conflict = store.admit(&tid("t3")?, &op("k1"), b"spec-B", |_, _| json!("never"));
    assert!(
        matches!(conflict, Err(StoreError::Conflict(_))),
        "{conflict:?}"
    );
    assert_eq!(store.event_count()?, 1, "one Admit event, no second task");
    assert_eq!(store.task_ids()?.len(), 1);
    Ok(())
}

#[test]
fn receipt_chain_and_observations_are_checked_on_append() -> R {
    let store = ready("receipt")?;
    let t = tid("t1")?;
    store.apply(&t, Event::Admit)?;
    let o1: ObservationId = "o1".parse()?;
    store.record_observation(&t, &o1, &observation()?)?;
    store.record_observation(&t, &o1, &observation()?)?; // idempotent
    let mut other = observation()?;
    other.outcome = Outcome::Fail;
    assert!(matches!(
        store.record_observation(&t, &o1, &other),
        Err(StoreError::ObservationConflict(_))
    ));

    let body = |id: &str, obs: Vec<ObservationId>| -> Result<ReceiptBody, Box<dyn Error>> {
        Ok(ReceiptBody {
            id: id.parse()?,
            task_id: t.clone(),
            decision: Decision {
                verdict: Verdict::Pass,
            },
            observed: obs,
        })
    };
    let r1 = Receipt::seal(Sha256Hex::GENESIS, body("r1", vec![o1.clone()])?);
    store.append_receipt(&r1)?;
    assert_eq!(store.chain_head(&t)?, r1.hash_self());
    let fork = Receipt::seal(Sha256Hex::GENESIS, body("r2", vec![])?);
    assert!(matches!(
        store.append_receipt(&fork),
        Err(StoreError::Chain(_))
    ));
    let unledgered = Receipt::seal(r1.hash_self(), body("r3", vec!["o9".parse()?])?);
    assert!(matches!(
        store.append_receipt(&unledgered),
        Err(StoreError::UnledgeredObservation(_))
    ));
    let r2 = Receipt::seal(r1.hash_self(), body("r2", vec![o1])?);
    store.append_receipt(&r2)?;
    assert_eq!(store.chain_head(&t)?, r2.hash_self());
    Ok(())
}

#[test]
fn reconcile_applies_r08_through_apply_and_is_idempotent() -> R {
    let store = ready("reconcile")?;
    for (name, events) in [
        ("a-running", vec![Event::Admit, Event::Dispatch]),
        ("b-admitted", vec![Event::Admit]),
        (
            "c-verifying",
            vec![
                Event::Admit,
                Event::Dispatch,
                Event::Settle(Settlement::Ready),
            ],
        ),
        (
            "d-cancel",
            vec![Event::Admit, Event::Dispatch, Event::Cancel],
        ),
        (
            "e-accepted",
            vec![
                Event::Admit,
                Event::Dispatch,
                Event::Settle(Settlement::Ready),
                Event::Accept,
            ],
        ),
    ] {
        for e in events {
            store.apply(&tid(name)?, e)?;
        }
    }
    let before = store.event_count()?;
    let report = reconcile(&store, &Observations::worker_absent())?;
    assert!(report.complete, "{:?}", report.findings);
    assert_eq!(report.applied, 2);
    assert_eq!(store.event_count()?, before + 2);
    let got: Vec<_> = report
        .rows
        .iter()
        .map(|r| (r.task_id.as_str().to_owned(), r.rule, r.before, r.after))
        .collect();
    assert_eq!(
        got,
        vec![
            (
                "a-running".into(),
                Some(RecoveryRule::R08WorkerAbsent),
                Phase::Running,
                Phase::EffectUnknown { cancel: false }
            ),
            ("b-admitted".into(), None, Phase::Admitted, Phase::Admitted),
            (
                "c-verifying".into(),
                Some(RecoveryRule::R12VerificationBoundary),
                Phase::Verifying,
                Phase::Verifying
            ),
            (
                "d-cancel".into(),
                Some(RecoveryRule::R08WorkerAbsent),
                Phase::CancellationRequested,
                Phase::EffectUnknown { cancel: true }
            ),
            (
                "e-accepted".into(),
                Some(RecoveryRule::R04AcceptanceStands),
                Phase::Accepted,
                Phase::Accepted
            ),
        ]
    );
    assert_eq!(
        store.history(&tid("a-running")?)?.last(),
        Some(&Event::Recover(RecoveryRule::R08WorkerAbsent))
    );
    for (name, state, outcome) in [
        ("a-running", AttemptState::Unknown, AttemptOutcome::R08),
        ("c-verifying", AttemptState::Settled, AttemptOutcome::Ready),
        ("e-accepted", AttemptState::Settled, AttemptOutcome::Ready),
    ] {
        let row = store.latest_attempt(&tid(name)?)?.ok_or("attempt row")?;
        assert_eq!(
            (
                row.state,
                row.outcome,
                row.cleanup,
                row.closed_seq.is_some()
            ),
            (state, Some(outcome), Cleanup::Pending, true),
            "{name}"
        );
    }
    let second = reconcile(&store, &Observations::worker_absent())?;
    assert_eq!((second.applied, second.complete), (0, true));
    assert_eq!(store.event_count()?, before + 2);
    Ok(())
}

#[test]
fn cache_divergence_is_healed_and_recorded() -> R {
    let path = db("heal")?;
    let store = Store::open(&path)?;
    assert!(reconcile(&store, &Observations::default())?.complete);
    let t = tid("task-heal-1")?;
    store.apply(&t, Event::Admit)?;
    assert_eq!(store.cache_heals()?.len(), 0);
    {
        let raw = rusqlite::Connection::open(&path)?;
        raw.execute(
            "UPDATE tasks SET phase = 'failed' WHERE id = 'task-heal-1'",
            [],
        )?;
    }
    assert_eq!(store.apply(&t, Event::Dispatch)?, Phase::Running);
    let healed = store.cache_heals()?;
    assert_eq!(healed.len(), 1, "{healed:?}");
    assert_eq!(healed[0].task_id, t);
    assert_eq!(healed[0].cached_phase, "failed");
    assert_eq!(healed[0].replayed_phase, "admitted");
    let u = tid("task-heal-2")?;
    store.apply(&u, Event::Admit)?;
    {
        let raw = rusqlite::Connection::open(&path)?;
        raw.execute(
            "UPDATE tasks SET phase = 'failed' WHERE id = 'task-heal-2'",
            [],
        )?;
    }
    let report = reconcile(&store, &Observations::default())?;
    assert_eq!(
        report.findings.len(),
        1,
        "reconcile still reports CacheMismatch"
    );
    assert!(!report.complete);
    Ok(())
}

#[test]
fn a_foreign_event_spelling_is_unreadable_history_never_repaired() -> R {
    let path = db("foreign-spelling")?;
    let store = Store::open(&path)?;
    assert!(reconcile(&store, &Observations::default())?.complete);
    let t = tid("task-foreign-1")?;
    store.apply(&t, Event::Admit)?;
    {
        let raw = rusqlite::Connection::open(&path)?;
        raw.execute(
            "INSERT INTO events(task_id, event_json, ts) VALUES ('task-foreign-1', '\"queued\"', 0)",
            [],
        )?;
    }
    let got = store.history(&t);
    assert!(
        matches!(&got, Err(StoreError::Corrupt { task, detail })
            if task == "task-foreign-1" && detail.contains("queued")),
        "{got:?}"
    );
    Ok(())
}

#[test]
fn admit_records_no_operation_and_no_event_when_its_closure_refuses() -> R {
    let store = ready("admit-refused")?;
    let t = tid("t1")?;
    store.apply(&t, Event::Admit)?;
    let before = store.event_count()?;
    // The second Admit of an existing task is refused inside operate's closure: nothing lands.
    let err = store.admit(&t, &op("k-refused"), b"spec", |_, _| json!("never"));
    assert!(matches!(err, Err(StoreError::Refused(_))), "{err:?}");
    assert_eq!(store.operation_by_key(&op("k-refused"))?, None);
    assert_eq!(store.event_count()?, before);
    Ok(())
}

#[test]
fn operation_readbacks_by_key_and_by_subject() -> R {
    let store = ready("readbacks")?;
    let t1 = tid("t1")?;
    let first = store.admit(
        &t1,
        &op("k1"),
        b"spec-A",
        |id, p| json!({"id": id.as_str(), "state": p.as_str()}),
    )?;
    let expected_id = format!(
        "op-{}",
        &Sha256Hex::digest(b"luke\ntask.submit\n1\nk1").to_string()[..24]
    );
    let row = store.operation_by_key(&op("k1"))?.ok_or("row")?;
    assert_eq!(row.operation_id, expected_id, "derived, never random");
    assert_eq!(row.action, "task.submit");
    assert_eq!(row.idem_key, "k1");
    assert_eq!(row.task_id, Some(t1.clone()));
    assert_eq!(row.subject.as_deref(), Some("t1"));
    assert_eq!(row.result, first.result);
    let replay = store.admit(&tid("t9")?, &op("k1"), b"spec-A", |_, _| json!("never"))?;
    assert!(replay.replayed);
    assert_eq!(
        store.operation_by_key(&op("k1"))?.map(|r| r.operation_id),
        Some(expected_id.clone()),
        "a replay keeps the stored id"
    );
    assert_eq!(
        store.last_operation_for("t1")?.map(|r| r.operation_id),
        Some(expected_id)
    );
    let t2 = tid("t2")?;
    store.admit(&t2, &op("k2"), b"spec-B", |_, _| json!("second"))?;
    assert_eq!(
        store
            .last_operation_for("t2")?
            .map(|r| (r.idem_key, r.result)),
        Some(("k2".into(), json!("second")))
    );
    assert_eq!(
        store.last_operation_for("t1")?.map(|r| r.idem_key),
        Some("k1".into())
    );
    assert_eq!(store.last_operation_for("nobody")?, None);
    assert_eq!(store.operation_by_key(&op("k-none"))?, None);
    Ok(())
}
