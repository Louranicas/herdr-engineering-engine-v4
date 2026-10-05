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

use hee4_app::{socket, wire};
use hee4_contracts::{Receipt, Verdict};
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

struct Server {
    child: Child,
    sock: PathBuf,
}

fn start(dir: &Path, log: &str) -> R<Server> {
    start_with(dir, log, false)
}

fn start_with(dir: &Path, log: &str, live: bool) -> R<Server> {
    let sock = dir.join("rt/control.sock");
    let mut cmd = Command::new(BIN);
    if live {
        cmd.env("HEE4_LIVE_MODEL", "1");
    } else {
        cmd.env_remove("HEE4_LIVE_MODEL");
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
    println!(
        "MEASURED health schema_version={} serve_cgroup={}",
        body["schema_version"], body["serve_cgroup"]
    );
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
    let junk = vec![b'a'; wire::MAX_FRAME_BYTES + 2];
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
    let mut server = start(&dir, "serve.log")?;
    let mut held = Vec::new();
    for _ in 0..socket::MAX_CONNECTIONS {
        held.push(UnixStream::connect(&server.sock)?);
    }
    let extra = UnixStream::connect(&server.sock)?;
    extra.set_read_timeout(Some(Duration::from_secs(10)))?;
    let mut line = String::new();
    BufReader::new(&extra).read_line(&mut line)?;
    let v: Value = serde_json::from_str(&line)?;
    assert_eq!(v["code"], "too_many_connections", "{v}");
    println!("MEASURED connection {} -> {v}", socket::MAX_CONNECTIONS + 1);
    // An admitted connection is unharmed: health answers on it.
    let mut first = held.remove(0);
    first.set_read_timeout(Some(Duration::from_secs(10)))?;
    first.write_all(format!("{}\n", wire::request("cap", "health", None, json!({}))).as_bytes())?;
    let mut line = String::new();
    BufReader::new(&first).read_line(&mut line)?;
    let h: Value = serde_json::from_str(&line)?;
    assert_eq!(h["body"]["ok"], true, "{h}");
    // Closing one frees a slot.
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

/// A fresh test directory on tmpfs when `/dev/shm` is writable, else under
/// `CARGO_TARGET_TMPDIR`. The slow-consumer proof needs ~300 admitted tasks (every submit and
/// every dispatcher step is one `synchronous=FULL` transaction plus a brief `fsync`): measured
/// 33 ms per submit on this machine's `/home` (dm-crypt) and 0.3 ms on tmpfs, with or without a
/// subscriber, so the disk is the test's clock and not what it proves.
///
/// `/dev/shm` is machine-global, so the root is unique per run (package, pid, nanosecond
/// stamp): concurrent slices or cargo invocations never share, delete or `pkill -f` each
/// other's directories. The guard removes the root when the test ends, pass or panic.
struct RunDir(PathBuf);

impl Drop for RunDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn fsync_cheap_dir(name: &str) -> R<(RunDir, PathBuf)> {
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

/// The server's `slow_consumer` log line: the reader dropped the subscriber, or the close frame
/// missed its deadline (`log_line` below is the second, the FLOW's "not delivered").
fn log_says(log: &Path, needle: &str) -> bool {
    fs::read_to_string(log).is_ok_and(|s| s.contains(needle))
}

fn is_close_frame(line: &str) -> bool {
    line.contains("\"kind\":\"close\"") && line.contains("slow_consumer")
}

/// A subscriber that reads one line per 64 submits is a slow consumer against a real
/// `hee4 serve`: the submit loop stops at the first signal (the server's log line, a close frame
/// or EOF on the subscriber) or at 3000 submits, and the wait after it is bounded by the same
/// signals, so the test finishes in seconds without weakening `delivered || logged`.
#[test]
fn a_subscriber_that_never_reads_gets_the_close_frame_or_the_log_line() -> R<()> {
    let (_run, dir) = fsync_cheap_dir("e2e-slow")?;
    let mut server = start(&dir, "serve.log")?;
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
        let mut l = String::new();
        match sub.read_line(&mut l) {
            Ok(0) => signal = Some("eof"),
            Ok(_) if is_close_frame(&l) => {
                delivered = true;
                signal = Some("close_frame");
            }
            Ok(_) | Err(_) => {}
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
    let mut server = start_with(&dir, "serve-live.log", true)?;
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
