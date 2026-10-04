//! One task end to end through `hee4 serve`, then `kill -9` mid-dispatch and a restart.

use std::error::Error;
use std::fs;
use std::io::{BufRead as _, BufReader, Write as _};
use std::os::unix::fs::PermissionsExt as _;
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use hee4_app::{socket, wire};
use hee4_contracts::Receipt;
use serde_json::{Value, json};

type R<T> = Result<T, Box<dyn Error>>;

const BIN: &str = env!("CARGO_BIN_EXE_hee4");
const TERMINAL: [&str; 4] = ["accepted", "failed", "cancelled", "abandoned"];

fn brief(verify: &str) -> String {
    format!(
        "GOAL: g\nSCOPE: s\nCONTEXT: c\nACCEPTANCE: a\nVERIFY: {verify}\nTIMEBOX: 60s\nFORBIDDEN: f\n\
         REPORT: r\nSTANDING: s\nRECON: r\nRESTATEMENT: run {verify} in the namespace\n"
    )
}

struct Server {
    child: Child,
    sock: PathBuf,
}

fn start(dir: &Path, log: &str) -> R<Server> {
    let sock = dir.join("rt/control.sock");
    let child = Command::new(BIN)
        .args(["serve", "--socket"])
        .arg(&sock)
        .arg("--ledger")
        .arg(dir.join("ledger.sqlite3"))
        .arg("--work")
        .arg(dir.join("work"))
        .env_remove("HEE4_LIVE_MODEL")
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

    // Task A: VERIFY /usr/bin/true, no live model.
    let a = call(
        &sock,
        "task.submit",
        Some("key-a"),
        json!({ "brief": brief("/usr/bin/true") }),
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
    println!(
        "task A receipt verdict={:?} hash_self={} verify_chain=ok",
        last.decision().verdict,
        last.hash_self()
    );
    let replay = call(
        &sock,
        "task.submit",
        Some("key-a"),
        json!({ "brief": brief("/usr/bin/true") }),
    )?;
    assert_eq!(replay["replayed"], true);

    // Task B: a long step, killed mid-dispatch.
    let b = call(
        &sock,
        "task.submit",
        Some("key-b"),
        json!({ "brief": brief("/usr/bin/sleep 30") }),
    )?;
    let b_id = b["body"]["task_id"].clone();
    let mut trace_b = vec!["admitted".to_owned()];
    poll(&sock, &b_id, |p| p == "running", &mut trace_b)?;
    server.child.kill()?; // SIGKILL
    server.child.wait()?;
    trace_b.push("<SIGKILL>".into());

    let mut server = start(&dir, "serve-2.log")?;
    let health = call(&server.sock, "health", None, json!({}))?;
    assert_eq!(health["body"]["recovery_complete"], true);
    let after = poll(&server.sock, &b_id, |_| true, &mut trace_b)?;
    let phase = after["body"]["phase"].as_str().unwrap_or("?");
    assert!(
        TERMINAL.contains(&phase) || ["effect_unknown", "blocked"].contains(&phase),
        "{phase}"
    );
    println!("task B {b_id} trace {}", trace_b.join(" -> "));
    println!("health after restart {}", health["body"]);
    for log in ["serve-1.log", "serve-2.log"] {
        for line in fs::read_to_string(dir.join(log))?.lines() {
            println!("{log}: {line}");
        }
    }
    let log2 = fs::read_to_string(dir.join("serve-2.log"))?;
    assert!(
        log2.contains("R07ProcessNotOurs") || log2.contains("R08WorkerAbsent"),
        "{log2}"
    );
    server.child.kill()?;
    server.child.wait()?;
    // The orphaned bwrap child of task B (if still alive) is the test's to clean up.
    let work = dir.join("work").to_string_lossy().into_owned();
    let _ = Command::new("pkill").args(["-KILL", "-f", &work]).status();
    Ok(())
}

/// Open an `events.subscribe` stream after `since`, returning the reader past the ack.
fn subscribe(sock: &Path, since: i64) -> R<BufReader<UnixStream>> {
    let mut s = UnixStream::connect(sock)?;
    let frame = wire::request(
        "sub",
        "events.subscribe",
        None,
        json!({ "since_seq": since }),
    );
    s.write_all(format!("{frame}\n").as_bytes())?;
    s.set_read_timeout(Some(Duration::from_secs(30)))?;
    let mut r = BufReader::new(s);
    let mut ack = String::new();
    r.read_line(&mut ack)?;
    let ack: Value = serde_json::from_str(&ack)?;
    assert_eq!(ack["body"]["since_seq"], since, "{ack}");
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
    let mut live = subscribe(&sock, 0)?;
    let a = call(
        &sock,
        "task.submit",
        Some("key-s"),
        json!({ "brief": brief("/usr/bin/true") }),
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
    let mut resumed = subscribe(&sock, seqs[1])?;
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
