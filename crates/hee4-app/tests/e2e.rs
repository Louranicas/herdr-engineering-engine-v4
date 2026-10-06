//! One task end to end through `hee4 serve`, then `kill -9` mid-dispatch and a restart.
//!
//! The fixture VERIFY is `/usr/bin/test -d /usr`: real, silent, inside the sandbox's `RO_BINDS`,
//! and the live proof of the named gap (admission reads the text, not the effect).

use std::error::Error;
use std::fs;
use std::io::{BufRead as _, BufReader, Write as _};
use std::os::unix::fs::PermissionsExt as _;
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use hee4_app::{doctor, socket, wire};
use hee4_contracts::{Budgets, Event, Receipt, Sha256Hex, Verdict};
use serde_json::{Value, json};

type R<T> = Result<T, Box<dyn Error>>;

const BIN: &str = env!("CARGO_BIN_EXE_hee4");
const TERMINAL: [&str; 4] = ["accepted", "failed", "cancelled", "abandoned"];
/// The admitted fixture: real, silent, in the sandbox's read-only binds.
const FIXTURE: &str = "/usr/bin/test -d /usr";

fn brief(verify: &str) -> String {
    format!(
        "GOAL: g\nSCOPE: s\nCONTEXT: c\nACCEPTANCE: a\nVERIFY: {verify}\nTIMEBOX: 60s\nFORBIDDEN: f\n\
         REPORT: r\nSTANDING: s\nRECON: r\nRESTATEMENT: run {verify} in the namespace\n"
    )
}

/// A running `hee4 serve`. Dropping it kills and reaps the child, so a test that returns early
/// or panics never leaves a serve behind; a test killed outright is covered by
/// [`serve_command`]'s parent-death signal.
struct Server {
    child: Child,
    sock: PathBuf,
}

