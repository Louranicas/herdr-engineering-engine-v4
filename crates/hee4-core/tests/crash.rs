//! Crash test (acceptance 6): a child process (this test binary, `HEE4_CRASH_CHILD=<db>`)
//! admits and dispatches tasks through the real store in a loop, printing `ACK <task> <phase>`
//! after each `apply` returns; the parent `SIGKILL`s it by pid mid-loop, reopens the file, runs
//! `reconcile` with every worker absent, and checks "acked ⇒ present" plus R08's target.
//!
//! Two more child modes hold one attempt open: `HEE4_CRASH_ATTEMPT=1` (acknowledged with
//! `attempt_started` + `attempt_pid`, so R08 is `AcknowledgedWorkerLost` and the writable
//! workspace attaches R09) and `HEE4_CRASH_AFTER_DISPATCH=1` (killed before the
//! acknowledgement: `DispatchUnacknowledged`). The parent reads custody with `probe::observe`.
//! In `HEE4_CRASH_AFTER_DISPATCH` mode no pid was recorded, so the probe reads `Unobserved` (R07, with the
//! same `DispatchUnacknowledged` class, proven probe-only); the R08 row there uses custody the
//! parent supplies from its own reap of the child (hand-supplied, not probed).

use std::collections::BTreeMap;
use std::error::Error;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Child, Command, ExitStatus, Stdio};

use hee4_contracts::{Event, Phase, RecoveryRule, TaskId};
use hee4_core::recovery::{
    AttemptFacts, Facts, ProcessCustody, R08Reason, WorkspaceReadback, WorkspaceReuseRefused,
    decide,
};
use hee4_core::{
    AttemptId, AttemptOutcome, AttemptRow, AttemptStart, AttemptState, Cleanup, Effect,
    Observations, Store, probe, reconcile,
};

const CHILD_ENV: &str = "HEE4_CRASH_CHILD";
const AFTER_ADMIT_ENV: &str = "HEE4_CRASH_AFTER_ADMIT";
const ATTEMPT_ENV: &str = "HEE4_CRASH_ATTEMPT";
const AFTER_DISPATCH_ENV: &str = "HEE4_CRASH_AFTER_DISPATCH";
const KILL_AFTER_ACKS: usize = 60;

fn crash_dir() -> Result<PathBuf, Box<dyn Error>> {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("hee4-core-crash");
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

fn fresh_db(name: &str) -> Result<PathBuf, Box<dyn Error>> {
    let path = crash_dir()?.join(name);
    for suffix in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{}{suffix}", path.display()));
    }
    Ok(path)
}

fn flag(env: &str) -> bool {
    std::env::var_os(env).is_some_and(|v| v == "1")
}

/// The child half. A no-op unless `HEE4_CRASH_CHILD` names a database.
#[test]
fn crash_child() -> Result<(), Box<dyn Error>> {
    let Some(path) = std::env::var_os(CHILD_ENV) else {
        return Ok(());
    };
    let store = Store::open(&PathBuf::from(path))?;
    reconcile(&store, &Observations::default())?;
    let mut out = std::io::stdout().lock();
    let after_admit = flag(AFTER_ADMIT_ENV);
    let attempt_mode = flag(ATTEMPT_ENV);
    if attempt_mode || flag(AFTER_DISPATCH_ENV) {
        let task: TaskId = "task-000000".parse()?;
        store.apply(&task, Event::Admit)?;
        store.apply(&task, Event::Dispatch)?;
        if attempt_mode {
            let workspace = crash_dir()?.join("ws-task-000000");
            let _ = std::fs::remove_dir_all(&workspace);
            std::fs::create_dir_all(&workspace)?;
            std::fs::write(workspace.join("note"), b"crash")?;
            let id = AttemptId::new(&task, 1);
            store.attempt_started(
                &id,
                &AttemptStart {
                    receipt_id: "r-task-000000-crash".parse()?,
                    permit_id: 1,
                    model: "none".into(),
                    head_sha: "a".repeat(40).parse()?,
                    workspace,
                    lease: None,
                },
            )?;
            let (pid, start_ticks) = probe::self_identity().ok_or("no self identity")?;
            store.attempt_pid(&id, pid, start_ticks)?;
        }
        writeln!(
            out,
            "ACK {task} {}",
            if attempt_mode { "attempt" } else { "running" }
        )?;
        out.flush()?;
        // Hold until the parent kills us.
        let mut hold = String::new();
        std::io::stdin().read_line(&mut hold)?;
        return Ok(());
    }
    for i in 0..100_000u32 {
        let task: TaskId = format!("task-{i:06}").parse()?;
        for event in [Event::Admit, Event::Dispatch] {
            let phase = store.apply(&task, event)?;
            writeln!(out, "ACK {task} {}", phase.as_str())?;
            out.flush()?;
            if after_admit {
                // Hold between the Admit ack and Dispatch until the parent kills us.
                let mut hold = String::new();
                std::io::stdin().read_line(&mut hold)?;
                return Ok(());
            }
        }
    }
    Ok(())
}

