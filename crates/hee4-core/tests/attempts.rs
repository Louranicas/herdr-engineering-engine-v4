//! The attempts ledger: the migration, the `apply_in` hook, the verbs and triggers by name,
//! and the reconcile rows it enables (R03, R06, R07, R08 reasons, R09, R13). Tests may hold
//! SQL (a test-side `rusqlite::Connection` plants rows); `src` may not.

use std::error::Error;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use hee4_contracts::{Event, Phase, RecoveryRule, Refusal, Settlement, TaskId};
use hee4_core::recovery::{
    AttemptFacts, Clock, Facts, Finding, ProcessCustody, R08Reason, WorkspaceReadback,
    WorkspaceReuseRefused, cursor, decide,
};
use hee4_core::{
    AttemptId, AttemptOutcome, AttemptStart, AttemptState, Cleanup, CursorVerdict, Effect, Lease,
    Observations, Store, StoreError, probe, reconcile,
};
use rusqlite::Connection;

type R = Result<(), Box<dyn Error>>;

/// One row of `decide_reason_table`: attempt, custody, then the expected rule, event, reason.
type ReasonCase<'a> = (
    Option<&'a AttemptFacts>,
    ProcessCustody,
    Option<RecoveryRule>,
    Option<Event>,
    Option<R08Reason>,
);

fn tmp() -> Result<PathBuf, Box<dyn Error>> {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("hee4-core-attempts");
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

fn db(name: &str) -> Result<PathBuf, Box<dyn Error>> {
    let path = tmp()?.join(format!("{name}.sqlite"));
    for suffix in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{}{suffix}", path.display()));
    }
    Ok(path)
}

/// A fresh store whose (empty) reconcile has completed, so `Dispatch` is open.
fn ready(name: &str) -> Result<(Store, PathBuf), Box<dyn Error>> {
    let path = db(name)?;
    let store = Store::open(&path)?;
    assert!(reconcile(&store, &Observations::default())?.complete);
    Ok((store, path))
}

fn tid(s: &str) -> Result<TaskId, Box<dyn Error>> {
    Ok(s.parse()?)
}

fn aid(s: &str) -> Result<AttemptId, Box<dyn Error>> {
    Ok(s.parse()?)
}

