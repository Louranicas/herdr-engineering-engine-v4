//! The model door: the candidate's only path to the model.
//!
//! The candidate runs with no network (`--unshare-net`). [`serve`] binds a unix socket on the
//! host; `spawn::plan` bind-mounts that socket into the namespace. Each connection carries one
//! HTTP/1.1 request, which the door forwards to a loopback TCP upstream (Ollama at
//! `127.0.0.1:11434`) with `Connection: close`, then relays the response. When the upstream does
//! not accept, the door answers `503 {"refused":"model unreachable"}`. Only `GET`/`POST` of
//! `/api/...` over `HTTP/1.1` is forwarded; anything else is answered `400`. Connections are served
//! on their own threads (at most [`DoorBudget::pool`] at once). Every request is logged as a
//! [`DoorRequest`] (byte count and sha256; for a refused request, of what was actually read).

use std::io::{ErrorKind, Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::os::unix::fs::FileTypeExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread;
use std::time::{Duration, Instant};

pub use hee4_contracts::DoorBudget;
use hee4_contracts::Sha256Hex;

/// A loopback HTTP upstream, parsed from `http://<loopback-ip>:<port>`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Upstream(SocketAddr);

/// Why an upstream URL was refused.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum UpstreamRefusal {
    /// Not `http://<ip>:<port>`.
    #[error("upstream must be http://<ip>:<port>: {0}")]
    Malformed(String),
    /// The door forwards to loopback only.
    #[error("upstream is not loopback: {0}")]
    NotLoopback(String),
}

impl Upstream {
    /// Parse `http://127.0.0.1:11434` (a trailing `/` is allowed). Only a loopback address is
    /// accepted: the door never opens a path off the host.
    ///
    /// # Errors
    /// [`UpstreamRefusal`] for another scheme, a host name, or a non-loopback address.
    pub fn parse(url: &str) -> Result<Self, UpstreamRefusal> {
        let rest = url
            .strip_prefix("http://")
            .map(|r| r.trim_end_matches('/'))
            .ok_or_else(|| UpstreamRefusal::Malformed(url.to_owned()))?;
        let addr: SocketAddr = rest
            .parse()
            .map_err(|_| UpstreamRefusal::Malformed(url.to_owned()))?;
        if !addr.ip().is_loopback() {
            return Err(UpstreamRefusal::NotLoopback(url.to_owned()));
        }
        Ok(Self(addr))
    }

    /// The socket address.
    #[must_use]
    pub fn addr(&self) -> SocketAddr {
        self.0
    }
}

/// What happened to a request at the door.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DoorFate {
    /// Sent upstream; the response was relayed.
    Forwarded,
    /// Upstream did not accept: answered `503`.
    Unreachable,
    /// Over the request or body budget, or not a parseable request: answered `4xx`.
    Refused(u16),
}

/// One request seen at the door.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DoorRequest {
    /// Request body length in bytes; for a `refused` row, every byte the door actually read.
    pub bytes: u64,
    /// sha256 of the request body; for a `refused` row, of the bytes read.
    pub sha256: Sha256Hex,
    /// What the door did with it.
    pub fate: DoorFate,
    /// `forwarded`, `unreachable` or `refused`.
    pub label: &'static str,
    /// The refusal name when `label` is `refused`, else empty.
    pub reason: &'static str,
}