impl Drop for Server {
    fn drop(&mut self) {
        // Already reaped by the test, or already gone: both errors mean nothing is left to stop.
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// `hee4` started through util-linux `setpriv --pdeathsig KILL`, which sets
/// `PR_SET_PDEATHSIG=SIGKILL` and then execs `hee4` in place (same pid): when the test thread
/// that spawned it dies (the harness killed, `timeout -s KILL`), the kernel kills the serve.
/// `pre_exec` would need `unsafe`, which the workspace forbids.
fn serve_command() -> Command {
    let mut cmd = Command::new("setpriv");
    cmd.args(["--pdeathsig", "KILL", "--", BIN]);
    cmd
}

fn start(dir: &Path, log: &str) -> R<Server> {
    start_with(dir, log, false, &[])
}

/// `hee4 serve` over `dir` with `env` set (and `HEE4_BUDGETS` removed unless `env` names it).
fn start_with(dir: &Path, log: &str, live: bool, env: &[(&str, &Path)]) -> R<Server> {
    let sock = dir.join("rt/control.sock");
    let mut cmd = serve_command();
    if live {
        cmd.env("HEE4_LIVE_MODEL", "1");
    } else {
        cmd.env_remove("HEE4_LIVE_MODEL");
    }
    cmd.env_remove("HEE4_BUDGETS");
    cmd.env_remove("HEE4_REQUIRE_BACKUPS");
    for (k, v) in env {
        cmd.env(k, v);
    }
    let child = cmd
        .args(["serve", "--socket"])
        .arg(&sock)
        .arg("--ledger")
        .arg(dir.join("ledger.sqlite3"))
        .arg("--work")
        .arg(dir.join("work"))
        .stdout(Stdio::null())
        .stderr(fs::File::create(dir.join(log))?)
        .spawn()?;
    let t0 = Instant::now();
    while t0.elapsed() < Duration::from_secs(20) {
        if let Ok(v) = call(&sock, "health", None, json!({}))
            && v["body"]["ok"] == true
        {
            return Ok(Server { child, sock });
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    Err("server did not become healthy".into())
}

fn call(sock: &Path, action: &str, key: Option<&str>, body: Value) -> R<Value> {
    Ok(socket::request(
        sock,
        &wire::request("e2e", action, key, body),
    )?)
}

/// Poll `task.get` until `until(phase)`, recording every distinct phase seen.
fn poll(
    sock: &Path,
    id: &Value,
    until: impl Fn(&str) -> bool,
    trace: &mut Vec<String>,
) -> R<Value> {
    let t0 = Instant::now();
    while t0.elapsed() < Duration::from_secs(30) {
        let got = call(sock, "task.get", None, json!({ "task_id": id }))?;
        let phase = got["body"]["phase"].as_str().unwrap_or("?").to_owned();
        if trace.last() != Some(&phase) {
            trace.push(phase.clone());
        }
        if until(&phase) {
            return Ok(got);
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    Err(format!("timed out; trace {trace:?}").into())
}

/// One `attempts` row, read through a read-only connection (SELECT only), as
/// `(receipt_id, workspace, pid, state, closed_seq, outcome)`.
type AttemptCols = (
    Option<String>,
    Option<String>,
    Option<i64>,
    String,
    Option<i64>,
    Option<String>,
);

fn attempt_row(ledger: &Path, id: &str) -> R<Option<AttemptCols>> {
    let conn =
        rusqlite::Connection::open_with_flags(ledger, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let mut stmt = conn.prepare(
        "SELECT receipt_id, workspace, pid, state, closed_seq, outcome FROM attempts WHERE id = ?1",
    )?;
    let mut rows = stmt.query_map([id], |r| {
        Ok((
            r.get(0)?,
            r.get(1)?,
            r.get(2)?,
            r.get(3)?,
            r.get(4)?,
            r.get(5)?,
        ))
    })?;
    Ok(rows.next().transpose()?)
}

/// The ids of `task`'s attempts, generation order (SELECT only).
fn attempt_ids(ledger: &Path, task: &str) -> R<Vec<String>> {
    let conn =
        rusqlite::Connection::open_with_flags(ledger, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let mut stmt =
        conn.prepare("SELECT id FROM attempts WHERE task_id = ?1 ORDER BY generation")?;
    let ids = stmt
        .query_map([task], |r| r.get::<_, String>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(ids)
}

/// Poll the attempt row until the worker's pid is recorded (`on_start` → `attempt_pid`).
fn wait_for_pid(ledger: &Path, id: &str) -> R<i64> {
    let t0 = Instant::now();
    while t0.elapsed() < Duration::from_secs(30) {
        if let Some((_, _, Some(pid), _, _, _)) = attempt_row(ledger, id)? {
            return Ok(pid);
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    Err(format!("{id}: no pid recorded").into())
}

/// The serve log line `recovery task=<task> ...`.
fn recovery_line(log: &str, task: &str) -> Option<String> {
    let prefix = format!("recovery task={task} ");
    log.lines()
        .find(|l| l.starts_with(&prefix))
        .map(str::to_owned)
}

fn receipts(ledger: &Path, task: &str) -> R<Vec<Receipt>> {
    let conn =
        rusqlite::Connection::open_with_flags(ledger, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let mut stmt = conn.prepare("SELECT json FROM receipts WHERE task_id = ?1 ORDER BY seq")?;
    let rows = stmt.query_map([task], |r| r.get::<_, String>(0))?;
    let mut out = Vec::new();
    for json in rows {
        out.push(serde_json::from_str(&json?)?);
    }
    Ok(out)
}

/// The health body carries K1's two readers beside the four skeleton fields: `schema_version`
/// (the migration count, an integer >= 2) and `serve_cgroup` (a non-empty string).
fn assert_health_fields(sock: &Path) -> R<()> {
    let health = call(sock, "health", None, json!({}))?;
    let body = &health["body"];
    assert_eq!(body["ok"], true, "{health}");
    assert_eq!(body["recovery_complete"], true, "{health}");
    assert!(body["head_sha"].is_string(), "{health}");
    assert!(body["uptime_s"].is_u64(), "{health}");
    assert!(
        body["schema_version"].as_i64().is_some_and(|v| v >= 2),
        "{health}"
    );
    assert!(
        body["serve_cgroup"].as_str().is_some_and(|s| !s.is_empty()),
        "{health}"
    );
    let budgets = Budgets::parse(&serde_json::to_string(&body["budgets"])?)?;
    println!("MEASURED health budgets {}", budgets.render());
    println!(
        "MEASURED health schema_version={} serve_cgroup={}",
        body["schema_version"], body["serve_cgroup"]
    );
    Ok(())
}

/// The orphaned bwrap child (FLOW Gaps: it can outlive `kill -9`) is the test's to clean up,
/// by the pid the ledger recorded: R06 is observe-only, so a live one would park the task.
fn kill_orphan(pid: i64) {
    let proc_dir = PathBuf::from(format!("/proc/{pid}"));
    let alive = proc_dir.exists();
    let _ = Command::new("kill")
        .args(["-KILL", &pid.to_string()])
        .status();
    println!("MEASURED task B worker pid={pid} alive_after_sigkill={alive}");
    let t0 = Instant::now();
    while proc_dir.exists() && t0.elapsed() < Duration::from_secs(5) {
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// Task B's generation-1 row after the restart: acknowledged, its workspace
/// `<work>/<b>/1/<b>`, the recorded pid, closed `unknown` by recovery.
fn assert_b_row_closed_unknown(dir: &Path, b_task: &str, pid: i64) -> R<()> {
    let workspace = dir.join("work").join(b_task).join("1").join(b_task);
    let (receipt_id, ws, pid_col, state, closed_seq, _) =
        attempt_row(&dir.join("ledger.sqlite3"), &format!("a-{b_task}-1"))?
            .ok_or("no attempt row for B")?;
    println!(
        "MEASURED attempt a-{b_task}-1 receipt_id={receipt_id:?} workspace={ws:?} pid={pid_col:?} state={state} closed_seq={closed_seq:?}"
    );
    assert!(receipt_id.is_some());
    assert_eq!(ws.as_deref(), workspace.to_str());
    assert_eq!(pid_col, Some(pid));
    assert_eq!(state, "unknown");
    assert!(closed_seq.is_some());
    Ok(())
}

/// `hee4 doctor`'s rows against this serve: `budgets` is present with the default render
/// (the other rows may be MISSING here: no unit, maybe no model).
fn assert_doctor_budgets_row(sock: &Path) -> R<()> {
    let rows = doctor::rows("hee4-e2e-absent.service", sock, Path::new("."));
    let budgets_row = rows
        .iter()
        .find(|r| r.name == "budgets")
        .ok_or("no budgets row")?;
    assert!(budgets_row.present, "{budgets_row:?}");
    assert_eq!(budgets_row.detail, Budgets::DEFAULT.render());
    print!("{}", doctor::render(&rows).0);
    Ok(())
}

/// ACCEPTANCE 14: copy the stopped kill-9 ledger to `<dir>/planted` and plant away `attempt`'s
/// acknowledgement facts and pid (the one write this test makes; the server is stopped).
fn plant_unacknowledged(dir: &Path, attempt: &str) -> R<PathBuf> {
    let planted = dir.join("planted");
    fs::create_dir_all(&planted)?;
    for suffix in ["", "-wal", "-shm"] {
        let from = dir.join(format!("ledger.sqlite3{suffix}"));
        if from.exists() {
            fs::copy(&from, planted.join(format!("ledger.sqlite3{suffix}")))?;
        }
    }
    let conn = rusqlite::Connection::open(planted.join("ledger.sqlite3"))?;
    let n = conn.execute(
        "UPDATE attempts SET receipt_id = NULL, permit_id = NULL, model = NULL, head_sha = NULL,
             workspace = NULL, deadline_ms = NULL, clock_epoch = NULL, pid = NULL,
             pid_start_ticks = NULL
         WHERE id = ?1",
        [attempt],
    )?;
    assert_eq!(n, 1);
    Ok(planted)
}

/// Serve the planted copy: no acknowledgement and no pid is the `DispatchUnacknowledged` class.
fn assert_unacknowledged_class(planted: &Path, task: &str) -> R<()> {
    let mut server = start(planted, "serve-planted.log")?;
    let log = fs::read_to_string(planted.join("serve-planted.log"))?;
    let line = recovery_line(&log, task).ok_or_else(|| format!("no line: {log}"))?;
    println!("serve-planted.log: {line}");
    assert!(line.contains("reason=DispatchUnacknowledged"), "{line}");
    server.child.kill()?;
    server.child.wait()?;
    Ok(())
}

#[test]
fn one_task_end_to_end_then_kill9_mid_dispatch() -> R<()> {
    let version = Command::new(BIN).arg("--version").output()?;
    let version = String::from_utf8(version.stdout)?;
    assert!(version.starts_with("hee4 4.0.0-skeleton "), "{version}");

    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("e2e");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir)?;
    let mut server = start(&dir, "serve-1.log")?;
    let sock = server.sock.clone();
    let dir_mode = fs::metadata(sock.parent().ok_or("no parent")?)?
        .permissions()
        .mode()
        & 0o777;
    let sock_mode = fs::metadata(&sock)?.permissions().mode() & 0o777;
    println!("perms dir={dir_mode:o} sock={sock_mode:o}");
    assert_eq!((dir_mode, sock_mode), (0o700, 0o600));

    assert_health_fields(&sock)?;
    let log1 = fs::read_to_string(dir.join("serve-1.log"))?;
    assert!(log1.contains(" budgets=default"), "{log1}");
    assert_doctor_budgets_row(&sock)?;

    // Task A: the silent fixture VERIFY, no live model.
    let a = call(
        &sock,
        "task.submit",
        Some("key-a"),
        json!({ "brief": brief(FIXTURE) }),
    )?;
    assert_eq!(a["kind"], "result", "{a}");
    let a_id = a["body"]["task_id"].clone();
    let mut trace_a = vec!["admitted".to_owned()];
    let done = poll(&sock, &a_id, |p| TERMINAL.contains(&p), &mut trace_a)?;
    println!(
        "task A {a_id} trace {} events={}",
        trace_a.join(" -> "),
        done["body"]["events"]
    );
    let chain = receipts(&dir.join("ledger.sqlite3"), a_id.as_str().ok_or("id")?)?;
    assert_eq!(chain.len(), 1);
    Receipt::verify_chain(&chain).map_err(|b| format!("{b:?}"))?;
    let last = chain.last().ok_or("empty chain")?;
    assert_eq!(
        done["body"]["last_receipt_hash"],
        last.hash_self().to_string()
    );
    assert_eq!(last.decision().verdict, Verdict::Pass, "{last:?}");
    println!(
        "task A receipt verdict={:?} hash_self={} verify_chain=ok",
        last.decision().verdict,
        last.hash_self()
    );
    let replay = call(
        &sock,
        "task.submit",
        Some("key-a"),
        json!({ "brief": brief(FIXTURE) }),
    )?;
    assert_eq!(replay["replayed"], true);

    // Task B: a long step, killed mid-dispatch once the worker's identity is recorded.
    let b = call(
        &sock,
        "task.submit",
        Some("key-b"),
        json!({ "brief": brief("/usr/bin/sleep 30") }),
    )?;
    let b_id = b["body"]["task_id"].clone();
    let b_task = b_id.as_str().ok_or("id")?.to_owned();
    let b_attempt = format!("a-{b_task}-1");
    let ledger = dir.join("ledger.sqlite3");
    let mut trace_b = vec!["admitted".to_owned()];
    poll(&sock, &b_id, |p| p == "running", &mut trace_b)?;
    let pid = wait_for_pid(&ledger, &b_attempt)?;
    server.child.kill()?; // SIGKILL
    server.child.wait()?;
    trace_b.push("<SIGKILL>".into());
    kill_orphan(pid);
    let planted = plant_unacknowledged(&dir, &b_attempt)?;

    let mut server = start(&dir, "serve-2.log")?;
    let health = call(&server.sock, "health", None, json!({}))?;
    assert_eq!(health["body"]["recovery_complete"], true);
    let after = poll(&server.sock, &b_id, |_| true, &mut trace_b)?;
    let phase = after["body"]["phase"].as_str().unwrap_or("?");
    assert_eq!(phase, "effect_unknown", "{after}");
    println!("task B {b_id} trace {}", trace_b.join(" -> "));
    println!("health after restart {}", health["body"]);
    for log in ["serve-1.log", "serve-2.log"] {
        for line in fs::read_to_string(dir.join(log))?.lines() {
            println!("{log}: {line}");
        }
    }
    let log2 = fs::read_to_string(dir.join("serve-2.log"))?;
    let probe = format!("recovery probe attempt={b_attempt} custody=");
    assert_eq!(log2.matches(&probe).count(), 1, "{log2}");
    let line = recovery_line(&log2, &b_task).ok_or_else(|| format!("no line: {log2}"))?;
    assert!(
        line.contains("rule=R08WorkerAbsent reason=AcknowledgedWorkerLost")
            && line.contains("workspace=NotLeasedWritable"),
        "{line}"
    );
    assert_b_row_closed_unknown(&dir, &b_task, pid)?;
    server.child.kill()?;
    server.child.wait()?;

    assert_unacknowledged_class(&planted, &b_task)?;
    Ok(())
}

/// Send an `events.subscribe {since_seq, epoch}` and return its first frame (ack or refusal)
/// with the reader positioned after it.
fn subscribe_frame(
    sock: &Path,
    since: i64,
    epoch: Option<&str>,
) -> R<(Value, BufReader<UnixStream>)> {
    let mut s = UnixStream::connect(sock)?;
    let frame = wire::request(
        "sub",
        "events.subscribe",
        None,
        json!({ "since_seq": since, "epoch": epoch }),
    );
    s.write_all(format!("{frame}\n").as_bytes())?;
    s.set_read_timeout(Some(Duration::from_secs(30)))?;
    let mut r = BufReader::new(s);
    let mut first = String::new();
    r.read_line(&mut first)?;
    Ok((serde_json::from_str(&first)?, r))
}

/// Open an `events.subscribe` stream after `since`, returning the reader past the ack, which
/// must carry `since_seq`, the ledger `epoch` (string) and `high_water` (u64).
fn subscribe(sock: &Path, since: i64, epoch: Option<&str>) -> R<BufReader<UnixStream>> {
    let (ack, r) = subscribe_frame(sock, since, epoch)?;
    assert_eq!(ack["kind"], "result", "{ack}");
    assert_eq!(ack["body"]["since_seq"], since, "{ack}");
    assert_eq!(ack["body"]["stream"], "events", "{ack}");
    assert!(
        ack["body"]["epoch"].as_str().is_some_and(|e| !e.is_empty()),
        "{ack}"
    );
    assert!(ack["body"]["high_water"].is_u64(), "{ack}");
    if let Some(epoch) = epoch {
        assert_eq!(ack["body"]["epoch"], epoch, "{ack}");
    }
    Ok(r)
}

/// Read event frames until one for `task` reaches a terminal phase.
fn until_terminal(r: &mut BufReader<UnixStream>, task: &Value) -> R<Vec<Value>> {
    let mut out = Vec::new();
    loop {
        let mut line = String::new();
        if r.read_line(&mut line)? == 0 {
            return Err(format!("stream closed after {out:?}").into());
        }
        let v: Value = serde_json::from_str(&line)?;
        assert_eq!(v["kind"], "event", "{v}");
        let done =
            v["task_id"] == *task && TERMINAL.contains(&v["phase_after"].as_str().unwrap_or("?"));
        out.push(v);
        if done {
            return Ok(out);
        }
    }
}

#[test]
fn events_subscribe_streams_in_seq_order_and_resumes_exactly_once() -> R<()> {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("e2e-stream");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir)?;
    let mut server = start(&dir, "serve.log")?;
    let sock = server.sock.clone();
    let mut live = subscribe(&sock, 0, None)?;
    let a = call(
        &sock,
        "task.submit",
        Some("key-s"),
        json!({ "brief": brief(FIXTURE) }),
    )?;
    let id = a["body"]["task_id"].clone();
    let first = until_terminal(&mut live, &id)?;
    for f in &first {
        println!("stream {f}");
    }
    assert_eq!(first[0]["event"], "admit");
    assert_eq!(first[0]["phase_after"], "admitted");
    let seqs: Vec<i64> = first.iter().filter_map(|f| f["seq"].as_i64()).collect();
    assert_eq!(seqs.len(), first.len());
    assert!(seqs.windows(2).all(|w| w[0] < w[1]), "{seqs:?}");
    assert!(first.len() >= 3, "{first:?}");

    // Resume after the second event: exactly the rest, once each, then nothing.
    let mut resumed = subscribe(&sock, seqs[1], None)?;
    let rest = until_terminal(&mut resumed, &id)?;
    assert_eq!(rest, first[2..].to_vec());
    resumed
        .get_ref()
        .set_read_timeout(Some(Duration::from_millis(500)))?;
    let mut extra = String::new();
    assert!(
        resumed.read_line(&mut extra).is_err(),
        "extra frame {extra}"
    );
    println!(
        "resume since_seq={} replayed={} frames, no extra",
        seqs[1],
        rest.len()
    );
    server.child.kill()?;
    server.child.wait()?;
    Ok(())
}

#[test]
fn an_oversize_line_is_refused_by_name_then_closed() -> R<()> {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("e2e-oversize");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir)?;
    let mut server = start(&dir, "serve.log")?;
    let mut s = UnixStream::connect(&server.sock)?;
    s.set_read_timeout(Some(Duration::from_secs(10)))?;
    let junk = vec![b'a'; usize::try_from(Budgets::DEFAULT.socket.frame_bytes)? + 2];
    s.write_all(&junk)?;
    let mut r = BufReader::new(s);
    let mut line = String::new();
    r.read_line(&mut line)?;
    let v: Value = serde_json::from_str(&line)?;
    assert_eq!(
        (v["kind"].as_str(), v["code"].as_str()),
        (Some("error"), Some("frame_too_large")),
        "{v}"
    );
    line.clear();
    assert_eq!(r.read_line(&mut line)?, 0, "closed after the refusal");
    assert_eq!(
        call(&server.sock, "health", None, json!({}))?["body"]["ok"],
        true
    );
    println!("MEASURED oversize line -> {v}");
    server.child.kill()?;
    server.child.wait()?;
    Ok(())
}

#[test]
fn the_connection_after_the_cap_is_refused_by_name_and_health_still_answers() -> R<()> {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("e2e-cap");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir)?;
    let file = dir.join("budgets.json");
    fs::write(&file, r#"{"socket":{"max_connections":2}}"#)?;
    let mut server = start_with(&dir, "serve.log", false, &[("HEE4_BUDGETS", &file)])?;
    let log = fs::read_to_string(dir.join("serve.log"))?;
    assert!(
        log.contains(&format!(" budgets=file:{}", file.display())),
        "{log}"
    );
    // Hold exactly `max_connections` connections that the server has ADMITTED: each one answers a
    // health round trip and stays open. `connect` alone proved nothing: the serve-start health
    // polls can still hold a slot for a moment, so a held connection could be the refused one and
    // the lingering slot then free just before `extra`, which was admitted and read until its
    // timeout (WouldBlock under load in the cut tier, 2026-10-05). A held connection refused
    // while a lingering slot drains is dropped and retried, within a bound.
    let admitted = |sock: &Path| -> R<Option<UnixStream>> {
        let mut c = UnixStream::connect(sock)?;
        c.set_read_timeout(Some(Duration::from_secs(10)))?;
        c.write_all(format!("{}\n", wire::request("hold", "health", None, json!({}))).as_bytes())?;
        let mut line = String::new();
        BufReader::new(&c).read_line(&mut line)?;
        let v: Value = serde_json::from_str(&line)?;
        if v["code"] == "too_many_connections" {
            return Ok(None);
        }
        assert_eq!(v["body"]["ok"], true, "{v}");
        Ok(Some(c))
    };
    let mut held = Vec::new();
    let t0 = Instant::now();
    while held.len() < 2 {
        match admitted(&server.sock)? {
            Some(c) => held.push(c),
            None if t0.elapsed() < Duration::from_secs(5) => {
                std::thread::sleep(Duration::from_millis(50));
            }
            None => {
                return Err("a held connection was refused for 5 s: a slot never drained".into());
            }
        }
    }
    let extra = UnixStream::connect(&server.sock)?;
    extra.set_read_timeout(Some(Duration::from_secs(10)))?;
    let mut line = String::new();
    BufReader::new(&extra).read_line(&mut line)?;
    let v: Value = serde_json::from_str(&line)?;
    assert_eq!(v["code"], "too_many_connections", "{v}");
    assert!(
        v["message"]
            .as_str()
            .is_some_and(|m| m.contains("2 connections are open")),
        "{v}"
    );
    println!("MEASURED connection 3 -> {v}");
    // An admitted connection is unharmed: health answers on it.
    let mut first = held.remove(0);
    first.set_read_timeout(Some(Duration::from_secs(10)))?;
    first.write_all(format!("{}\n", wire::request("cap", "health", None, json!({}))).as_bytes())?;
    let mut line = String::new();
    BufReader::new(&first).read_line(&mut line)?;
    let h: Value = serde_json::from_str(&line)?;
    assert_eq!(h["body"]["ok"], true, "{h}");
    assert_eq!(h["body"]["budgets"]["socket"]["max_connections"], 2, "{h}");
    // Closing them frees the slots.
    drop(first);
    drop(held);
    std::thread::sleep(Duration::from_millis(300));
    assert_eq!(
        call(&server.sock, "health", None, json!({}))?["body"]["ok"],
        true
    );
    server.child.kill()?;
    server.child.wait()?;
    Ok(())
}

/// A budgets file with a zero field: `hee4 serve` exits non-zero naming the field, by the
/// env and by the flag, before the ledger is opened or the socket bound.
#[test]
fn a_budgets_refusal_names_the_field_and_serve_does_not_listen() -> R<()> {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("e2e-budgets-refused");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir)?;
    let file = dir.join("b.json");
    fs::write(&file, r#"{"door":{"max_body_bytes":0}}"#)?;
    for by_flag in [false, true] {
        let mut cmd = serve_command();
        cmd.env_remove("HEE4_BUDGETS")
            .args(["serve", "--socket"])
            .arg(dir.join("rt/control.sock"))
            .arg("--ledger")
            .arg(dir.join("ledger.sqlite3"))
            .arg("--work")
            .arg(dir.join("work"));
        if by_flag {
            cmd.arg("--budgets").arg(&file);
        } else {
            cmd.env("HEE4_BUDGETS", &file);
        }
        let out = cmd.output()?;
        let stderr = String::from_utf8(out.stderr)?;
        println!(
            "MEASURED by_flag={by_flag} status={} stderr={stderr}",
            out.status
        );
        assert!(!out.status.success(), "{stderr}");
        assert!(stderr.contains("hee4 serve refused: budgets:"), "{stderr}");
        assert!(stderr.contains("max_body_bytes"), "{stderr}");
        assert!(!dir.join("rt/control.sock").exists());
        assert!(!dir.join("ledger.sqlite3").exists());
    }
    Ok(())
}

/// `attempt.deadline_ms` bounds every step: under a 500 ms deadline a brief whose TIMEBOX is
/// 60 s runs `/usr/bin/sleep 3`, the step is killed at the deadline, and the task ends `failed`
/// with its attempt settled `not_ready`, well before the sleep would have finished.
#[test]
fn a_step_that_outlives_attempt_deadline_ms_is_killed_at_the_deadline() -> R<()> {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("e2e-deadline");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir)?;
    let file = dir.join("budgets.json");
    fs::write(&file, r#"{"attempt":{"deadline_ms":500}}"#)?;
    let mut server = start_with(&dir, "serve.log", false, &[("HEE4_BUDGETS", &file)])?;
    let t0 = Instant::now();
    let t = call(
        &server.sock,
        "task.submit",
        Some("key-deadline"),
        json!({ "brief": brief("/usr/bin/sleep 3") }),
    )?;
    assert_eq!(t["kind"], "result", "{t}");
    let id = t["body"]["task_id"].clone();
    let task = id.as_str().ok_or("id")?.to_owned();
    let mut trace = vec!["admitted".to_owned()];
    let done = poll(&server.sock, &id, |p| TERMINAL.contains(&p), &mut trace)?;
    let took = t0.elapsed();
    let log = fs::read_to_string(dir.join("serve.log"))?;
    println!(
        "MEASURED deadline task {task} trace {} took={took:?}",
        trace.join(" -> ")
    );
    assert_eq!(done["body"]["phase"], "failed", "{done}\n{log}");
    assert!(took < Duration::from_millis(2500), "took {took:?}\n{log}");
    let (_, _, _, state, _, outcome) =
        attempt_row(&dir.join("ledger.sqlite3"), &format!("a-{task}-1"))?.ok_or("no row")?;
    assert_eq!(
        (state.as_str(), outcome.as_deref()),
        ("settled", Some("not_ready")),
        "{log}"
    );
    server.child.kill()?;
    server.child.wait()?;
    Ok(())
}

/// `task`'s events in seq order (SELECT only).
fn events_of(ledger: &Path, task: &str) -> R<Vec<Event>> {
    let conn =
        rusqlite::Connection::open_with_flags(ledger, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let mut stmt = conn.prepare("SELECT event_json FROM events WHERE task_id = ?1 ORDER BY seq")?;
    let rows = stmt.query_map([task], |r| r.get::<_, String>(0))?;
    let mut out = Vec::new();
    for json in rows {
        out.push(serde_json::from_str(&json?)?);
    }
    Ok(out)
}

/// A task cancelled before its dispatch reaches `cancelled` with events admit, cancel, stop and
/// no attempt. The dispatcher is held on a blocker task (`sleep 3`) while the second task is
/// submitted and cancelled, so the cancel lands before any pick of it.
#[test]
fn a_cancel_before_dispatch_reaches_cancelled() -> R<()> {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("e2e-cbd");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir)?;
    let mut server = start(&dir, "serve.log")?;
    let blocker = call(
        &server.sock,
        "task.submit",
        Some("key-blocker"),
        json!({ "brief": brief("/usr/bin/sleep 3") }),
    )?;
    assert_eq!(blocker["kind"], "result", "{blocker}");
    let blocker_id = blocker["body"]["task_id"].clone();
    poll(
        &server.sock,
        &blocker_id,
        |p| p == "running",
        &mut Vec::new(),
    )?;
    let t = call(
        &server.sock,
        "task.submit",
        Some("key-cancelled"),
        json!({ "brief": brief(FIXTURE) }),
    )?;
    assert_eq!(t["kind"], "result", "{t}");
    let id = t["body"]["task_id"].clone();
    let task = id.as_str().ok_or("id")?.to_owned();
    let c = call(
        &server.sock,
        "task.cancel",
        Some("key-cancel"),
        json!({ "task_id": id }),
    )?;
    assert_eq!(c["body"]["phase"], "cancellation_requested", "{c}");
    let mut trace = vec!["admitted".to_owned()];
    let done = poll(&server.sock, &id, |p| TERMINAL.contains(&p), &mut trace)?;
    let log = fs::read_to_string(dir.join("serve.log"))?;
    println!(
        "MEASURED cancel-before-dispatch task {task} trace {}",
        trace.join(" -> ")
    );
    assert_eq!(done["body"]["phase"], "cancelled", "{done}\n{log}");
    let ledger = dir.join("ledger.sqlite3");
    assert_eq!(
        events_of(&ledger, &task)?,
        [Event::Admit, Event::Cancel, Event::Stop],
        "{log}"
    );
    assert!(attempt_ids(&ledger, &task)?.is_empty(), "{log}");
    server.child.kill()?;
    server.child.wait()?;
    Ok(())
}

/// `true` when the contracts' attempt budget carries `max_generations` (the repair leg's bound).
fn max_generations_present() -> R<bool> {
    let attempt = serde_json::to_value(Budgets::DEFAULT)?["attempt"].clone();
    Ok(attempt.get("max_generations").is_some())
}

/// D6 repair leg. `attempt.max_generations` is absent from K0's `AttemptBudget`, so no repair
/// is dispatched: this test measures today's parking (generation 1 in `<work>/<t>/1/<t>`,
/// settled `not_ready`, `failed`, no receipt) and prints the named gap. It fails once the field
/// lands, so the redispatch assertions are written then.
#[test]
fn a_failed_step_is_redispatched_in_a_fresh_generation_dir() -> R<()> {
    if max_generations_present()? {
        return Err("attempt.max_generations landed: write the redispatch assertions".into());
    }
    println!("UNMEASURED: attempt.max_generations absent");
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("e2e-repair");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir)?;
    let mut server = start(&dir, "serve.log")?;
    let verify = r#"sh: /usr/bin/test "$(/usr/bin/basename "$(/usr/bin/dirname "$(/usr/bin/pwd -P)")")" = 2"#;
    let t = call(
        &server.sock,
        "task.submit",
        Some("key-repair"),
        json!({ "brief": brief(verify) }),
    )?;
    assert_eq!(t["kind"], "result", "{t}");
    let id = t["body"]["task_id"].clone();
    let task = id.as_str().ok_or("id")?.to_owned();
    let mut trace = vec!["admitted".to_owned()];
    let done = poll(&server.sock, &id, |p| TERMINAL.contains(&p), &mut trace)?;
    println!("repair task {task} trace {}", trace.join(" -> "));
    assert_eq!(done["body"]["phase"], "failed", "{done}");
    let ledger = dir.join("ledger.sqlite3");
    assert_eq!(attempt_ids(&ledger, &task)?, vec![format!("a-{task}-1")]);
    let (_, ws, _, state, _, outcome) =
        attempt_row(&ledger, &format!("a-{task}-1"))?.ok_or("no row")?;
    let gen1 = dir.join("work").join(&task).join("1").join(&task);
    assert_eq!(ws.as_deref(), gen1.to_str());
    assert!(gen1.is_dir());
    assert_eq!(
        (state.as_str(), outcome.as_deref()),
        ("settled", Some("not_ready"))
    );
    assert_eq!(receipts(&ledger, &task)?.len(), 0);
    server.child.kill()?;
    server.child.wait()?;
    Ok(())
}

/// D6 bound: a repair that never passes ends `failed` after `attempt.max_generations`
/// dispatches. Gated on the same absent field.
#[test]
fn a_repair_that_never_passes_stops_after_max_generations() -> R<()> {
    if max_generations_present()? {
        return Err("attempt.max_generations landed: write the exhaustion assertions".into());
    }
    println!("UNMEASURED: attempt.max_generations absent");
    Ok(())
}

/// A fresh test directory on tmpfs when `/dev/shm` is writable, else under
/// `CARGO_TARGET_TMPDIR`. The slow-consumer proof needs ~300 admitted tasks (every submit and
/// every dispatcher step is one `synchronous=FULL` transaction plus a brief `fsync`): measured
/// 33 ms per submit on this machine's `/home` (dm-crypt) and 0.3 ms on tmpfs, with or without a
/// subscriber, so the disk is the test's clock and not what it proves.
///
/// `/dev/shm` is machine-global, so the root is unique per run (package, pid, nanosecond
/// stamp): concurrent slices or cargo invocations never share, delete or `pkill -f` each
/// other's directories. The guard removes the root when the test ends, pass or panic; a test
/// binary killed outright (SIGKILL) runs no guard, so each new run first sweeps the roots of
/// this package's dead runs ([`sweep_dead_runs`]).
struct RunDir(PathBuf);

impl Drop for RunDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// Remove each `<parent>/hee4-e2e-<package>-<pid>-<stamp>` directory whose `<pid>` has no
/// `/proc` entry and whose owner is this process's uid: the leftovers of a killed run of this
/// package. A live pid's directory, another package's, another user's, a symlink or a name
/// that does not parse is never touched. Returns the directories removed.
fn sweep_dead_runs(parent: &Path, package: &str) -> R<Vec<PathBuf>> {
    use std::os::unix::fs::MetadataExt as _;
    let uid = fs::metadata("/proc/self")?.uid();
    let prefix = format!("hee4-e2e-{package}-");
    let mut removed = Vec::new();
    let Ok(entries) = fs::read_dir(parent) else {
        return Ok(removed);
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(rest) = name.to_str().and_then(|n| n.strip_prefix(&prefix)) else {
            continue;
        };
        let Some((pid, stamp)) = rest.split_once('-') else {
            continue;
        };
        let digits = |t: &str| !t.is_empty() && t.bytes().all(|b| b.is_ascii_digit());
        if !digits(pid) || !digits(stamp) || Path::new("/proc").join(pid).exists() {
            continue;
        }
        let path = entry.path();
        let Ok(meta) = fs::symlink_metadata(&path) else {
            continue;
        };
        if meta.file_type().is_dir() && meta.uid() == uid && fs::remove_dir_all(&path).is_ok() {
            removed.push(path);
        }
    }
    Ok(removed)
}

fn fsync_cheap_dir(name: &str) -> R<(RunDir, PathBuf)> {
    for gone in sweep_dead_runs(Path::new("/dev/shm"), env!("CARGO_PKG_NAME"))? {
        println!("removed a dead run's directory: {}", gone.display());
    }
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_nanos();
    let run = format!(
        "hee4-e2e-{}-{}-{nanos}",
        env!("CARGO_PKG_NAME"),
        std::process::id()
    );
    let shm = PathBuf::from("/dev/shm").join(&run);
    let root = if fs::create_dir(&shm).is_ok() {
        shm
    } else {
        let tmp = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(&run);
        fs::create_dir_all(&tmp)?;
        tmp
    };
    let dir = root.join(name);
    fs::create_dir_all(&dir)?;
    Ok((RunDir(root), dir))
}

/// A killed run's `/dev/shm` root is removed by the next run's fresh-directory helper; a live
/// pid's root (this test's own pid) and another package's root are kept.
#[test]
fn the_fresh_dir_helper_removes_only_this_packages_dead_runs() -> R<()> {
    let shm = Path::new("/dev/shm");
    if !shm.is_dir() {
        println!("UNMEASURED: no /dev/shm");
        return Ok(());
    }
    // A pid that was ours and is now reaped: no `/proc` entry.
    let mut child = Command::new("/usr/bin/true").spawn()?;
    let dead = child.id();
    child.wait()?;
    if Path::new("/proc").join(dead.to_string()).exists() {
        return Err(format!("pid {dead} was reused before the test could plant it").into());
    }
    let pkg = env!("CARGO_PKG_NAME");
    let dead_run = shm.join(format!("hee4-e2e-{pkg}-{dead}-1"));
    let live_run = shm.join(format!("hee4-e2e-{pkg}-{}-1", std::process::id()));
    let other_run = shm.join(format!("hee4-e2e-other-{dead}-1"));
    for d in [&dead_run, &live_run, &other_run] {
        fs::create_dir_all(d.join("inner"))?;
    }
    let made = fsync_cheap_dir("sweep");
    let kept = (live_run.is_dir(), other_run.is_dir());
    let swept = !dead_run.exists();
    for d in [&dead_run, &live_run, &other_run] {
        let _ = fs::remove_dir_all(d);
    }
    drop(made?);
    assert!(swept, "{} was left behind", dead_run.display());
    assert_eq!(kept, (true, true), "a live or foreign run root was removed");
    Ok(())
}

/// The server's `slow_consumer` log line: the reader dropped the subscriber, or the close frame
/// missed its deadline (`log_line` below is the second, the FLOW's "not delivered").
fn log_says(log: &Path, needle: &str) -> bool {
    fs::read_to_string(log).is_ok_and(|s| s.contains(needle))
}

fn is_close_frame(line: &str) -> bool {
    line.contains("\"kind\":\"close\"") && line.contains("slow_consumer")
}

/// The cut tier's defect, end to end through `read_ledger` on a real `hee4 serve`: with a
/// 16-frame queue, a subscriber that reads every frame but more slowly than the server writes
/// them replays the whole ledger from `since_seq` 0 to `high_water`, in `seq` order, with no
/// `slow_consumer`. The ledger is seeded past 3000 events because the server's writer first drains
/// the queue into the kernel socket buffer (hundreds of KB): only once that buffer is full does
/// the queue stay full, which is where the drive was cut off (172-525 of ~800 frames, 2026-10-06).
/// Under the old instant-drop rule this test fails.
#[test]
fn a_slow_reader_replays_more_than_three_queues_through_a_real_serve() -> R<()> {
    let (_run, dir) = fsync_cheap_dir("e2e-slow-replay")?;
    let budgets = dir.join("budgets.json");
    fs::write(
        &budgets,
        r#"{"stream":{"queue_frames":16,"batch_rows":16}}"#,
    )?;
    let mut server = start_with(&dir, "serve.log", false, &[("HEE4_BUDGETS", &budgets)])?;
    let mut c = UnixStream::connect(&server.sock)?;
    c.set_read_timeout(Some(Duration::from_secs(30)))?;
    let mut cr = BufReader::new(c.try_clone()?);
    let mut hw = 0;
    let mut submits = 0;
    // Enough events to fill the kernel socket buffer and then the 16-frame queue many times over.
    while hw <= 3000 && submits < 6000 {
        let f = wire::request(
            "replay",
            "task.submit",
            Some(&format!("replay-{submits}")),
            json!({ "brief": brief(FIXTURE) }),
        );
        c.write_all(format!("{f}\n").as_bytes())?;
        let mut l = String::new();
        cr.read_line(&mut l)?;
        submits += 1;
        if submits % 500 == 0 {
            let (ack, _) = subscribe_frame(&server.sock, 0, None)?;
            hw = ack["body"]["high_water"].as_i64().unwrap_or(0);
        }
    }
    assert!(hw > 3000, "only {hw} events after {submits} submits");
    let (ack, mut sub) = subscribe_frame(&server.sock, 0, None)?;
    let high_water = ack["body"]["high_water"].as_i64().ok_or("no high_water")?;
    let mut seqs = Vec::new();
    while seqs.last().copied().unwrap_or(0) < high_water {
        let mut l = String::new();
        if sub.read_line(&mut l)? == 0 {
            break;
        }
        assert!(
            !is_close_frame(&l),
            "dropped as slow_consumer after {} frames: {l}",
            seqs.len()
        );
        let v: Value = serde_json::from_str(&l)?;
        seqs.push(v["seq"].as_i64().ok_or("frame without seq")?);
        std::thread::sleep(Duration::from_micros(200)); // slower than the server writes
    }
    println!(
        "MEASURED replay frames={} high_water={high_water} submits={submits}",
        seqs.len()
    );
    assert_eq!(
        seqs.last().copied(),
        Some(high_water),
        "the replay stopped early"
    );
    assert!(seqs.windows(2).all(|w| w[0] < w[1]), "seq not ascending");
    server.child.kill()?;
    server.child.wait()?;
    Ok(())
}

/// A subscriber that never reads is a slow consumer against a real `hee4 serve`: its queue
/// accepts no frame for `stream.stall_ms` (set to 500 ms here through `HEE4_BUDGETS`, so the test
/// stays fast). The submit loop never touches the subscriber; it stops at the server's log line
/// or at 3000 submits, and the bounded drain after it then finds the close frame or EOF. (Before
/// 2026-10-06 this test read one line per 64 submits, which the instant-drop rule also dropped;
/// under the stall rule a draining reader is never dropped, so the test now truly never reads.)
#[test]
fn a_subscriber_that_never_reads_gets_the_close_frame_or_the_log_line() -> R<()> {
    let (_run, dir) = fsync_cheap_dir("e2e-slow")?;
    let budgets = dir.join("budgets.json");
    fs::write(&budgets, r#"{"stream":{"stall_ms":500}}"#)?;
    let mut server = start_with(&dir, "serve.log", false, &[("HEE4_BUDGETS", &budgets)])?;
    let log = dir.join("serve.log");
    let mut sub = subscribe(&server.sock, 0, None)?;
    sub.get_ref()
        .set_read_timeout(Some(Duration::from_secs(1)))?;
    // One connection, many submits: enough events to fill the queue and the socket buffer.
    let mut c = UnixStream::connect(&server.sock)?;
    c.set_read_timeout(Some(Duration::from_secs(30)))?;
    let mut cr = BufReader::new(c.try_clone()?);
    let t_submit = Instant::now();
    let mut submits = 0;
    let mut delivered = false;
    let mut signal = None;
    while signal.is_none() && submits < 3000 {
        for _ in 0..64 {
            let f = wire::request(
                "slow",
                "task.submit",
                Some(&format!("slow-{submits}")),
                json!({ "brief": brief(FIXTURE) }),
            );
            c.write_all(format!("{f}\n").as_bytes())?;
            let mut l = String::new();
            cr.read_line(&mut l)?;
            submits += 1;
        }
        if log_says(&log, "slow_consumer") {
            signal = Some("log");
        }
    }
    // Bounded wait: drain the subscriber to the close frame or EOF, re-reading the log.
    let t0 = Instant::now();
    let mut logged = log_says(&log, "slow_consumer close frame not delivered");
    while t0.elapsed() < Duration::from_secs(10) && !(delivered || logged) {
        let mut l = String::new();
        match sub.read_line(&mut l) {
            Ok(0) => {
                logged = log_says(&log, "slow_consumer close frame not delivered");
                break;
            }
            Ok(_) => delivered |= is_close_frame(&l),
            Err(_) => logged = log_says(&log, "slow_consumer close frame not delivered"),
        }
    }
    println!(
        "MEASURED close_frame_delivered={delivered} log_line={logged} submits={submits} signal={} submit_ms={} drain_ms={}",
        signal.unwrap_or("none"),
        t_submit.elapsed().as_millis(),
        t0.elapsed().as_millis()
    );
    assert!(
        delivered || logged,
        "neither the close frame nor the log line"
    );
    server.child.kill()?;
    server.child.wait()?;
    // Sandboxed children outlive the killed server; the pattern is this run's unique root, so
    // it can match no other run's processes. `_run` removes the root on return.
    let work = dir.join("work").to_string_lossy().into_owned();
    let _ = Command::new("pkill").args(["-KILL", "-f", &work]).status();
    Ok(())
}

/// Over the real socket: a vacuous VERIFY is refused at admission by name (`invalid_argument`
/// at `/body/brief`, message naming VERIFY), nothing is admitted, and the silent fixture then
/// runs to `accepted` with a `Pass` receipt.
#[test]
fn vacuous_verify_is_refused_at_admission() -> R<()> {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("e2e-vacuous");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir)?;
    let mut server = start(&dir, "serve.log")?;
    let sock = server.sock.clone();
    for (i, verify) in ["sh: true", "/usr/bin/true", "model: hi"]
        .iter()
        .enumerate()
    {
        let refused = call(
            &sock,
            "task.submit",
            Some(&format!("vacuous-{i}")),
            json!({ "brief": brief(verify) }),
        )?;
        println!("MEASURED vacuous_verify submit {verify:?} -> {refused}");
        assert_eq!(refused["kind"], "error", "{refused}");
        assert_eq!(refused["code"], "invalid_argument", "{refused}");
        assert_eq!(refused["retry"], "never", "{refused}");
        assert_eq!(refused["field"], "/body/brief", "{refused}");
        assert!(
            refused["message"]
                .as_str()
                .is_some_and(|m| m.contains("VERIFY")),
            "{refused}"
        );
        let preview = call(
            &sock,
            "task.preview",
            None,
            json!({ "brief": brief(verify) }),
        )?;
        println!("MEASURED vacuous_verify preview {verify:?} -> {preview}");
        assert_eq!(preview["body"]["eligible"], false, "{preview}");
        assert_eq!(preview["body"]["refusal"], "invalid_argument", "{preview}");
        assert!(
            preview["body"]["message"]
                .as_str()
                .is_some_and(|m| m.contains("VERIFY")),
            "{preview}"
        );
    }
    let listed = call(&sock, "task.list", None, json!({}))?;
    assert_eq!(listed["body"]["tasks"], json!([]), "{listed}");

    let ok = call(
        &sock,
        "task.submit",
        Some("key-real"),
        json!({ "brief": brief(FIXTURE) }),
    )?;
    assert_eq!(ok["kind"], "result", "{ok}");
    let id = ok["body"]["task_id"].clone();
    let mut trace = vec!["admitted".to_owned()];
    let done = poll(&sock, &id, |p| TERMINAL.contains(&p), &mut trace)?;
    assert_eq!(done["body"]["phase"], "accepted", "{done}; trace {trace:?}");
    let chain = receipts(&dir.join("ledger.sqlite3"), id.as_str().ok_or("id")?)?;
    assert_eq!(chain.len(), 1);
    Receipt::verify_chain(&chain).map_err(|b| format!("{b:?}"))?;
    let last = chain.last().ok_or("empty chain")?;
    assert_eq!(last.decision().verdict, Verdict::Pass, "{last:?}");
    println!(
        "task receipt verdict={:?} hash_self={} verify_chain=ok",
        last.decision().verdict,
        last.hash_self()
    );
    server.child.kill()?;
    server.child.wait()?;
    Ok(())
}

/// R13 over the real socket: each stale cursor is `resync_required` named by its verdict, the
/// boundary cursor is a snapshot, a restored-from epoch is refused after a restart, and a valid
/// epoch cursor streams exactly what the legacy cursor streams.
#[test]
fn events_subscribe_epoch_cursor_refuses_resync_by_name() -> R<()> {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("e2e-cursor");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir)?;
    let mut server = start(&dir, "serve.log")?;
    let sock = server.sock.clone();
    let a = call(
        &sock,
        "task.submit",
        Some("key-c"),
        json!({ "brief": brief(FIXTURE) }),
    )?;
    let id = a["body"]["task_id"].clone();
    let mut trace = vec!["admitted".to_owned()];
    poll(&sock, &id, |p| TERMINAL.contains(&p), &mut trace)?;

    // The ledger's own cursor, from a legacy (epoch null) ack.
    let (ack, _r) = subscribe_frame(&sock, 0, None)?;
    assert_eq!(ack["kind"], "result", "{ack}");
    let epoch = ack["body"]["epoch"].as_str().ok_or("ack epoch")?.to_owned();
    let high_water = ack["body"]["high_water"].as_u64().ok_or("ack high_water")?;
    assert!(high_water >= 3, "{ack}");

    // (a) a foreign epoch.
    let (foreign, _r) = subscribe_frame(&sock, 0, Some("not-this-ledger"))?;
    println!("MEASURED resync case=epoch_changed -> {foreign}");
    assert_eq!(foreign["kind"], "error", "{foreign}");
    assert_eq!(foreign["code"], "resync_required", "{foreign}");
    assert_eq!(foreign["field"], "/body/epoch", "{foreign}");
    assert_eq!(foreign["because"], "epoch_changed", "{foreign}");
    assert_eq!(foreign["retry"], "never", "{foreign}");

    // (b) the right epoch, a sequence past the high water.
    let future_seq = i64::try_from(high_water + 1000)?;
    let (future, _r) = subscribe_frame(&sock, future_seq, Some(&epoch))?;
    println!("MEASURED resync case=future_sequence -> {future}");
    assert_eq!(future["code"], "resync_required", "{future}");
    assert_eq!(future["field"], "/body/since_seq", "{future}");
    assert_eq!(future["because"], "future_sequence", "{future}");

    // (c) the boundary: since_seq == high_water with the right epoch is a snapshot (ack).
    let boundary_seq = i64::try_from(high_water)?;
    let (boundary, _r) = subscribe_frame(&sock, boundary_seq, Some(&epoch))?;
    println!("MEASURED resync case=snapshot_only -> {boundary}");
    assert_eq!(boundary["kind"], "result", "{boundary}");
    assert_eq!(boundary["body"]["epoch"], epoch, "{boundary}");
    assert_eq!(boundary["body"]["high_water"], high_water, "{boundary}");

    // (e) a valid epoch cursor streams exactly the frames the legacy cursor streams.
    let mut by_epoch = subscribe(&sock, 0, Some(&epoch))?;
    let with_epoch = until_terminal(&mut by_epoch, &id)?;
    let mut legacy = subscribe(&sock, 0, None)?;
    let without = until_terminal(&mut legacy, &id)?;
    assert_eq!(with_epoch, without);
    assert!(with_epoch.len() >= 3, "{with_epoch:?}");
    println!(
        "MEASURED resync case=valid_cursor_streams -> {} frames, same as the legacy cursor",
        with_epoch.len()
    );
    drop(by_epoch);
    drop(legacy);

    // (d) restart over a ledger that records this epoch as the one it was restored from.
    server.child.kill()?;
    server.child.wait()?;
    {
        let store = hee4_core::Store::open(&dir.join("ledger.sqlite3"))?;
        store.mark_restored_from(&epoch)?;
    }
    let mut server = start(&dir, "serve-2.log")?;
    let (prior, _r) = subscribe_frame(&server.sock, 0, Some(&epoch))?;
    println!("MEASURED resync case=prior_epoch_of_restore -> {prior}");
    assert_eq!(prior["code"], "resync_required", "{prior}");
    assert_eq!(prior["field"], "/body/epoch", "{prior}");
    assert_eq!(prior["because"], "prior_epoch_of_restore", "{prior}");
    server.child.kill()?;
    server.child.wait()?;
    let work = dir.join("work").to_string_lossy().into_owned();
    let _ = Command::new("pkill").args(["-KILL", "-f", &work]).status();
    Ok(())
}

/// The first real model attempt through the door: a `sh:` step curls `$HEE4_MODEL_SOCKET`.
/// Skipped (UNMEASURED) unless `HEE4_LIVE_MODEL=1` and the model answers on loopback.
#[test]
fn live_model_attempt_through_the_door() -> R<()> {
    let answers = || -> bool {
        Command::new("/usr/bin/curl")
            .args(["-sS", "-m", "3", "http://127.0.0.1:11434/api/tags"])
            .output()
            .is_ok_and(|o| o.status.success() && !o.stdout.is_empty())
    };
    if std::env::var("HEE4_LIVE_MODEL").as_deref() != Ok("1") || !answers() {
        println!(
            "UNMEASURED: live model attempt skipped (HEE4_LIVE_MODEL!=1 or no model on 11434)"
        );
        return Ok(());
    }
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("e2e-live");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir)?;
    let mut server = start_with(&dir, "serve-live.log", true, &[])?;
    let verify = r#"sh: /usr/bin/curl -sS -m 60 --unix-socket "$HEE4_MODEL_SOCKET" http://model/api/generate -d '{"model":"qwen2.5:0.5b","prompt":"Say exactly: hee4 ok","stream":false}'"#;
    let sub = call(
        &server.sock,
        "task.submit",
        Some("key-live"),
        json!({ "brief": brief(verify) }),
    )?;
    assert_eq!(sub["kind"], "result", "{sub}");
    let id = sub["body"]["task_id"].clone();
    let mut trace = vec!["admitted".to_owned()];
    let t0 = Instant::now();
    let mut done = None;
    while t0.elapsed() < Duration::from_secs(120) {
        let got = call(&server.sock, "task.get", None, json!({ "task_id": id }))?;
        let phase = got["body"]["phase"].as_str().unwrap_or("?").to_owned();
        if trace.last() != Some(&phase) {
            trace.push(phase.clone());
        }
        if TERMINAL.contains(&phase.as_str()) {
            done = Some(got);
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    println!("live task {id} trace {}", trace.join(" -> "));
    for line in fs::read_to_string(dir.join("serve-live.log"))?.lines() {
        println!("serve-live.log: {line}");
    }
    let done = done.ok_or("live task never reached a terminal phase")?;
    let chain = receipts(&dir.join("ledger.sqlite3"), id.as_str().ok_or("id")?)?;
    server.child.kill()?;
    server.child.wait()?;
    assert_eq!(done["body"]["phase"], "accepted", "{done}");
    assert_eq!(chain.len(), 1);
    Receipt::verify_chain(&chain).map_err(|b| format!("{b:?}"))?;
    let rec = chain.last().ok_or("empty chain")?;
    let json = serde_json::to_value(rec)?;
    println!(
        "live receipt verdict={:?} hash_self={} verify_chain=ok",
        rec.decision().verdict,
        rec.hash_self()
    );
    assert_eq!(rec.decision().verdict, hee4_contracts::Verdict::Pass);
    let text = json.to_string();
    let n = observed_model_requests(&dir.join("ledger.sqlite3"), id.as_str().ok_or("id")?)?;
    println!("model_request evidence rows on observations: {n}");
    assert!(n >= 1, "no model_request evidence; receipt {text}");
    Ok(())
}

fn observed_model_requests(ledger: &Path, task: &str) -> R<usize> {
    let conn =
        rusqlite::Connection::open_with_flags(ledger, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let mut n = 0;
    let mut q = conn.prepare("SELECT json FROM observations WHERE task_id = ?1")?;
    for row in q.query_map([task], |r| r.get::<_, String>(0))? {
        n += row?.matches("\"model_request\"").count();
    }
    Ok(n)
}

// ---- DC-22 backups and the deep-diff-forge observation (dispatcher-backups-ddf) ----

/// A backup root on another device than `dir` (the ledger's): `serve` passes
/// `SameDisk::Refuse`, so the e2e needs no same-device escape. `dir` is on `/dev/shm`
/// (`fsync_cheap_dir`); the root is under `CARGO_TARGET_TMPDIR`. Removed when the guard drops.
fn backup_root(dir: &Path, name: &str) -> R<(RunDir, PathBuf)> {
    use std::os::unix::fs::MetadataExt as _;
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_nanos();
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("hee4-bk-{name}-{}-{nanos}", std::process::id()));
    fs::create_dir_all(&root)?;
    let (ledger_dev, root_dev) = (fs::metadata(dir)?.dev(), fs::metadata(&root)?.dev());
    println!("MEASURED st_dev ledger_dir={ledger_dev} backup_root={root_dev}");
    if ledger_dev == root_dev {
        return Err(format!(
            "{} and {} share st_dev {ledger_dev}: no cross-device root for this test",
            dir.display(),
            root.display()
        )
        .into());
    }
    let bk = root.join("bk");
    Ok((RunDir(root), bk))
}

/// `hee4 serve --backups backups` over `dir`, with `env` set; polls health like `start_with`.
fn start_backed(dir: &Path, log: &str, backups: &Path, env: &[(&str, &str)]) -> R<Server> {
    let sock = dir.join("rt/control.sock");
    let mut cmd = serve_command();
    cmd.env_remove("HEE4_LIVE_MODEL")
        .env_remove("HEE4_BUDGETS")
        .env_remove("HEE4_REQUIRE_BACKUPS");
    for (k, v) in env {
        cmd.env(k, v);
    }
    let child = cmd
        .args(["serve", "--socket"])
        .arg(&sock)
        .arg("--ledger")
        .arg(dir.join("ledger.sqlite3"))
        .arg("--work")
        .arg(dir.join("work"))
        .arg("--backups")
        .arg(backups)
        .stdout(Stdio::null())
        .stderr(fs::File::create(dir.join(log))?)
        .spawn()?;
    let t0 = Instant::now();
    while t0.elapsed() < Duration::from_secs(20) {
        if let Ok(v) = call(&sock, "health", None, json!({}))
            && v["body"]["ok"] == true
        {
            return Ok(Server { child, sock });
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    Err("backed server did not become healthy".into())
}

fn lines_of(path: &Path) -> R<Vec<String>> {
    Ok(fs::read_to_string(path)?
        .lines()
        .map(str::to_owned)
        .collect())
}

/// The `id=` word of a `backup ...` line.
fn backup_id(line: &str) -> R<String> {
    line.split(' ')
        .find_map(|w| w.strip_prefix("id="))
        .map(str::to_owned)
        .ok_or_else(|| format!("no id= in {line}").into())
}

/// Submit `verify` under `key` and poll it to a terminal phase.
fn run_task(sock: &Path, key: &str, verify: &str) -> R<(String, Value)> {
    let got = call(
        sock,
        "task.submit",
        Some(key),
        json!({ "brief": brief(verify) }),
    )?;
    assert_eq!(got["kind"], "result", "{got}");
    let id = got["body"]["task_id"].clone();
    let done = poll(sock, &id, |p| TERMINAL.contains(&p), &mut Vec::new())?;
    Ok((id.as_str().ok_or("id")?.to_owned(), done))
}

/// `hee4 restore --into into id --backups bk` as a subprocess: `(exit code, stdout)`.
fn restore(into: &Path, id: &str, bk: &Path) -> R<(Option<i32>, String)> {
    let out = Command::new(BIN)
        .arg("restore")
        .arg("--into")
        .arg(into)
        .arg(id)
        .arg("--backups")
        .arg(bk)
        .output()?;
    println!(
        "restore stderr: {}",
        String::from_utf8_lossy(&out.stderr).trim()
    );
    Ok((
        out.status.code(),
        String::from_utf8(out.stdout)?.trim().to_owned(),
    ))
}

#[test]
fn backup_at_start_then_restore_into_round_trip() -> R<()> {
    let (_run, dir) = fsync_cheap_dir("e2e-backup")?;
    let (_bk_run, bk) = backup_root(&dir, "round-trip")?;
    let mut server = start_backed(&dir, "serve.log", &bk, &[])?;
    // Health answered, so the start backup is already on disk.
    let log = lines_of(&bk.join("backup.log"))?;
    println!("backup.log: {log:?}");
    assert_eq!(log.len(), 1, "{log:?}");
    let first = &log[0];
    assert!(
        first.starts_with("backup id=")
            && first.contains(" trigger=start ")
            && first.ends_with(" verdict=PASS"),
        "{first}"
    );
    let id = backup_id(first)?;
    assert!(bk.join(&id).join("manifest.json").is_file());
    let serve_log = fs::read_to_string(dir.join("serve.log"))?;
    assert!(
        serve_log.contains(&format!(" backups={}", bk.display())),
        "{serve_log}"
    );

    let (task, done) = run_task(&server.sock, "key-bk", FIXTURE)?;
    assert_eq!(done["body"]["phase"], "accepted", "{done}");
    let serve_log = fs::read_to_string(dir.join("serve.log"))?;
    // The fixture writes nothing: the workspace is diffed (no `.git` needed) and is empty.
    let skip = format!("dispatch task={task} ddf=skipped reason=no_diff");
    println!("serve.log has `{skip}`: {}", serve_log.contains(&skip));
    assert!(serve_log.contains(&skip), "{serve_log}");
    server.child.kill()?;
    server.child.wait()?;

    let into = dir.join("restored");
    let (code, line) = restore(&into, &id, &bk)?;
    println!("{line}");
    assert_eq!(code, Some(0), "{line}");
    assert!(
        line.starts_with(&format!("restore backup={id} ledger="))
            && line.ends_with(" verdict=PASS"),
        "{line}"
    );
    assert_eq!(lines_of(&bk.join("restore.log"))?, vec![line]);
    assert!(into.join("ledger.sqlite3").is_file());
    Ok(())
}

/// `serve.log` contains every needle, within `within`.
fn wait_log(log: &Path, needles: &[&str], within: Duration) -> R<String> {
    let t0 = Instant::now();
    loop {
        let text = fs::read_to_string(log).unwrap_or_default();
        if needles.iter().all(|n| text.lines().any(|l| l.contains(n))) {
            return Ok(text);
        }
        if t0.elapsed() > within {
            return Err(format!("{needles:?} not in {}: {text}", log.display()).into());
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

#[test]
fn batch_backup_fires_before_the_ninth_dispatch_and_a_failed_backup_blocks_dispatch_by_name()
-> R<()> {
    let (_run, dir) = fsync_cheap_dir("e2e-batch")?;
    let (_bk_run, bk) = backup_root(&dir, "batch")?;
    let mut server = start_backed(&dir, "serve.log", &bk, &[])?;
    let sock = server.sock.clone();
    for i in 1..=hee4_app::dispatcher::DC22_BATCH_TASKS {
        let (_, done) = run_task(&sock, &format!("key-batch-{i}"), FIXTURE)?;
        assert_eq!(done["body"]["phase"], "accepted", "{done}");
    }
    assert_eq!(
        lines_of(&bk.join("backup.log"))?.len(),
        1,
        "no batch backup before the ninth"
    );
    fs::set_permissions(&bk, fs::Permissions::from_mode(0o000))?;
    let ninth = call(
        &sock,
        "task.submit",
        Some("key-batch-9"),
        json!({ "brief": brief(FIXTURE) }),
    )?;
    let ninth_id = ninth["body"]["task_id"].clone();
    let t0 = Instant::now();
    while t0.elapsed() < Duration::from_secs(2) {
        let got = call(&sock, "task.get", None, json!({ "task_id": ninth_id }))?;
        if got["body"]["phase"] != "admitted" {
            fs::set_permissions(&bk, fs::Permissions::from_mode(0o700))?;
            return Err(format!("dispatched while the backup failed: {got}").into());
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    let blocked = wait_log(
        &dir.join("serve.log"),
        &["trigger=batch", "verdict=FAIL", "dispatch error: backup:"],
        Duration::from_secs(5),
    );
    fs::set_permissions(&bk, fs::Permissions::from_mode(0o700))?;
    let blocked = blocked?;
    for l in blocked
        .lines()
        .filter(|l| l.starts_with("backup ") || l.starts_with("dispatch error:"))
    {
        println!("serve.log: {l}");
    }
    assert!(
        blocked.lines().any(|l| l.starts_with("backup ")
            && l.contains(" trigger=batch ")
            && l.contains(" verdict=FAIL ")),
        "{blocked}"
    );
    let done = poll(&sock, &ninth_id, |p| TERMINAL.contains(&p), &mut Vec::new())?;
    assert_eq!(done["body"]["phase"], "accepted", "{done}");
    let log = lines_of(&bk.join("backup.log"))?;
    println!("backup.log: {log:#?}");
    assert_eq!(log.len(), 3, "{log:?}");
    assert!(
        log[0].contains(" trigger=start ") && log[0].ends_with(" verdict=PASS"),
        "{log:?}"
    );
    assert!(
        log[1].contains(" trigger=batch ") && log[1].contains(" verdict=FAIL reason="),
        "{log:?}"
    );
    assert!(
        log[2].contains(" trigger=batch ") && log[2].ends_with(" verdict=PASS"),
        "{log:?}"
    );
    let dispatch = serde_json::to_string(&Event::Dispatch)?;
    let conn = rusqlite::Connection::open_with_flags(
        dir.join("ledger.sqlite3"),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )?;
    let dispatches: i64 = conn.query_row(
        "SELECT count(*) FROM events WHERE task_id = ?1 AND event_json = ?2",
        [ninth_id.as_str().ok_or("id")?, dispatch.as_str()],
        |r| r.get(0),
    )?;
    assert_eq!(dispatches, 1, "one Dispatch for the ninth task");
    server.child.kill()?;
    server.child.wait()?;
    Ok(())
}

#[test]
fn serve_without_backups_is_refused_by_name_when_required() -> R<()> {
    let (_run, dir) = fsync_cheap_dir("e2e-require")?;
    let sock = dir.join("rt/control.sock");
    let mut child = serve_command()
        .env("HEE4_REQUIRE_BACKUPS", "1")
        .args(["serve", "--socket"])
        .arg(&sock)
        .arg("--ledger")
        .arg(dir.join("ledger.sqlite3"))
        .arg("--work")
        .arg(dir.join("work"))
        .stdout(Stdio::null())
        .stderr(fs::File::create(dir.join("serve.log"))?)
        .spawn()?;
    let t0 = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break status;
        }
        if t0.elapsed() > Duration::from_secs(5) {
            child.kill()?;
            child.wait()?;
            return Err("serve still running 5 s after the refusal was due".into());
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    let log = fs::read_to_string(dir.join("serve.log"))?;
    println!("exit={status} log={}", log.trim());
    assert!(!status.success());
    assert!(
        log.contains("hee4 serve refused: backups: --backups is required (HEE4_REQUIRE_BACKUPS=1)"),
        "{log}"
    );
    assert!(!sock.exists(), "no socket file");
    assert!(
        !dir.join("ledger.sqlite3").exists(),
        "refused before Store::open"
    );
    Ok(())
}

#[test]
fn restore_refuses_an_unknown_id_and_an_occupied_target_by_name() -> R<()> {
    let (_run, dir) = fsync_cheap_dir("e2e-restore-refuse")?;
    let (_bk_run, bk) = backup_root(&dir, "refuse")?;
    let mut server = start_backed(&dir, "serve.log", &bk, &[])?;
    server.child.kill()?;
    server.child.wait()?;
    let id = backup_id(
        lines_of(&bk.join("backup.log"))?
            .first()
            .ok_or("no backup line")?,
    )?;

    let (code, unknown) = restore(&dir.join("x"), "nosuch", &bk)?;
    println!("{unknown}");
    assert_eq!(code, Some(1), "{unknown}");
    assert!(
        unknown.starts_with("restore backup=nosuch ")
            && unknown.ends_with(" verdict=FAIL reason=not_found"),
        "{unknown}"
    );

    let occupied = dir.join("occupied");
    fs::create_dir_all(&occupied)?;
    fs::write(occupied.join("ledger.sqlite3"), b"not a ledger")?;
    let (code, refused) = restore(&occupied, &id, &bk)?;
    println!("{refused}");
    assert_eq!(code, Some(1), "{refused}");
    assert!(
        refused.starts_with(&format!("restore backup={id} "))
            && refused.ends_with(" verdict=FAIL reason=target_occupied"),
        "{refused}"
    );
    assert_eq!(fs::read(occupied.join("ledger.sqlite3"))?, b"not a ledger");
    assert_eq!(lines_of(&bk.join("restore.log"))?, vec![unknown, refused]);
    Ok(())
}

/// Whether `deep-diff-forge` is on this process's `PATH` (the serve child inherits it).
fn ddf_present() -> bool {
    std::env::var_os("PATH")
        .is_some_and(|p| std::env::split_paths(&p).any(|d| d.join("deep-diff-forge").is_file()))
}

/// The brief's VERIFY `sh: git init -q . && echo x > f && git add f` cannot run in the landed
/// sandbox: `bwrap` mounts no `/dev`, and git exits 128 `could not open '/dev/null'` (MEASURED,
/// hee4-host `spawn::plan` argv; the `--dev /dev` fix is hee4-host/hee4-worker's). So the
/// candidate makes its worktree the way git would, inside the sandbox: a `.git` directory and
/// one new file. Nothing is seeded on the host; the engine diffs the workspace in-process
/// against its pre-attempt snapshot.
#[test]
fn sealed_diff_observation_reaches_the_receipt() -> R<()> {
    const VERIFY: &str = "sh: /usr/bin/mkdir .git && echo x > f";
    const KEY: &str = "key-ddf";
    let (_run, dir) = fsync_cheap_dir("e2e-ddf")?;
    let mut server = start(&dir, "serve.log")?;
    let (task, done) = run_task(&server.sock, KEY, VERIFY)?;
    server.child.kill()?;
    server.child.wait()?;
    let log = fs::read_to_string(dir.join("serve.log"))?;
    for l in log.lines().filter(|l| l.contains("ddf")) {
        println!("serve.log: {l}");
    }
    assert_eq!(done["body"]["phase"], "accepted", "{done} {log}");
    if !ddf_present() {
        println!("UNMEASURED: deep-diff-forge not on PATH");
        assert!(
            log.contains(&format!(
                "dispatch task={task} ddf=skipped reason=tool_absent"
            )),
            "{log}"
        );
        return Ok(());
    }
    assert!(
        log.contains(&format!(
            "dispatch task={task} ddf=observed tool=deep-diff-forge "
        )),
        "{log}"
    );
    let ledger = dir.join("ledger.sqlite3");
    let conn =
        rusqlite::Connection::open_with_flags(&ledger, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let mut q = conn.prepare("SELECT id, json FROM observations WHERE task_id = ?1")?;
    let mut ddf_rows = Vec::new();
    for row in q.query_map([&task], |r| {
        Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
    })? {
        let (id, json) = row?;
        let obs: Value = serde_json::from_str(&json)?;
        if obs["tool"]["name"] == "deep-diff-forge" {
            ddf_rows.push((id, obs));
        }
    }
    assert_eq!(ddf_rows.len(), 1, "{ddf_rows:?}");
    let (obs_id, obs) = &ddf_rows[0];
    println!("MEASURED ddf observation {obs_id} {obs}");
    assert_eq!(
        obs["input_sha256"],
        Sha256Hex::digest(VERIFY.as_bytes()).to_string(),
        "bound to the VERIFY digest"
    );
    let chain = receipts(&ledger, &task)?;
    let receipt = chain.last().ok_or("no receipt")?;
    assert!(
        receipt.observed().iter().any(|o| o.to_string() == *obs_id),
        "{receipt:?}"
    );
    assert_eq!(receipt.decision().verdict, Verdict::Pass, "{receipt:?}");
    Ok(())
}

/// The refuter's escape (rung: test): a candidate that plants a valid `.git` whose config names
/// an fsmonitor hook, a clean filter and a textconv driver that write `<dir>/PWNED_*` — a path
/// outside the sandbox's binds, so only a host-side git could create it. The engine runs no git
/// over the workspace, so none of them exists afterwards and the task is still observed.
#[test]
fn a_candidate_git_config_runs_nothing_on_the_host() -> R<()> {
    let (_run, dir) = fsync_cheap_dir("e2e-ddf-pwn")?;
    let pwn = |what: &str| dir.join(format!("PWNED_{what}"));
    let verify = format!(
        r#"sh: /usr/bin/mkdir -p .git/objects .git/refs/heads && echo 'ref: refs/heads/main' > .git/HEAD && printf '[core]\n\trepositoryformatversion = 0\n\tfsmonitor = /usr/bin/touch {fsm}\n[filter "p"]\n\tclean = /usr/bin/touch {clean}\n[diff "p"]\n\ttextconv = /usr/bin/touch {conv}\n' > .git/config && echo '* filter=p diff=p' > .gitattributes && echo x > f"#,
        fsm = pwn("FSMONITOR").display(),
        clean = pwn("CLEAN").display(),
        conv = pwn("TEXTCONV").display(),
    );
    let mut server = start(&dir, "serve.log")?;
    let (task, done) = run_task(&server.sock, "key-pwn", &verify)?;
    server.child.kill()?;
    server.child.wait()?;
    let log = fs::read_to_string(dir.join("serve.log"))?;
    for l in log.lines().filter(|l| l.contains("ddf")) {
        println!("serve.log: {l}");
    }
    assert_eq!(done["body"]["phase"], "accepted", "{done} {log}");
    let config = dir
        .join("work")
        .join(&task)
        .join("1")
        .join(&task)
        .join(".git")
        .join("config");
    let planted = fs::read_to_string(&config)?;
    println!("planted .git/config:\n{planted}");
    assert!(planted.contains("fsmonitor = /usr/bin/touch "), "{planted}");
    for what in ["FSMONITOR", "CLEAN", "TEXTCONV"] {
        assert!(
            !pwn(what).exists(),
            "host ran the candidate's {what} command"
        );
    }
    if ddf_present() {
        assert!(
            log.contains(&format!(
                "dispatch task={task} ddf=observed tool=deep-diff-forge "
            )),
            "{log}"
        );
    } else {
        println!("UNMEASURED: deep-diff-forge not on PATH");
    }
    Ok(())
}

/// Acceptance 3 at the binary: a claim left behind in the ledger (its holder committed, then
/// died before the release) is reported by name in `serve.log` when `hee4 serve` starts. The
/// claim is seeded through the store's own verbs (no SQL here): claim `drive` at generation 1,
/// commit the act, never release. The `expired` reason needs a claim older than
/// `CLAIM_STALE_MS`, which only SQL could backdate; the store test covers it.
#[test]
fn a_stale_claim_is_reported_by_name_at_serve_start() -> R<()> {
    use hee4_contracts::{Evidence, Observation, Outcome, ToolId};
    use hee4_core::OperationKey;
    use hee4_core::service::{Expected, Seed};

    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("e2e-stale-claim");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir)?;
    let held = {
        let store = hee4_core::Store::open(&dir.join("ledger.sqlite3"))?;
        store.service_seed(&[Seed {
            service_id: "drive",
            owner_id: "deploy",
            unit_id: "hee4-drive.service",
            actable: true,
        }])?;
        let op = OperationKey {
            principal: "uid:dead".into(),
            action: "service.action".into(),
            version: 1,
            idem_key: "died-before-release".into(),
        };
        store.service_claim("drive", 1, &op)?;
        let held = store
            .service_claim_get("drive")?
            .ok_or("the claim was not taken")?;
        let health = Observation {
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
                sha256: Sha256Hex::digest(b"inactive"),
            }],
            advisory: false,
            elapsed_ms: 3,
            budget_ms: 5000,
        };
        let expected = Expected {
            generation: 1,
            owner_sha256: Sha256Hex::digest(b"deploy"),
        };
        store.service_action_commit(&op, b"{}", "drive", expected, &health, |_| json!(null))?;
        held
    };
    let mut server = start(&dir, "serve.log")?;
    let line = format!(
        "service family start stale_claim service_id=drive operation_id={} generation=1",
        held.operation_id
    );
    let found = wait_log(
        &dir.join("serve.log"),
        &[&line, "reason=generation_moved"],
        Duration::from_secs(10),
    );
    server.child.kill()?;
    server.child.wait()?;
    let log = found?;
    let named: Vec<&str> = log.lines().filter(|l| l.contains("stale_claim")).collect();
    println!("MEASURED serve.log stale_claim lines: {named:?}");
    assert_eq!(named.len(), 1, "{log}");
    assert!(named[0].contains(&line), "{log}");
    assert!(named[0].ends_with(" reason=generation_moved"), "{log}");
    Ok(())
}

/// `ledger.checkpoint_every` is read: with it at 2, two accepted tasks (two receipts) write at
/// least one `checkpoints` row (read-only SELECT), the dispatcher logs it, and `hee4
/// verify-ledger` re-derives every root and passes.
#[test]
fn checkpoints_are_written_every_n_receipts() -> R<()> {
    let (_run, dir) = fsync_cheap_dir("e2e-checkpoint")?;
    let budgets = dir.join("budgets.json");
    fs::write(&budgets, r#"{"ledger":{"checkpoint_every":2}}"#)?;
    let mut server = start_with(&dir, "serve.log", false, &[("HEE4_BUDGETS", &budgets)])?;
    for key in ["key-cp-1", "key-cp-2"] {
        let (_, done) = run_task(&server.sock, key, FIXTURE)?;
        assert_eq!(done["body"]["phase"], "accepted", "{done}");
    }
    server.child.kill()?;
    server.child.wait()?;
    let log = fs::read_to_string(dir.join("serve.log"))?;
    let ledger = dir.join("ledger.sqlite3");
    let conn =
        rusqlite::Connection::open_with_flags(&ledger, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let rows: i64 = conn.query_row("SELECT count(*) FROM checkpoints", [], |r| r.get(0))?;
    println!("MEASURED checkpoints={rows}");
    assert!(rows >= 1, "no checkpoint row; {log}");
    assert!(log.contains("dispatch checkpoint seq="), "{log}");
    let out = Command::new(BIN)
        .args(["verify-ledger", "--ledger"])
        .arg(&ledger)
        .output()?;
    let stdout = String::from_utf8(out.stdout)?;
    println!("{stdout}");
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.trim_end().ends_with("verdict=PASS"), "{stdout}");
    assert!(!stdout.contains(" checkpoints=0 "), "{stdout}");
    Ok(())
}

/// `hee4 serve` with `extra` after the base argv over `dir` and `env` set: `(exit code,
/// stderr)`. A serve still running after 5 s is killed and is an error (the refusal was due).
fn serve_refusal(dir: &Path, extra: &[&str], env: &[(&str, &str)]) -> R<(Option<i32>, String)> {
    let mut cmd = serve_command();
    cmd.env_remove("HEE4_BUDGETS")
        .env_remove("HEE4_REQUIRE_BACKUPS");
    for (k, v) in env {
        cmd.env(k, v);
    }
    let log = dir.join("refusal.log");
    let mut child = cmd
        .args(["serve", "--socket"])
        .arg(dir.join("rt/control.sock"))
        .arg("--ledger")
        .arg(dir.join("ledger.sqlite3"))
        .arg("--work")
        .arg(dir.join("work"))
        .args(extra)
        .stdout(Stdio::null())
        .stderr(fs::File::create(&log)?)
        .spawn()?;
    let t0 = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break status;
        }
        if t0.elapsed() > Duration::from_secs(5) {
            child.kill()?;
            child.wait()?;
            return Err(format!("serve {extra:?} still running after 5 s").into());
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    Ok((status.code(), fs::read_to_string(&log)?))
}

/// One refused serve: extra argv, extra env, and the text stderr must carry.
type FlagCase<'a> = (&'a [&'a str], &'a [(&'a str, &'a str)], &'a str);

/// Serve's flags and `HEE4_REQUIRE_BACKUPS` are parsed at the boundary: a flag with no value
/// (last, or followed by a flag), a repeated flag, a relative `--backups` and a
/// `HEE4_REQUIRE_BACKUPS` other than 1/0 each exit 2 naming the flag or variable, before the
/// ledger is opened.
#[test]
fn serve_flags_are_refused_by_name_at_the_boundary() -> R<()> {
    let (_run, dir) = fsync_cheap_dir("e2e-serve-flags")?;
    let cases: &[FlagCase] = &[
        (&["--budgets"], &[], "--budgets needs a value"),
        (
            &["--budgets", "--backups", "/x"],
            &[],
            "--budgets needs a value",
        ),
        (
            &["--budgets", "/a.json", "--budgets", "/b.json"],
            &[],
            "--budgets is given twice",
        ),
        (
            &["--backups", "rel/path"],
            &[],
            "--backups must be an absolute path",
        ),
        (
            &[],
            &[("HEE4_REQUIRE_BACKUPS", "true")],
            "HEE4_REQUIRE_BACKUPS must be 1 or 0",
        ),
    ];
    for &(extra, env, named) in cases {
        let (code, stderr) = serve_refusal(&dir, extra, env)?;
        println!(
            "{extra:?} {env:?} -> exit={code:?} {}",
            stderr.lines().next().unwrap_or("")
        );
        assert_eq!(code, Some(2), "{extra:?} {env:?}: {stderr}");
        assert!(stderr.contains(named), "{extra:?} {env:?}: {stderr}");
        assert!(
            !dir.join("ledger.sqlite3").exists(),
            "{extra:?}: refused before Store::open"
        );
    }
    Ok(())
}

/// The unit's exact `ExecStart` argv, with `%h`/`%t` expanded under this run and its
/// `--backups` value moved to a cross-device scratch root (never the live one), plus the unit's
/// `HEE4_REQUIRE_BACKUPS=1`, still parses and serves.
#[test]
fn the_unit_exec_start_argv_serves() -> R<()> {
    let (_run, dir) = fsync_cheap_dir("e2e-unit-argv")?;
    let (_bk_run, bk) = backup_root(&dir, "unit-argv")?;
    let unit = include_str!("../../../systemd/hee4.service");
    let exec = unit
        .lines()
        .find_map(|l| l.strip_prefix("ExecStart="))
        .ok_or("no ExecStart")?;
    let home = dir.join("home");
    let rt = dir.join("rt-unit");
    let mut words: Vec<String> = exec
        .split_whitespace()
        .skip(1)
        .map(|w| {
            w.replace("%h", &home.to_string_lossy())
                .replace("%t", &rt.to_string_lossy())
        })
        .collect();
    let at = words
        .iter()
        .position(|w| w == "--backups")
        .ok_or("the unit names no --backups")?;
    *words.get_mut(at + 1).ok_or("--backups has no value")? = bk.to_string_lossy().into_owned();
    fs::create_dir_all(home.join(".local/share/hee4"))?;
    let child = serve_command()
        .env_remove("HEE4_LIVE_MODEL")
        .env_remove("HEE4_BUDGETS")
        .env("HEE4_REQUIRE_BACKUPS", "1")
        .args(&words)
        .stdout(Stdio::null())
        .stderr(fs::File::create(dir.join("serve.log"))?)
        .spawn()?;
    let mut server = Server {
        child,
        sock: rt.join("hee4/control.sock"),
    };
    let t0 = Instant::now();
    let healthy = loop {
        if let Ok(v) = call(&server.sock, "health", None, json!({}))
            && v["body"]["ok"] == true
        {
            break true;
        }
        if t0.elapsed() > Duration::from_secs(20) || server.child.try_wait()?.is_some() {
            break false;
        }
        std::thread::sleep(Duration::from_millis(50));
    };
    server.child.kill()?;
    server.child.wait()?;
    let log = fs::read_to_string(dir.join("serve.log"))?;
    println!("argv {words:?}");
    assert!(healthy, "{log}");
    Ok(())
}
