//! Crash test (acceptance 6): a child process (this test binary, `HEE4_CRASH_CHILD=<db>`)
//! admits and dispatches tasks through the real store in a loop, printing `ACK <task> <phase>`
//! after each `apply` returns; the parent `SIGKILL`s it by pid mid-loop, reopens the file, runs
//! `reconcile` with every worker absent, and checks "acked ⇒ present" plus R08's target.

use std::collections::BTreeMap;
use std::error::Error;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};

use hee4_contracts::{Event, Phase, RecoveryRule, TaskId};
use hee4_core::{Observations, Store, reconcile};

const CHILD_ENV: &str = "HEE4_CRASH_CHILD";
const AFTER_ADMIT_ENV: &str = "HEE4_CRASH_AFTER_ADMIT";
const KILL_AFTER_ACKS: usize = 60;

/// The child half. A no-op unless `HEE4_CRASH_CHILD` names a database.
#[test]
fn crash_child() -> Result<(), Box<dyn Error>> {
    let Some(path) = std::env::var_os(CHILD_ENV) else {
        return Ok(());
    };
    let store = Store::open(&PathBuf::from(path))?;
    reconcile(&store, &Observations::default())?;
    let mut out = std::io::stdout().lock();
    let after_admit = std::env::var_os(AFTER_ADMIT_ENV).is_some_and(|v| v == "1");
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

/// The `admitted` landing: killed right after the first Admit ack, before any Dispatch.
#[test]
fn sigkill_after_admit_before_dispatch() -> Result<(), Box<dyn Error>> {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("hee4-core-crash");
    std::fs::create_dir_all(&dir)?;
    let path = dir.join("crash-admit.sqlite");
    for suffix in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{}{suffix}", path.display()));
    }
    let mut child = Command::new(std::env::current_exe()?)
        .args(["crash_child", "--exact", "--nocapture", "--test-threads=1"])
        .env(CHILD_ENV, &path)
        .env(AFTER_ADMIT_ENV, "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;
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
    println!("crash: mode=after_admit child pid={pid} acked={acked:?} status={status}");
    assert!(!status.success(), "the child must die by signal");
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

#[test]
fn sigkill_mid_loop_then_reconcile() -> Result<(), Box<dyn Error>> {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("hee4-core-crash");
    std::fs::create_dir_all(&dir)?;
    let path = dir.join("crash.sqlite");
    for suffix in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{}{suffix}", path.display()));
    }
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
    Ok(())
}