/// Why a door could not open.
#[derive(Debug, thiserror::Error)]
pub enum DoorError {
    /// A non-socket file already sits at the path.
    #[error("door path occupied by a non-socket: {0}")]
    Occupied(PathBuf),
    /// Binding or spawning failed.
    #[error("door io: {0}")]
    Io(#[from] std::io::Error),
}

/// A running door. [`Door::close`] stops it and returns the log; dropping it also stops it.
#[derive(Debug)]
pub struct Door {
    path: PathBuf,
    log: Arc<Mutex<Vec<DoorRequest>>>,
    stop: Arc<AtomicBool>,
    worker: Option<thread::JoinHandle<()>>,
}

impl Door {
    /// The socket path (host path = namespace path).
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The requests seen so far.
    #[must_use]
    pub fn requests(&self) -> Vec<DoorRequest> {
        self.log
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    /// Stop accepting, remove the socket, and return every request seen.
    #[must_use]
    pub fn close(mut self) -> Vec<DoorRequest> {
        self.shutdown();
        self.requests()
    }

    fn shutdown(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(w) = self.worker.take() {
            // Wake the blocking accept; it sees `stop` and returns.
            let _ = UnixStream::connect(&self.path);
            let _ = w.join();
        }
        let _ = std::fs::remove_file(&self.path);
    }
}

impl Drop for Door {
    fn drop(&mut self) {
        self.shutdown();
    }
}

/// Open a door at `socket_path` forwarding to `upstream` under `budget`. A stale socket at the
/// path is replaced; any other file there is refused. One request per connection, each
/// connection on its own thread, at most `budget.pool` at once.
///
/// # Errors
/// [`DoorError`] when the path holds a non-socket or the socket cannot be bound.
pub fn serve(
    socket_path: &Path,
    upstream: Upstream,
    budget: DoorBudget,
) -> Result<Door, DoorError> {
    match std::fs::symlink_metadata(socket_path) {
        Ok(m) if m.file_type().is_socket() => std::fs::remove_file(socket_path)?,
        Ok(_) => return Err(DoorError::Occupied(socket_path.to_path_buf())),
        Err(_) => {}
    }
    let listener = UnixListener::bind(socket_path)?;
    let log = Arc::new(Mutex::new(Vec::new()));
    let stop = Arc::new(AtomicBool::new(false));
    let worker = {
        let (log, stop) = (Arc::clone(&log), Arc::clone(&stop));
        thread::Builder::new()
            .name("hee4-model-door".into())
            .spawn(move || accept_loop(&listener, upstream, budget, log, &stop))?
    };
    Ok(Door {
        path: socket_path.to_path_buf(),
        log,
        stop,
        worker: Some(worker),
    })
}

/// State shared by every connection thread of one door.
struct Shared {
    log: Arc<Mutex<Vec<DoorRequest>>>,
    /// Slots reserved for forwarding (against `max_requests`).
    forwarded: AtomicU64,
    /// Sum of request bodies read in this attempt (against `max_total_bytes`).
    total_bytes: AtomicU64,
}

/// A contracts byte or count budget (`u64`) as the `usize` the buffers compare against.
fn as_usize(v: u64) -> usize {
    usize::try_from(v).unwrap_or(usize::MAX)
}

fn accept_loop(
    listener: &UnixListener,
    upstream: Upstream,
    budget: DoorBudget,
    log: Arc<Mutex<Vec<DoorRequest>>>,
    stop: &AtomicBool,
) {
    let shared = Arc::new(Shared {
        log,
        forwarded: AtomicU64::new(0),
        total_bytes: AtomicU64::new(0),
    });
    let pool = as_usize(budget.pool);
    let mut live: Vec<thread::JoinHandle<()>> = Vec::new();
    for conn in listener.incoming() {
        if stop.load(Ordering::SeqCst) {
            break;
        }
        let Ok(client) = conn else { continue };
        loop {
            live.retain(|h| !h.is_finished());
            if live.len() < pool {
                break;
            }
            thread::sleep(Duration::from_millis(5));
        }
        let shared = Arc::clone(&shared);
        if let Ok(h) = thread::Builder::new()
            .name("hee4-model-door-conn".into())
            .spawn(move || {
                if let Some(req) = handle(client, upstream, budget, &shared) {
                    shared
                        .log
                        .lock()
                        .unwrap_or_else(PoisonError::into_inner)
                        .push(req);
                }
            })
        {
            live.push(h);
        }
    }
    for h in live {
        let _ = h.join();
    }
}

/// A parsed request: the head lines (request line first) and the body.
struct Request {
    head: Vec<String>,
    body: Vec<u8>,
}

/// Why a request could not be read: the status, the name, and every byte read so far.
struct Bad {
    code: u16,
    why: &'static str,
    raw: Vec<u8>,
}

fn bad(code: u16, why: &'static str, raw: &[u8]) -> Bad {
    Bad {
        code,
        why,
        raw: raw.to_vec(),
    }
}

/// Accept only `GET|POST SP /api/... SP HTTP/1.1`.
fn valid_request_line(line: &[u8]) -> bool {
    let Ok(line) = std::str::from_utf8(line) else {
        return false;
    };
    let mut p = line.split(' ');
    let (Some(method), Some(path), Some(version), None) = (p.next(), p.next(), p.next(), p.next())
    else {
        return false;
    };
    matches!(method, "GET" | "POST")
        && version == "HTTP/1.1"
        && path.starts_with("/api/")
        && path.bytes().all(|b| b.is_ascii_graphic())
}

enum Pull {
    Got(usize),
    Eof,
    Timeout,
}

/// One read that gives up at `deadline`.
fn pull(s: &mut UnixStream, chunk: &mut [u8], deadline: Instant) -> Pull {
    let left = deadline.saturating_duration_since(Instant::now());
    if left.is_zero() || s.set_read_timeout(Some(left)).is_err() {
        return Pull::Timeout;
    }
    match s.read(chunk) {
        Ok(0) => Pull::Eof,
        Ok(n) => Pull::Got(n),
        Err(e) if matches!(e.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => Pull::Timeout,
        Err(_) => Pull::Eof,
    }
}

fn read_request(s: &mut UnixStream, budget: DoorBudget) -> Result<Request, Bad> {
    let mut buf = Vec::new();
    let mut chunk = [0u8; 8192];
    let header_deadline = Instant::now() + budget.header_deadline();
    let max_header = as_usize(budget.max_header_bytes);
    let mut line_ok = false;
    let end = loop {
        if !line_ok && let Some(i) = buf.windows(2).position(|w| w == b"\r\n") {
            if !valid_request_line(&buf[..i]) {
                return Err(bad(400, "bad request line", &buf));
            }
            line_ok = true;
        }
        if let Some(i) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
            break i;
        }
        if buf.len() > max_header {
            return Err(bad(431, "header too large", &buf));
        }
        match pull(s, &mut chunk, header_deadline) {
            Pull::Got(n) => buf.extend_from_slice(&chunk[..n]),
            Pull::Eof => return Err(bad(400, "incomplete request", &buf)),
            Pull::Timeout => return Err(bad(408, "header timeout", &buf)),
        }
    };
    let head_text =
        std::str::from_utf8(&buf[..end]).map_err(|_| bad(400, "header not utf-8", &buf))?;
    let head: Vec<String> = head_text.split("\r\n").map(str::to_owned).collect();
    let mut len = 0usize;
    for h in &head[1..] {
        let Some((k, v)) = h.split_once(':') else {
            continue;
        };
        if k.eq_ignore_ascii_case("transfer-encoding") {
            return Err(bad(
                411,
                "chunked bodies are not forwarded; send Content-Length",
                &buf,
            ));
        }
        if k.eq_ignore_ascii_case("content-length") {
            len = v
                .trim()
                .parse()
                .map_err(|_| bad(400, "bad content-length", &buf))?;
        }
    }
    if len > as_usize(budget.max_body_bytes) {
        return Err(bad(413, "body over budget", &buf));
    }
    let body_deadline = Instant::now() + budget.body_deadline();
    let mut body = buf[end + 4..].to_vec();
    while body.len() < len {
        match pull(s, &mut chunk, body_deadline) {
            Pull::Got(n) => body.extend_from_slice(&chunk[..n]),
            Pull::Eof => return Err(bad(400, "short body", &raw_of(&buf[..end + 4], &body))),
            Pull::Timeout => return Err(bad(408, "body timeout", &raw_of(&buf[..end + 4], &body))),
        }
    }
    body.truncate(len);
    Ok(Request { head, body })
}

/// Head bytes plus the body bytes read so far.
fn raw_of(head: &[u8], body: &[u8]) -> Vec<u8> {
    let mut v = head.to_vec();
    v.extend_from_slice(body);
    v
}

fn answer(s: &mut UnixStream, status: u16, reason: &str, json_body: &str) {
    let msg = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{json_body}",
        json_body.len()
    );
    let _ = s.write_all(msg.as_bytes());
}

fn refusal(why: &str) -> String {
    serde_json::json!({ "refused": why }).to_string()
}

/// A refused row: `bytes` and `sha256` are of what the door actually read.
fn refused_row(code: u16, why: &'static str, read: &[u8]) -> DoorRequest {
    DoorRequest {
        bytes: read.len() as u64,
        sha256: Sha256Hex::digest(read),
        fate: DoorFate::Refused(code),
        label: "refused",
        reason: why,
    }
}

fn handle(
    mut client: UnixStream,
    upstream: Upstream,
    budget: DoorBudget,
    shared: &Shared,
) -> Option<DoorRequest> {
    let _ = client.set_write_timeout(Some(budget.io_timeout()));
    let req = match read_request(&mut client, budget) {
        Ok(r) => r,
        // A connect that sent nothing (the shutdown wake-up): no request, no reply.
        Err(b) if b.raw.is_empty() && b.code == 400 => return None,
        Err(b) => {
            let shown = if b.code == 400 { "bad request" } else { b.why };
            answer(&mut client, b.code, "Refused", &refusal(shown));
            return Some(refused_row(b.code, b.why, &b.raw));
        }
    };
    let len = req.body.len() as u64;
    let before = shared.total_bytes.fetch_add(len, Ordering::SeqCst);
    if before.saturating_add(len) > budget.max_total_bytes {
        answer(
            &mut client,
            413,
            "Payload Too Large",
            &refusal("byte budget exhausted"),
        );
        return Some(refused_row(413, "byte budget exhausted", &req.body));
    }
    let record = |fate, label| DoorRequest {
        bytes: len,
        sha256: Sha256Hex::digest(&req.body),
        fate,
        label,
        reason: "",
    };
    if shared.forwarded.fetch_add(1, Ordering::SeqCst) >= budget.max_requests {
        shared.forwarded.fetch_sub(1, Ordering::SeqCst);
        answer(
            &mut client,
            429,
            "Too Many Requests",
            &refusal("door request budget exhausted"),
        );
        return Some(refused_row(429, "door request budget exhausted", &req.body));
    }
    let unreachable = |client: &mut UnixStream| {
        shared.forwarded.fetch_sub(1, Ordering::SeqCst);
        answer(
            client,
            503,
            "Service Unavailable",
            &refusal("model unreachable"),
        );
        record(DoorFate::Unreachable, "unreachable")
    };
    let Ok(mut up) = TcpStream::connect_timeout(&upstream.addr(), budget.io_timeout()) else {
        return Some(unreachable(&mut client));
    };
    let _ = up.set_read_timeout(Some(budget.io_timeout()));
    let _ = up.set_write_timeout(Some(budget.io_timeout()));
    let mut out = format!("{}\r\n", req.head[0]);
    for h in &req.head[1..] {
        let name = h.split(':').next().unwrap_or("").trim();
        let hop = ["connection", "keep-alive", "host", "proxy-connection"]
            .iter()
            .any(|n| name.eq_ignore_ascii_case(n));
        if !hop {
            out.push_str(h);
            out.push_str("\r\n");
        }
    }
    out.push_str("Host: ");
    out.push_str(&upstream.addr().to_string());
    out.push_str("\r\nConnection: close\r\n\r\n");
    let mut wire = out.into_bytes();
    wire.extend_from_slice(&req.body);
    if up.write_all(&wire).is_err() {
        return Some(unreachable(&mut client));
    }
    let _ = std::io::copy(&mut up, &mut client);
    Some(record(DoorFate::Forwarded, "forwarded"))
}

#[cfg(test)]
mod tests {
    type R = Result<(), Box<dyn std::error::Error>>;
    use super::*;
    use hee4_contracts::Budgets;
    use std::net::TcpListener;
    use std::sync::mpsc;