fn spawn_child(path: &PathBuf, mode_env: &str) -> Result<Child, Box<dyn Error>> {
    Ok(Command::new(std::env::current_exe()?)
        .args(["crash_child", "--exact", "--nocapture", "--test-threads=1"])
        .env(CHILD_ENV, path)
        .env(mode_env, "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?)
}

/// Spawn the child in `mode_env`, SIGKILL it (by pid only, AP-38) right after its first ACK,
/// reap it, and return the ACK text and the exit status.
fn kill_after_first_ack(
    path: &PathBuf,
    mode_env: &str,
) -> Result<(u32, String, ExitStatus), Box<dyn Error>> {
    let mut child = spawn_child(path, mode_env)?;
    let pid = child.id();
    let stdout = child.stdout.take().ok_or("no child stdout")?;
    let mut acked = None;
    for line in BufReader::new(stdout).lines() {
        let line = line?;
        if let Some(rest) = line.find("ACK ").map(|at| &line[at + 4..]) {
            acked = Some(rest.to_owned());
            child.kill()?; // SIGKILL to this child's pid only (AP-38)
            break;
        }
    }
    let status = child.wait()?;
    let acked = acked.ok_or("child never acked")?;
    assert!(!status.success(), "the child must die by signal");
    Ok((pid, acked, status))
}

/// The `admitted` landing: killed right after the first Admit ack, before any Dispatch.
#[test]
fn sigkill_after_admit_before_dispatch() -> Result<(), Box<dyn Error>> {
    let path = fresh_db("crash-admit.sqlite")?;
    let (pid, acked, status) = kill_after_first_ack(&path, AFTER_ADMIT_ENV)?;
    println!("crash: mode=after_admit child pid={pid} acked={acked:?} status={status}");
    assert_eq!(acked, "task-000000 admitted");

    let store = Store::open(&path)?;
    let id: TaskId = "task-000000".parse()?;
    assert_eq!(store.task_ids()?.len(), 1, "exactly the acked task");
    assert_eq!(
        store.history(&id)?,
        [Event::Admit],
        "no Dispatch, no attempt"
    );
    assert_eq!(store.phase(&id)?, Some(Phase::Admitted));
    assert_eq!(
        store.attempts(&id)?,
        vec![],
        "no attempt row without a Dispatch"
    );
    let report = reconcile(&store, &Observations::worker_absent())?;
    assert_eq!(report.rows.len(), 1);
    let row = &report.rows[0];
    assert_eq!(row.before, Phase::Admitted);
    assert_eq!(row.after, Phase::Admitted);
    assert_eq!(row.rule, None);
    assert!(report.complete, "{:?}", report.findings);
    println!(
        "crash: mode=after_admit landing=admitted rule={:?} complete={} integrity_check={}",
        row.rule,
        report.complete,
        store.integrity_check()?
    );
    Ok(())
}

/// Shared parent half of the two attempt modes: kill after the ACK, reopen, probe, reconcile
/// twice, check the reason class, the attachment and the attempt row.
/// What the probe's observation alone decides for an attempt killed before `attempt_started`
/// (no pid recorded, so custody is `Unobserved`): R07 with `DispatchUnacknowledged`.
fn probe_only_is_r07_unacknowledged(
    store: &Store,
    task: &TaskId,
    row: &AttemptRow,
    observed: &Observations,
    mode: &str,
) -> Result<(), Box<dyn Error>> {
    // Probe-only: what the observation alone decides, before the parent adds anything.
    let probe_only = decide(
        &store.epoch()?,
        &Facts::from_history(Phase::Running, &store.history_with_seq(task)?),
        Some(&AttemptFacts::from_row(row)),
        ProcessCustody::Unobserved,
        WorkspaceReadback::Unobserved,
        observed.clock.as_ref(),
        None,
    );
    assert_eq!(
        (probe_only.rule, probe_only.reason),
        (
            Some(RecoveryRule::R07ProcessNotOurs),
            Some(R08Reason::DispatchUnacknowledged)
        )
    );
    println!(
        "crash: mode={mode} probe_only rule={:?} reason={:?} (custody below is hand-supplied)",
        probe_only.rule, probe_only.reason
    );
    Ok(())
}

fn attempt_crash(
    db_name: &str,
    mode_env: &str,
    mode: &str,
    expected_ack: &str,
    reason: R08Reason,
    workspace: Option<WorkspaceReuseRefused>,
) -> Result<(), Box<dyn Error>> {
    let path = fresh_db(db_name)?;
    let (pid, acked, status) = kill_after_first_ack(&path, mode_env)?;
    println!("crash: mode={mode} child pid={pid} acked={acked:?} status={status}");
    assert_eq!(acked, expected_ack);

    let store = Store::open(&path)?;
    let task: TaskId = "task-000000".parse()?;
    let id = AttemptId::new(&task, 1);
    assert_eq!(
        store.task_ids()?,
        std::slice::from_ref(&task),
        "acked => present"
    );
    assert_eq!(store.phase(&task)?, Some(Phase::Running));
    let open = store.open_attempts()?;
    assert_eq!(open.len(), 1, "{open:?}");
    assert_eq!(open[0].id, id);
    assert_eq!(open[0].acknowledged(), mode_env == ATTEMPT_ENV);
    let acknowledged = open[0].acknowledged();
    let mut observed = probe::observe(&open, 1 << 20);
    if acknowledged {
        assert_eq!(
            open[0].pid.map(|(p, _)| p),
            Some(pid),
            "the child recorded itself"
        );
        assert_eq!(
            observed.process.get(&task),
            Some(&ProcessCustody::Absent),
            "the probe read ENOENT for the reaped child"
        );
        assert_eq!(
            observed.workspace.get(&id),
            Some(&WorkspaceReadback::Writable { bytes: 5 })
        );
    } else {
        // No pid was recorded before the kill, so the probe cannot know; the parent, which
        // reaped the child above, supplies the custody it measured.
        assert_eq!(
            observed.process.get(&task),
            Some(&ProcessCustody::Unobserved)
        );
        probe_only_is_r07_unacknowledged(&store, &task, &open[0], &observed, mode)?;
        observed
            .process
            .insert(task.clone(), ProcessCustody::Absent);
        assert_eq!(
            observed.workspace.get(&id),
            Some(&WorkspaceReadback::Unobserved)
        );
    }
    let report = reconcile(&store, &observed)?;
    assert_eq!(report.rows.len(), 1);
    let row = &report.rows[0];
    assert_eq!(row.rule, Some(RecoveryRule::R08WorkerAbsent));
    assert_eq!(row.reason, Some(reason));
    assert_eq!(row.workspace, workspace);
    assert_eq!(row.after, Phase::EffectUnknown { cancel: false });
    assert_eq!(
        (report.applied, report.complete),
        (1, true),
        "{:?}",
        report.findings
    );
    let (closing_seq, closing) = *store.history_with_seq(&task)?.last().ok_or("history")?;
    assert_eq!(closing, Event::Recover(RecoveryRule::R08WorkerAbsent));
    let after = store.attempt(&id)?.ok_or("attempt row")?;
    assert_eq!(
        (
            after.state,
            after.effect,
            after.outcome,
            after.cleanup,
            after.closed_seq
        ),
        (
            AttemptState::Unknown,
            Effect::Unknown,
            Some(AttemptOutcome::R08),
            Cleanup::Pending,
            Some(closing_seq)
        )
    );
    assert_eq!(store.open_attempts()?, Vec::<AttemptRow>::new());

    let second = reconcile(&store, &probe::observe(&store.open_attempts()?, 1 << 20))?;
    assert_eq!(
        (second.applied, second.complete),
        (0, true),
        "second pass converges"
    );
    assert_eq!(
        store.attempt(&id)?.as_ref(),
        Some(&after),
        "identical row after the second pass"
    );
    let integrity = store.integrity_check()?;
    println!(
        "crash: mode={mode} reason={:?} workspace={:?} closed_seq={:?} applied={} integrity_check={integrity}",
        row.reason, row.workspace, after.closed_seq, report.applied
    );
    assert_eq!(integrity, "ok");
    Ok(())
}

/// Killed after `attempt_started` + `attempt_pid`: an acknowledged attempt whose worker is
/// gone, with a writable, unleased workspace.
#[test]
fn sigkill_after_attempt_started() -> Result<(), Box<dyn Error>> {
    attempt_crash(
        "crash-attempt.sqlite",
        ATTEMPT_ENV,
        "attempt",
        "task-000000 attempt",
        R08Reason::AcknowledgedWorkerLost,
        Some(WorkspaceReuseRefused::NotLeasedWritable),
    )
}

/// Killed after the Dispatch ack, before any acknowledgement facts were written.
#[test]
fn sigkill_after_dispatch_before_attempt_started() -> Result<(), Box<dyn Error>> {
    attempt_crash(
        "crash-after-dispatch.sqlite",
        AFTER_DISPATCH_ENV,
        "after_dispatch",
        "task-000000 running",
        R08Reason::DispatchUnacknowledged,
        None,
    )
}

/// The task's attempt rows are exactly one, in `state` with `outcome`.
fn one_attempt(
    store: &Store,
    task: &TaskId,
    state: AttemptState,
    outcome: Option<AttemptOutcome>,
) -> Result<(), Box<dyn Error>> {
    let rows = store.attempts(task)?;
    assert_eq!(rows.len(), 1, "{task:?} has one attempt row: {rows:?}");
    assert_eq!(
        (rows[0].state, rows[0].outcome),
        (state, outcome),
        "{task:?}"
    );
    Ok(())
}

#[test]
fn sigkill_mid_loop_then_reconcile() -> Result<(), Box<dyn Error>> {
    let path = fresh_db("crash.sqlite")?;
    let mut child = Command::new(std::env::current_exe()?)
        .args(["crash_child", "--exact", "--nocapture", "--test-threads=1"])
        .env(CHILD_ENV, &path)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;
    let pid = child.id();
    let stdout = child.stdout.take().ok_or("no child stdout")?;
    let mut acked: BTreeMap<String, String> = BTreeMap::new();
    let mut acks = 0;
    for line in BufReader::new(stdout).lines() {
        let line = line?;
        // libtest may print `test crash_child ... ` on the same line as the first ACK.
        if let Some(rest) = line.find("ACK ").map(|at| &line[at + 4..]) {
            let (task, phase) = rest.split_once(' ').ok_or("bad ACK line")?;
            acked.insert(task.to_owned(), phase.to_owned());
            acks += 1;
            if acks == KILL_AFTER_ACKS {
                child.kill()?; // SIGKILL to this child's pid only (AP-38)
                break;
            }
        }
    }
    let status = child.wait()?;
    println!("crash: child pid={pid} killed after acks={acks} status={status}");
    assert!(
        !status.success(),
        "the child must die by signal, not finish"
    );

    let store = Store::open(&path)?;
    assert!(
        !store.recovery_complete()?,
        "gate is closed until reconcile"
    );
    let ids = store.task_ids()?;
    for (task, phase) in &acked {
        let id: TaskId = task.parse()?;
        assert!(
            ids.contains(&id),
            "acked {task} ({phase}) missing after kill"
        );
        if phase == "running" {
            assert!(
                store.history(&id)?.contains(&Event::Dispatch),
                "acked dispatch of {task} lost"
            );
            one_attempt(&store, &id, AttemptState::Running, None)?;
        }
    }
    let report = reconcile(&store, &Observations::worker_absent())?;
    let mut by_after: BTreeMap<&str, usize> = BTreeMap::new();
    for row in &report.rows {
        *by_after.entry(row.after.as_str()).or_default() += 1;
        let dispatched = store.history(&row.task_id)?.contains(&Event::Dispatch);
        match row.before {
            Phase::Running => {
                assert_eq!(row.rule, Some(RecoveryRule::R08WorkerAbsent));
                assert_eq!(row.after, Phase::EffectUnknown { cancel: false });
                assert_eq!(row.reason, Some(R08Reason::DispatchUnacknowledged));
                let r08 = Some(AttemptOutcome::R08);
                one_attempt(&store, &row.task_id, AttemptState::Unknown, r08)?;
            }
            Phase::Admitted => assert!(!dispatched && row.rule.is_none(), "{row:?}"),
            other => return Err(format!("unexpected pre-reconcile phase {other:?}").into()),
        }
        assert!(
            row.after.is_terminal()
                || matches!(
                    row.after,
                    Phase::EffectUnknown { .. } | Phase::Blocked { .. }
                )
                || (row.after == Phase::Admitted && !dispatched),
            "{row:?}"
        );
    }
    let integrity = store.integrity_check()?;
    println!(
        "crash: tasks={} acked_tasks={} acked_present={}/{} after={by_after:?} applied={} complete={} integrity_check={integrity}",
        report.rows.len(),
        acked.len(),
        acked
            .keys()
            .filter(|t| ids.iter().any(|i| i.as_str() == t.as_str()))
            .count(),
        acked.len(),
        report.applied,
        report.complete,
    );
    assert_eq!(integrity, "ok");
    assert!(report.complete, "{:?}", report.findings);
    assert!(store.recovery_complete()?);
    assert_eq!(
        reconcile(&store, &Observations::worker_absent())?.applied,
        0,
        "second pass converges"
    );
    let open = store.open_attempts()?.len();
    assert_eq!(open, 0, "no running row survives two passes");
    Ok(())
}