/// A workspace directory with one 5-byte file.
fn workspace(name: &str) -> Result<PathBuf, Box<dyn Error>> {
    let dir = tmp()?.join(format!("ws-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir)?;
    std::fs::write(dir.join("note"), b"hello")?;
    Ok(dir)
}

fn start(
    task: &str,
    workspace: &Path,
    lease: Option<Lease>,
) -> Result<AttemptStart, Box<dyn Error>> {
    Ok(AttemptStart {
        receipt_id: format!("r-{task}-test").parse()?,
        permit_id: 1,
        model: "none".into(),
        head_sha: "a".repeat(40).parse()?,
        workspace: workspace.to_path_buf(),
        lease,
    })
}

fn dispatched(store: &Store, task: &str) -> Result<TaskId, Box<dyn Error>> {
    let t = tid(task)?;
    store.apply(&t, Event::Admit)?;
    assert_eq!(store.apply(&t, Event::Dispatch)?, Phase::Running);
    Ok(t)
}

fn last_seq(store: &Store, task: &TaskId) -> Result<i64, Box<dyn Error>> {
    Ok(store
        .history_with_seq(task)?
        .last()
        .map(|(seq, _)| *seq)
        .ok_or("empty history")?)
}

/// Field 22 of `/proc/<pid>/stat`, test-side (the probe's parser is private).
fn proc_start_ticks(pid: u32) -> Result<u64, Box<dyn Error>> {
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat"))?;
    let (_, rest) = stat.rsplit_once(')').ok_or("no comm")?;
    Ok(rest
        .split_whitespace()
        .nth(19)
        .ok_or("short stat")?
        .parse()?)
}

fn facts(phase: Phase, attempt_open: bool) -> Facts {
    Facts {
        phase,
        generation: 1,
        attempt_open,
        closes: Vec::new(),
    }
}

fn attempt_facts(acknowledged: bool, lease: Option<Lease>) -> Result<AttemptFacts, Box<dyn Error>> {
    Ok(AttemptFacts {
        id: aid("a-t1-1")?,
        generation: 1,
        dispatch_seq: 2,
        state: AttemptState::Running,
        acknowledged,
        pid: None,
        workspace: acknowledged.then(|| PathBuf::from("/nowhere")),
        lease,
        cleanup: Cleanup::None,
        closed_seq: None,
        outcome: None,
    })
}

#[test]
fn migration_registers_attempts_once() -> R {
    let path = db("migration")?;
    let count = |conn: &Connection| -> Result<(i64, i64), Box<dyn Error>> {
        let rows: i64 = conn.query_row(
            "SELECT count(*) FROM meta WHERE key = 'migration:m005_attempts'",
            [],
            |r| r.get(0),
        )?;
        let tables: i64 = conn.query_row(
            "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name = 'attempts'",
            [],
            |r| r.get(0),
        )?;
        Ok((rows, tables))
    };
    let store = Store::open(&path)?;
    assert_eq!(store.integrity_check()?, "ok");
    let meta_rows: i64 = {
        let raw = Connection::open(&path)?;
        assert_eq!(count(&raw)?, (1, 1), "fresh file");
        raw.query_row("SELECT count(*) FROM meta", [], |r| r.get(0))?
    };
    drop(store);
    // A foundation-era file: the four earlier rows, no m005 row, no attempts table.
    {
        let raw = Connection::open(&path)?;
        raw.execute_batch(
            "DELETE FROM meta WHERE key = 'migration:m005_attempts'; DROP TABLE attempts;",
        )?;
        assert_eq!(count(&raw)?, (0, 0));
    }
    let store = Store::open(&path)?;
    assert_eq!(store.integrity_check()?, "ok");
    {
        let raw = Connection::open(&path)?;
        assert_eq!(count(&raw)?, (1, 1), "foundation-era file migrated");
        let again: i64 = raw.query_row("SELECT count(*) FROM meta", [], |r| r.get(0))?;
        assert_eq!(again, meta_rows, "the same meta rows as a fresh file");
    }
    drop(store);
    let _ = Store::open(&path)?;
    let raw = Connection::open(&path)?;
    assert_eq!(count(&raw)?, (1, 1), "a third open adds nothing");
    Ok(())
}

#[test]
fn dispatch_opens_row_in_same_commit() -> R {
    let (store, _) = ready("dispatch")?;
    let t = tid("t1")?;
    store.apply(&t, Event::Admit)?;
    assert_eq!(store.attempts(&t)?, vec![], "no row before Dispatch");
    store.apply(&t, Event::Dispatch)?;
    let rows = store.attempts(&t)?;
    assert_eq!(rows.len(), 1);
    let row = &rows[0];
    assert_eq!(row.id, AttemptId::new(&t, 1));
    assert_eq!(row.id.to_string(), "a-t1-1");
    assert_eq!(aid("a-t1-1")?, row.id);
    assert_eq!(
        (
            row.state,
            row.effect,
            row.cleanup,
            row.dispatch_seq,
            row.generation
        ),
        (AttemptState::Running, Effect::None, Cleanup::None, 2, 1)
    );
    assert_eq!(row.dispatch_seq, last_seq(&store, &t)?);
    assert_eq!(
        store.history_with_seq(&t)?,
        [(1, Event::Admit), (2, Event::Dispatch)]
    );
    assert!(!row.acknowledged());
    assert_eq!(
        (row.started.clone(), row.pid, row.closed_seq, row.outcome),
        (None, None, None, None)
    );
    assert!(row.started_ms > 0);
    assert_eq!(store.open_attempts()?, rows);
    assert_eq!(store.latest_attempt(&t)?.as_ref(), Some(row));
    assert_eq!(store.attempt(&row.id)?.as_ref(), Some(row));
    for bad in ["b-t1-1", "a-t1-0", "a-t1", "a-t1-x", "a--1"] {
        assert!(bad.parse::<AttemptId>().is_err(), "{bad} must not parse");
    }
    Ok(())
}

#[test]
fn settle_and_recover_close_the_row() -> R {
    let (store, _) = ready("close")?;
    let cases = [
        (
            "s-ready",
            Event::Settle(Settlement::Ready),
            AttemptState::Settled,
            Effect::None,
            AttemptOutcome::Ready,
        ),
        (
            "s-notready",
            Event::Settle(Settlement::NotReady),
            AttemptState::Settled,
            Effect::None,
            AttemptOutcome::NotReady,
        ),
        (
            "s-unsettled",
            Event::Settle(Settlement::Unsettled),
            AttemptState::Unknown,
            Effect::Unknown,
            AttemptOutcome::Unsettled,
        ),
        (
            "r07",
            Event::Recover(RecoveryRule::R07ProcessNotOurs),
            AttemptState::Unknown,
            Effect::Unknown,
            AttemptOutcome::R07,
        ),
        (
            "r08",
            Event::Recover(RecoveryRule::R08WorkerAbsent),
            AttemptState::Unknown,
            Effect::Unknown,
            AttemptOutcome::R08,
        ),
    ];
    for (name, close, state, effect, outcome) in cases {
        let t = dispatched(&store, name)?;
        store.apply(&t, close)?;
        let row = store.latest_attempt(&t)?.ok_or("row")?;
        assert_eq!(
            (row.state, row.effect, row.cleanup, row.outcome),
            (state, effect, Cleanup::Pending, Some(outcome)),
            "{name}"
        );
        assert_eq!(row.closed_seq, Some(last_seq(&store, &t)?), "{name}");
        assert!(store.open_attempts()?.iter().all(|r| r.task_id != t));
    }
    Ok(())
}

#[test]
fn close_without_open_row_is_refused_and_writes_nothing() -> R {
    let (store, path) = ready("missing")?;
    let t = dispatched(&store, "t1")?;
    {
        // Closed by hand (no trigger forbids running -> unknown): the events still say open.
        let raw = Connection::open(&path)?;
        raw.execute(
            "UPDATE attempts SET state = 'unknown', effect = 'unknown', cleanup = 'pending',
               outcome = 'unsettled' WHERE id = 'a-t1-1'",
            [],
        )?;
    }
    let before = store.event_count()?;
    let err = store.apply(&t, Event::Settle(Settlement::Ready));
    assert!(
        matches!(&err, Err(StoreError::AttemptMissing { task, generation: 1 }) if *task == t),
        "{err:?}"
    );
    assert_eq!(store.event_count()?, before, "fail closed: no event");
    assert_eq!(store.phase(&t)?, Some(Phase::Running));
    assert_eq!(store.cached(&t)?.ok_or("row")?.phase, "running");
    Ok(())
}

#[test]
fn verbs_refuse_by_name() -> R {
    let (store, _) = ready("verbs")?;
    let shared = workspace("shared")?;
    let t1 = dispatched(&store, "t1")?;
    store.apply(&t1, Event::Settle(Settlement::Ready))?;
    let a1 = aid("a-t1-1")?;
    let err = store.attempt_started(&a1, &start("t1", &shared, None)?);
    assert!(
        matches!(&err, Err(StoreError::AttemptClosed { id, state: AttemptState::Settled }) if *id == a1),
        "{err:?}"
    );
    let err = store.attempt_pid(&a1, 1, 1);
    assert!(
        matches!(err, Err(StoreError::AttemptClosed { .. })),
        "{err:?}"
    );
    let ghost = aid("a-ghost-1")?;
    let err = store.attempt_started(&ghost, &start("ghost", &shared, None)?);
    assert!(
        matches!(&err, Err(StoreError::AttemptMissing { task, generation: 1 }) if task.as_str() == "ghost"),
        "{err:?}"
    );
    assert!(matches!(
        store.attempt_pid(&ghost, 1, 1),
        Err(StoreError::AttemptMissing { .. })
    ));
    assert!(matches!(
        store.attempt_cleanup_settled(&ghost),
        Err(StoreError::AttemptMissing { .. })
    ));

    let t2 = dispatched(&store, "t2")?;
    let a2 = aid("a-t2-1")?;
    let lease = Lease {
        deadline_ms: 1_000,
        clock_epoch: "boot-1".into(),
    };
    store.attempt_started(&a2, &start("t2", &shared, Some(lease.clone()))?)?;
    let row = store.attempt(&a2)?.ok_or("row")?;
    assert!(row.acknowledged());
    let started = row.started.ok_or("started")?;
    assert_eq!(started.receipt_id.as_str(), "r-t2-test");
    assert_eq!((started.permit_id, started.model.as_str()), (1, "none"));
    assert_eq!(started.workspace, shared);
    assert_eq!(started.lease, Some(lease));

    let _t3 = dispatched(&store, "t3")?;
    let a3 = aid("a-t3-1")?;
    let err = store.attempt_started(&a3, &start("t3", &shared, None)?);
    assert!(
        matches!(&err, Err(StoreError::WorkspaceLeased { workspace, other })
            if *workspace == shared && *other == a2),
        "{err:?}"
    );
    assert!(
        !store.attempt(&a3)?.ok_or("row")?.acknowledged(),
        "refused: nothing written"
    );

    let err = store.attempt_cleanup_settled(&a2);
    assert!(
        matches!(&err, Err(StoreError::AttemptOpen { id }) if *id == a2),
        "{err:?}"
    );
    store.apply(&t2, Event::Settle(Settlement::Ready))?;
    assert_eq!(store.attempt(&a2)?.ok_or("row")?.cleanup, Cleanup::Pending);
    let err = store.attempt_started(&a3, &start("t3", &shared, None)?);
    assert!(
        matches!(err, Err(StoreError::WorkspaceLeased { .. })),
        "closed but cleanup pending still leases"
    );
    store.attempt_cleanup_settled(&a2)?;
    assert_eq!(store.attempt(&a2)?.ok_or("row")?.cleanup, Cleanup::Settled);
    store.attempt_started(&a3, &start("t3", &shared, None)?)?;
    assert!(store.attempt(&a3)?.ok_or("row")?.acknowledged());

    store.attempt_pid(&a3, 4242, 7)?;
    store.attempt_pid(&a3, 4243, 8)?;
    assert_eq!(
        store.attempt(&a3)?.ok_or("row")?.pid,
        Some((4243, 8)),
        "last pid wins"
    );
    Ok(())
}

#[test]
fn triggers_hold() -> R {
    let (store, path) = ready("triggers")?;
    dispatched(&store, "t1")?;
    let t2 = dispatched(&store, "t2")?;
    store.apply(&t2, Event::Settle(Settlement::Ready))?;
    let raw = Connection::open(&path)?;
    let text = |sql: &str| -> String {
        raw.execute(sql, [])
            .err()
            .map_or_else(|| "NO ERROR".into(), |e| e.to_string())
    };
    assert!(text("DELETE FROM attempts WHERE id = 'a-t1-1'").contains("append-only"));
    assert!(text("DELETE FROM attempts").contains("append-only"));
    for col in [
        "task_id = 't2'",
        "generation = 9",
        "dispatch_seq = 1",
        "started_ms = 0",
    ] {
        let msg = text(&format!("UPDATE attempts SET {col} WHERE id = 'a-t1-1'"));
        assert!(msg.contains("immutable"), "{col}: {msg}");
    }
    let msg = text("UPDATE attempts SET state = 'running' WHERE id = 'a-t2-1'");
    assert!(msg.contains("IA-1"), "{msg}");
    let msg = text(
        "INSERT INTO attempts(id, task_id, generation, dispatch_seq, started_ms, state, effect,
           cleanup) VALUES ('a-t1-1-dup', 't1', 1, 2, 0, 'running', 'none', 'none')",
    );
    assert!(
        msg.contains("UNIQUE"),
        "IA-3 by UNIQUE(task_id, generation): {msg}"
    );
    let rows: i64 = raw.query_row("SELECT count(*) FROM attempts", [], |r| r.get(0))?;
    assert_eq!(rows, 2, "nothing landed");
    assert_eq!(
        store.attempt(&aid("a-t2-1")?)?.ok_or("row")?.state,
        AttemptState::Settled
    );
    Ok(())
}

#[test]
fn decide_reason_table() -> R {
    use ProcessCustody as C;
    use RecoveryRule as R;
    let open = facts(Phase::Running, true);
    let unacked = attempt_facts(false, None)?;
    let acked = attempt_facts(true, None)?;
    let cases: [ReasonCase<'_>; 5] = [
        (
            None,
            C::Absent,
            Some(R::R08WorkerAbsent),
            Some(Event::Recover(R::R08WorkerAbsent)),
            Some(R08Reason::AcknowledgementUnrecorded),
        ),
        (
            Some(&unacked),
            C::Absent,
            Some(R::R08WorkerAbsent),
            Some(Event::Recover(R::R08WorkerAbsent)),
            Some(R08Reason::DispatchUnacknowledged),
        ),
        (
            Some(&acked),
            C::Absent,
            Some(R::R08WorkerAbsent),
            Some(Event::Recover(R::R08WorkerAbsent)),
            Some(R08Reason::AcknowledgedWorkerLost),
        ),
        (
            Some(&acked),
            C::PidReused,
            Some(R::R07ProcessNotOurs),
            Some(Event::Recover(R::R07ProcessNotOurs)),
            None,
        ),
        (
            Some(&acked),
            C::LiveSameIdentity,
            Some(R::R06LiveOwnedChild),
            None,
            None,
        ),
    ];
    for (attempt, custody, rule, event, reason) in cases {
        let d = decide(
            "e",
            &open,
            attempt,
            custody,
            WorkspaceReadback::Unobserved,
            None,
            None,
        );
        assert_eq!(
            (d.rule, d.event, d.reason, d.workspace, d.contradiction),
            (rule, event, reason, None, false),
            "{attempt:?} {custody:?}"
        );
    }
    Ok(())
}

#[test]
fn decide_workspace_table() -> R {
    use WorkspaceReuseRefused as W;
    let open = facts(Phase::Running, true);
    let lease = |epoch: &str| {
        Some(Lease {
            deadline_ms: 100,
            clock_epoch: epoch.into(),
        })
    };
    let clock = |epoch: &str, now_ms: i64| Clock {
        now_ms,
        clock_epoch: epoch.into(),
    };
    let writable = WorkspaceReadback::Writable { bytes: 10 };
    let cases: [(Option<Lease>, Option<Clock>, Option<W>); 5] = [
        (None, Some(clock("b1", 200)), Some(W::NotLeasedWritable)),
        (lease("b1"), None, Some(W::ClockUnavailable)),
        (
            lease("b1"),
            Some(clock("b2", 200)),
            Some(W::LeaseClockNotComparable),
        ),
        (
            lease("b1"),
            Some(clock("b1", 200)),
            Some(W::LeaseExpiredWritable),
        ),
        (lease("b1"), Some(clock("b1", 50)), None),
    ];
    for (lease, clock, expected) in cases {
        let attempt = attempt_facts(true, lease.clone())?;
        let d = decide(
            "e",
            &open,
            Some(&attempt),
            ProcessCustody::Absent,
            writable,
            clock.as_ref(),
            None,
        );
        assert_eq!(d.workspace, expected, "{lease:?} {clock:?}");
        assert_eq!(
            (d.rule, d.event, d.reason),
            (
                Some(RecoveryRule::R08WorkerAbsent),
                Some(Event::Recover(RecoveryRule::R08WorkerAbsent)),
                Some(R08Reason::AcknowledgedWorkerLost)
            ),
            "the attachment never changes the rule or the event"
        );
        for other in [
            WorkspaceReadback::Absent,
            WorkspaceReadback::Unreadable,
            WorkspaceReadback::Unobserved,
        ] {
            let d = decide(
                "e",
                &open,
                Some(&attempt),
                ProcessCustody::Absent,
                other,
                clock.as_ref(),
                None,
            );
            assert_eq!(d.workspace, None, "{other:?} attaches nothing");
        }
    }
    Ok(())
}

/// R11: a closed row with cleanup pending under a writable workspace attaches R09; once
/// cleanup is settled nothing attaches.
#[test]
fn decide_workspace_on_cleanup_readback() -> R {
    use WorkspaceReuseRefused as W;
    let writable = WorkspaceReadback::Writable { bytes: 10 };
    let mut closed = attempt_facts(true, None)?;
    closed.state = AttemptState::Settled;
    closed.cleanup = Cleanup::Pending;
    closed.closed_seq = Some(3);
    closed.outcome = Some(AttemptOutcome::NotReady);
    let mut repair = facts(Phase::RepairPending, false);
    repair.closes = vec![(3, Event::Settle(Settlement::NotReady))];
    let d = decide(
        "e",
        &repair,
        Some(&closed),
        ProcessCustody::Absent,
        writable,
        None,
        None,
    );
    assert_eq!(
        (d.rule, d.event, d.workspace, d.contradiction),
        (
            Some(RecoveryRule::R11CleanupReadback),
            None,
            Some(W::NotLeasedWritable),
            false
        )
    );
    closed.cleanup = Cleanup::Settled;
    let d = decide(
        "e",
        &repair,
        Some(&closed),
        ProcessCustody::Absent,
        writable,
        None,
        None,
    );
    assert_eq!(
        (d.rule, d.workspace),
        (Some(RecoveryRule::R11CleanupReadback), None)
    );
    Ok(())
}

#[test]
fn r06_live_child_keeps_running_then_r08_when_it_dies() -> R {
    let (store, _) = ready("r06")?;
    let t = dispatched(&store, "t1")?;
    let ws = workspace("r06")?;
    let id = AttemptId::new(&t, 1);
    store.attempt_started(&id, &start("t1", &ws, None)?)?;
    let mut child = Command::new("/usr/bin/sleep")
        .arg("300")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    let pid = child.id();
    let ticks = proc_start_ticks(pid)?;
    store.attempt_pid(&id, pid, ticks)?;

    let observed = probe::observe(&store.open_attempts()?, 1 << 20);
    assert_eq!(
        observed.process.get(&t),
        Some(&ProcessCustody::LiveSameIdentity)
    );
    assert_eq!(
        observed.workspace.get(&id),
        Some(&WorkspaceReadback::Writable { bytes: 5 })
    );
    assert!(observed.clock.is_some(), "boot_id readable on this host");
    let report = reconcile(&store, &observed)?;
    let row = &report.rows[0];
    assert_eq!(
        (row.rule, row.before, row.after, row.reason, row.workspace),
        (
            Some(RecoveryRule::R06LiveOwnedChild),
            Phase::Running,
            Phase::Running,
            None,
            None
        )
    );
    assert_eq!((report.applied, report.complete), (0, true));
    assert!(store.recovery_complete()?);
    assert_eq!(
        store.attempt(&id)?.ok_or("row")?.state,
        AttemptState::Running
    );
    println!(
        "r06: child pid={pid} start_ticks={ticks} rule={:?} applied={}",
        row.rule, report.applied
    );

    child.kill()?; // SIGKILL to this child's pid only (AP-38)
    let status = child.wait()?;
    assert!(!status.success());
    let observed = probe::observe(&store.open_attempts()?, 1 << 20);
    assert_eq!(observed.process.get(&t), Some(&ProcessCustody::Absent));
    let report = reconcile(&store, &observed)?;
    let row = &report.rows[0];
    assert_eq!(
        (row.rule, row.after, row.reason, row.workspace),
        (
            Some(RecoveryRule::R08WorkerAbsent),
            Phase::EffectUnknown { cancel: false },
            Some(R08Reason::AcknowledgedWorkerLost),
            Some(WorkspaceReuseRefused::NotLeasedWritable)
        )
    );
    assert_eq!((report.applied, report.complete), (1, true));
    let after = store.attempt(&id)?.ok_or("row")?;
    assert_eq!(
        (after.state, after.outcome, after.cleanup, after.closed_seq),
        (
            AttemptState::Unknown,
            Some(AttemptOutcome::R08),
            Cleanup::Pending,
            Some(last_seq(&store, &t)?)
        )
    );
    println!(
        "r06->r08: status={status} rule={:?} reason={:?} workspace={:?}",
        row.rule, row.reason, row.workspace
    );
    Ok(())
}

#[test]
fn pid_reused_is_r07() -> R {
    let (store, _) = ready("r07")?;
    let t = dispatched(&store, "t1")?;
    let ws = workspace("r07")?;
    let id = AttemptId::new(&t, 1);
    store.attempt_started(&id, &start("t1", &ws, None)?)?;
    let (pid, ticks) = probe::self_identity().ok_or("self identity")?;
    assert_eq!(pid, std::process::id());
    store.attempt_pid(&id, pid, ticks + 1)?;
    let observed = probe::observe(&store.open_attempts()?, 1 << 20);
    assert_eq!(observed.process.get(&t), Some(&ProcessCustody::PidReused));
    let report = reconcile(&store, &observed)?;
    let row = &report.rows[0];
    assert_eq!(
        (row.rule, row.after, row.reason),
        (
            Some(RecoveryRule::R07ProcessNotOurs),
            Phase::EffectUnknown { cancel: false },
            None
        )
    );
    let after = store.attempt(&id)?.ok_or("row")?;
    assert_eq!(
        (after.state, after.outcome),
        (AttemptState::Unknown, Some(AttemptOutcome::R07))
    );
    Ok(())
}

#[test]
fn planted_contradiction_withholds_complete() -> R {
    let (store, path) = ready("r03")?;
    let t = dispatched(&store, "t1")?;
    {
        let raw = Connection::open(&path)?;
        raw.execute(
            "UPDATE attempts SET state = 'settled', outcome = 'ready', closed_seq = NULL
             WHERE id = 'a-t1-1'",
            [],
        )?;
    }
    let history = store.history(&t)?;
    let expected = vec![Finding::Contradictory {
        task_id: t.clone(),
        attempt_id: aid("a-t1-1")?,
        rule: RecoveryRule::R03CommitOrdering,
    }];
    for pass in 1..=2 {
        let report = reconcile(&store, &Observations::worker_absent())?;
        assert_eq!(report.findings, expected, "pass {pass}");
        assert_eq!((report.applied, report.complete), (0, false), "pass {pass}");
        assert!(!store.recovery_complete()?, "pass {pass}");
        let row = &report.rows[0];
        assert_eq!(
            (row.rule, row.before, row.after),
            (
                Some(RecoveryRule::R03CommitOrdering),
                Phase::Running,
                Phase::Running
            )
        );
        assert_eq!(
            store.history(&t)?,
            history,
            "pass {pass}: history unchanged"
        );
        assert_eq!(store.phase(&t)?, Some(Phase::Running));
    }
    let t9 = tid("t9")?;
    store.apply(&t9, Event::Admit)?;
    let refused = store.apply(&t9, Event::Dispatch);
    assert!(
        matches!(refused, Err(StoreError::RecoveryIncomplete)),
        "no Dispatch over a contradictory ledger: {refused:?}"
    );
    Ok(())
}

#[test]
fn cursor_four_verdicts() -> R {
    let path = db("cursor")?;
    let store = Store::open(&path)?;
    let epoch = store.epoch()?;
    store.apply(&tid("t1")?, Event::Admit)?;
    assert_eq!(store.event_high_water()?, 1);
    let d = cursor(&store, &epoch, 1)?;
    assert_eq!((d.verdict, d.rule), (CursorVerdict::SnapshotOnly, None));
    let d = cursor(&store, &epoch, 2)?;
    assert_eq!(
        (d.verdict, d.rule),
        (
            CursorVerdict::FutureSequence,
            Some(RecoveryRule::R13CursorEpoch)
        )
    );
    let d = cursor(&store, "other-epoch", 0)?;
    assert_eq!(
        (d.verdict, d.rule),
        (
            CursorVerdict::EpochChanged,
            Some(RecoveryRule::R13CursorEpoch)
        )
    );
    store.mark_restored_from("old-epoch")?;
    let d = cursor(&store, "old-epoch", 0)?;
    assert_eq!(
        (d.verdict, d.rule),
        (
            CursorVerdict::PriorEpochOfRestore,
            Some(RecoveryRule::R13CursorEpoch)
        )
    );
    assert_eq!(store.event_count()?, 1, "a cursor verdict writes nothing");
    Ok(())
}

#[test]
fn redispatch_opens_generation_two() -> R {
    let (store, _) = ready("gen2")?;
    let t = dispatched(&store, "t1")?;
    let err = store.apply(&t, Event::Dispatch);
    assert!(
        matches!(err, Err(StoreError::Refused(Refusal::Illegal { .. }))),
        "IA-3: a second Dispatch on a running task is refused by transition: {err:?}"
    );
    assert_eq!(
        store.apply(&t, Event::Settle(Settlement::NotReady))?,
        Phase::RepairPending
    );
    assert_eq!(store.apply(&t, Event::Dispatch)?, Phase::Running);
    let rows = store.attempts(&t)?;
    assert_eq!(rows.len(), 2);
    assert_eq!(
        (rows[0].generation, rows[0].state, rows[0].outcome),
        (1, AttemptState::Settled, Some(AttemptOutcome::NotReady))
    );
    assert_eq!(
        (rows[1].generation, rows[1].state, rows[1].id.to_string()),
        (2, AttemptState::Running, "a-t1-2".into())
    );
    assert_eq!(rows[1].dispatch_seq, last_seq(&store, &t)?);
    assert_eq!(store.latest_attempt(&t)?.as_ref(), Some(&rows[1]));
    assert_eq!(store.open_attempts()?, vec![rows[1].clone()]);
    Ok(())
}