    fn sock(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("hee4-door-{}-{name}.sock", std::process::id()))
    }

    /// A door budget through the K0 door: `door_json` is the `door` section, validated by
    /// `Budgets::parse` (floors, ceilings, order), never a host-side literal.
    fn door_budget(door_json: &str) -> Result<DoorBudget, Box<dyn std::error::Error>> {
        Ok(Budgets::parse(&format!(r#"{{"door":{door_json}}}"#))?.door)
    }

    /// A 200-byte header block in two writes: the lines without the terminator, then, after
    /// `gap`, the terminator. The door sees the lines on their own first, so the size check
    /// runs on them before the end of the block can be found.
    fn ask_split_header(path: &Path, gap: Duration) -> std::io::Result<String> {
        let pad = "x".repeat(200 - "GET /api/tags HTTP/1.1\r\nHost: model\r\nX-Pad: \r\n".len());
        let lines = format!("GET /api/tags HTTP/1.1\r\nHost: model\r\nX-Pad: {pad}\r\n");
        let mut s = UnixStream::connect(path)?;
        s.write_all(lines.as_bytes())?;
        thread::sleep(gap);
        let _ = s.write_all(b"\r\n");
        let mut resp = String::new();
        s.read_to_string(&mut resp)?;
        Ok(resp)
    }

    /// Mock upstream on 127.0.0.1: answers every connection with `body`, and sends each raw
    /// request it read to the returned channel.
    fn mock(body: &'static str) -> std::io::Result<(Upstream, mpsc::Receiver<String>)> {
        let l = TcpListener::bind("127.0.0.1:0")?;
        let up = Upstream(l.local_addr()?);
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            for conn in l.incoming() {
                let Ok(mut s) = conn else { continue };
                let mut buf = vec![0u8; 8192];
                let n = s.read(&mut buf).unwrap_or(0);
                let _ = tx.send(String::from_utf8_lossy(&buf[..n]).into_owned());
                let _ = s.write_all(
                    format!(
                        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                        body.len()
                    )
                    .as_bytes(),
                );
            }
        });
        Ok((up, rx))
    }

    fn ask(path: &Path, raw: &str) -> std::io::Result<String> {
        let mut s = UnixStream::connect(path)?;
        s.write_all(raw.as_bytes())?;
        let mut resp = String::new();
        s.read_to_string(&mut resp)?;
        Ok(resp)
    }

    fn ask_raw(path: &Path, raw: &[u8]) -> std::io::Result<String> {
        let mut s = UnixStream::connect(path)?;
        s.write_all(raw)?;
        let mut resp = Vec::new();
        s.read_to_end(&mut resp)?;
        Ok(String::from_utf8_lossy(&resp).into_owned())
    }

    fn assert_bad_request(resp: &str) -> R {
        assert!(resp.starts_with("HTTP/1.1 400 "), "{resp}");
        let body = resp.split("\r\n\r\n").nth(1).ok_or("no body")?;
        let v: serde_json::Value = serde_json::from_str(body)?;
        assert_eq!(v, serde_json::json!({"refused": "bad request"}));
        Ok(())
    }

    #[test]
    fn garbage_gets_400_is_logged_and_never_forwarded() -> R {
        let (up, rx) = mock("{}")?;
        let path = sock("garbage");
        let door = serve(&path, up, DoorBudget::DEFAULT)?;
        let raw: &[u8] = b"\x00\xff\x01garbage\r\n\r\n";
        assert_bad_request(&ask_raw(&path, raw)?)?;
        assert!(rx.recv_timeout(Duration::from_millis(300)).is_err());
        let log = door.close();
        assert_eq!(log.len(), 1);
        assert_eq!(log[0].fate, DoorFate::Refused(400));
        assert_eq!(log[0].label, "refused");
        assert_eq!(log[0].reason, "bad request line");
        assert_eq!(log[0].bytes, raw.len() as u64);
        assert_eq!(log[0].sha256, Sha256Hex::digest(raw));
        Ok(())
    }

    #[test]
    fn http09_and_connect_and_non_api_paths_get_400() -> R {
        let (up, rx) = mock("{}")?;
        let path = sock("badline");
        let door = serve(&path, up, DoorBudget::DEFAULT)?;
        for raw in [
            "GET /api/tags\r\n\r\n",
            "CONNECT example.com:443 HTTP/1.1\r\nHost: x\r\n\r\n",
            "GET /etc/passwd HTTP/1.1\r\nHost: x\r\n\r\n",
            "DELETE /api/tags HTTP/1.1\r\nHost: x\r\n\r\n",
            "GET /api/tags HTTP/1.0\r\nHost: x\r\n\r\n",
            "GET  /api/tags HTTP/1.1\r\nHost: x\r\n\r\n",
        ] {
            assert_bad_request(&ask(&path, raw)?)?;
        }
        assert!(rx.recv_timeout(Duration::from_millis(300)).is_err());
        let log = door.close();
        assert_eq!(log.len(), 6);
        assert!(log.iter().all(|r| r.fate == DoorFate::Refused(400)));
        Ok(())
    }

    #[test]
    fn stalled_client_does_not_block_a_healthy_one() -> R {
        let (up, _rx) = mock("{}")?;
        let path = sock("stall");
        let door = serve(&path, up, DoorBudget::DEFAULT)?;
        let mut stalled = UnixStream::connect(&path)?;
        stalled.write_all(b"GET /api/ta")?;
        let t = Instant::now();
        let resp = ask(&path, "GET /api/tags HTTP/1.1\r\nHost: model\r\n\r\n")?;
        let took = t.elapsed();
        assert!(resp.starts_with("HTTP/1.1 200"), "{resp}");
        assert!(took < Duration::from_secs(1), "healthy took {took:?}");
        let mut late = String::new();
        stalled.read_to_string(&mut late)?;
        assert!(late.starts_with("HTTP/1.1 408 "), "{late}");
        assert!(late.contains(r#"{"refused":"header timeout"}"#), "{late}");
        let log = door.close();
        assert!(log.iter().any(|r| r.reason == "header timeout"
            && r.fate == DoorFate::Refused(408)
            && r.bytes == 11));
        Ok(())
    }

    #[test]
    fn header_over_budget_answers_431() -> R {
        let (up, rx) = mock("{}")?;
        let path = sock("431");
        let door = serve(&path, up, door_budget(r#"{"max_header_bytes":64}"#)?)?;
        let resp = ask_split_header(&path, Duration::from_millis(100))?;
        assert!(resp.starts_with("HTTP/1.1 431 "), "{resp}");
        assert!(resp.contains(r#"{"refused":"header too large"}"#), "{resp}");
        assert!(rx.recv_timeout(Duration::from_millis(300)).is_err());
        let log = door.close();
        assert_eq!(log.len(), 1);
        assert_eq!(log[0].fate, DoorFate::Refused(431));
        assert_eq!(log[0].reason, "header too large");
        assert_eq!(log[0].bytes, 200, "the lines sent before the terminator");

        let path = sock("431-default");
        let door = serve(&path, up, DoorBudget::DEFAULT)?;
        let resp = ask_split_header(&path, Duration::from_millis(100))?;
        assert!(resp.starts_with("HTTP/1.1 200"), "{resp}");
        let seen = rx.recv_timeout(Duration::from_secs(5))?;
        assert!(seen.starts_with("GET /api/tags HTTP/1.1\r\n"), "{seen}");
        let log = door.close();
        assert_eq!(log.len(), 1);
        assert_eq!(log[0].fate, DoorFate::Forwarded);
        Ok(())
    }

    #[test]
    fn pool_of_one_waits_for_the_stalled_slot() -> R {
        let (up, _rx) = mock("{}")?;
        let path = sock("pool1");
        let budget = door_budget(r#"{"pool":1,"header_deadline_ms":200}"#)?;
        let door = serve(&path, up, budget)?;
        let mut stalled = UnixStream::connect(&path)?;
        stalled.write_all(b"GET /api/ta")?;
        let t = Instant::now();
        let resp = ask(&path, "GET /api/tags HTTP/1.1\r\nHost: model\r\n\r\n")?;
        let took = t.elapsed();
        assert!(resp.starts_with("HTTP/1.1 200"), "{resp}");
        assert!(
            took >= Duration::from_millis(150),
            "healthy answered in {took:?}: the stalled slot was not the only one"
        );
        assert!(took < Duration::from_secs(5), "healthy took {took:?}");
        let mut late = String::new();
        stalled.read_to_string(&mut late)?;
        assert!(late.starts_with("HTTP/1.1 408 "), "{late}");
        let log = door.close();
        assert!(log.iter().any(|r| r.reason == "header timeout"
            && r.fate == DoorFate::Refused(408)
            && r.bytes == 11));
        Ok(())
    }

    #[test]
    fn cumulative_byte_budget_trips_on_the_nth_request() -> R {
        let (up, _rx) = mock("{}")?;
        let path = sock("cum");
        let budget = DoorBudget {
            max_body_bytes: 100,
            max_total_bytes: 100,
            ..DoorBudget::DEFAULT
        };
        let door = serve(&path, up, budget)?;
        let body = "x".repeat(40);
        let post =
            format!("POST /api/generate HTTP/1.1\r\nHost: m\r\nContent-Length: 40\r\n\r\n{body}");
        assert!(ask(&path, &post)?.starts_with("HTTP/1.1 200"));
        assert!(ask(&path, &post)?.starts_with("HTTP/1.1 200"));
        let third = ask(&path, &post)?;
        assert!(third.starts_with("HTTP/1.1 413 "), "{third}");
        assert!(third.contains("byte budget exhausted"), "{third}");
        let log = door.close();
        let fates: Vec<DoorFate> = log.iter().map(|r| r.fate).collect();
        assert_eq!(
            fates,
            vec![
                DoorFate::Forwarded,
                DoorFate::Forwarded,
                DoorFate::Refused(413)
            ]
        );
        assert_eq!(log[2].label, "refused");
        assert_eq!(log[2].bytes, 40);
        assert_eq!(log[2].sha256, Sha256Hex::digest(body.as_bytes()));
        Ok(())
    }

    #[test]
    fn over_cap_body_is_refused_with_bytes_actually_read() -> R {
        let (up, _rx) = mock("{}")?;
        let path = sock("cap");
        let budget = DoorBudget {
            max_body_bytes: 10,
            ..DoorBudget::DEFAULT
        };
        let door = serve(&path, up, budget)?;
        let raw = "POST /api/generate HTTP/1.1\r\nContent-Length: 50\r\n\r\n";
        let resp = ask(&path, raw)?;
        assert!(resp.starts_with("HTTP/1.1 413 "), "{resp}");
        let log = door.close();
        assert_eq!(log[0].bytes, raw.len() as u64);
        assert_eq!(log[0].sha256, Sha256Hex::digest(raw.as_bytes()));
        Ok(())
    }

    #[test]
    fn forwards_one_request_and_logs_body_digest() -> R {
        let (up, rx) = mock(r#"{"models":[]}"#)?;
        let path = sock("fwd");
        let door = serve(&path, up, DoorBudget::DEFAULT)?;
        let body = r#"{"model":"m","prompt":"p"}"#;
        let resp = ask(
            &path,
            &format!(
                "POST /api/generate HTTP/1.1\r\nHost: model\r\nConnection: keep-alive\r\nContent-Length: {}\r\n\r\n{body}",
                body.len()
            ),
        )?;
        assert!(resp.starts_with("HTTP/1.1 200 OK"), "{resp}");
        assert!(resp.ends_with(r#"{"models":[]}"#), "{resp}");
        let seen = rx.recv_timeout(Duration::from_secs(5))?;
        assert!(
            seen.starts_with("POST /api/generate HTTP/1.1\r\n"),
            "{seen}"
        );
        assert!(seen.contains("Connection: close\r\n"), "{seen}");
        assert!(!seen.contains("keep-alive"), "{seen}");
        assert!(seen.ends_with(body), "{seen}");
        let log = door.close();
        assert_eq!(
            log,
            vec![DoorRequest {
                bytes: body.len() as u64,
                sha256: Sha256Hex::digest(body.as_bytes()),
                fate: DoorFate::Forwarded,
                label: "forwarded",
                reason: "",
            }]
        );
        assert!(!path.exists(), "socket removed on close");
        Ok(())
    }

    #[test]
    fn refused_upstream_answers_503_json() -> R {
        let port = TcpListener::bind("127.0.0.1:0")?.local_addr()?.port();
        let up = Upstream::parse(&format!("http://127.0.0.1:{port}"))?;
        let path = sock("503");
        let door = serve(&path, up, DoorBudget::DEFAULT)?;
        let resp = ask(&path, "GET /api/tags HTTP/1.1\r\nHost: model\r\n\r\n")?;
        assert!(resp.starts_with("HTTP/1.1 503 "), "{resp}");
        let body = resp.split("\r\n\r\n").nth(1).ok_or("no body")?;
        let v: serde_json::Value = serde_json::from_str(body)?;
        assert_eq!(v, serde_json::json!({"refused": "model unreachable"}));
        assert_eq!(door.close()[0].fate, DoorFate::Unreachable);
        Ok(())
    }

    #[test]
    fn request_budget_answers_429() -> R {
        let (up, _rx) = mock("{}")?;
        let path = sock("429");
        let budget = DoorBudget {
            max_requests: 1,
            ..DoorBudget::DEFAULT
        };
        let door = serve(&path, up, budget)?;
        let get = "GET /api/tags HTTP/1.1\r\nHost: model\r\n\r\n";
        assert!(ask(&path, get)?.starts_with("HTTP/1.1 200"));
        assert!(ask(&path, get)?.starts_with("HTTP/1.1 429"));
        let fates: Vec<DoorFate> = door.close().iter().map(|r| r.fate).collect();
        assert_eq!(fates, vec![DoorFate::Forwarded, DoorFate::Refused(429)]);
        Ok(())
    }

    #[test]
    fn upstream_must_be_loopback_http() {
        assert!(Upstream::parse("http://127.0.0.1:11434").is_ok());
        assert!(Upstream::parse("http://[::1]:11434/").is_ok());
        assert!(matches!(
            Upstream::parse("http://10.0.0.1:11434"),
            Err(UpstreamRefusal::NotLoopback(_))
        ));
        assert!(matches!(
            Upstream::parse("https://127.0.0.1:11434"),
            Err(UpstreamRefusal::Malformed(_))
        ));
        assert!(matches!(
            Upstream::parse("http://localhost:11434"),
            Err(UpstreamRefusal::Malformed(_))
        ));
    }

    #[test]
    fn non_socket_at_path_is_refused() -> R {
        let path = sock("occupied");
        std::fs::write(&path, b"x")?;
        let up = Upstream::parse("http://127.0.0.1:9")?;
        let r = serve(&path, up, DoorBudget::DEFAULT);
        std::fs::remove_file(&path)?;
        assert!(matches!(r, Err(DoorError::Occupied(_))));
        Ok(())
    }
}
