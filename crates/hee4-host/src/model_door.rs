//! The model door: the candidate's only path to the model.
//!
//! The candidate runs with no network (`--unshare-net`). [`serve`] binds a unix socket on the
//! host; `spawn::plan` bind-mounts that socket into the namespace. Each connection carries one
//! HTTP/1.1 request, which the door forwards to a loopback TCP upstream (Ollama at
//! `127.0.0.1:11434`) with `Connection: close`, then relays the response. When the upstream does
//! not accept, the door answers `503 {"refused":"model unreachable"}`. Every request is logged as
//! a [`DoorRequest`] (body byte count and body sha256).

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::os::unix::fs::FileTypeExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread;
use std::time::Duration;

use hee4_contracts::Sha256Hex;

/// Largest header block the door reads.
const MAX_HEADER: usize = 64 * 1024;

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

/// Limits for one door.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DoorBudget {
    /// Requests forwarded before the door answers `429`.
    pub max_requests: u32,
    /// Largest request body, bytes; a larger one is answered `413`.
    pub max_body_bytes: usize,
    /// Read, write and connect timeout on each side.
    pub io_timeout: Duration,
}

impl DoorBudget {
    /// 64 requests, 1 MiB body, 120 s io. UNMEASURED stand-ins for K1 `budget`.
    pub const DEFAULT: Self = Self {
        max_requests: 64,
        max_body_bytes: 1024 * 1024,
        io_timeout: Duration::from_secs(120),
    };
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
    /// Request body length in bytes.
    pub bytes: u64,
    /// sha256 of the request body.
    pub sha256: Sha256Hex,
    /// What the door did with it.
    pub fate: DoorFate,
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
/// path is replaced; any other file there is refused. One request per connection, served in
/// order on one thread.
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
            .spawn(move || accept_loop(&listener, upstream, budget, &log, &stop))?
    };
    Ok(Door {
        path: socket_path.to_path_buf(),
        log,
        stop,
        worker: Some(worker),
    })
}

fn accept_loop(
    listener: &UnixListener,
    upstream: Upstream,
    budget: DoorBudget,
    log: &Mutex<Vec<DoorRequest>>,
    stop: &AtomicBool,
) {
    let mut forwarded = 0u32;
    for conn in listener.incoming() {
        if stop.load(Ordering::SeqCst) {
            return;
        }
        let Ok(client) = conn else { continue };
        let over = forwarded >= budget.max_requests;
        if let Some(req) = handle(client, upstream, budget, over) {
            if req.fate == DoorFate::Forwarded {
                forwarded += 1;
            }
            log.lock().unwrap_or_else(PoisonError::into_inner).push(req);
        }
    }
}

/// A parsed request: the head lines (request line first) and the body.
struct Request {
    head: Vec<String>,
    body: Vec<u8>,
}

/// Why a request could not be read; the status the door answers.
struct Bad(u16, &'static str);

fn read_request(s: &mut UnixStream, max_body: usize) -> Result<Request, Bad> {
    let mut buf = Vec::new();
    let mut chunk = [0u8; 8192];
    let end = loop {
        if let Some(i) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
            break i;
        }
        if buf.len() > MAX_HEADER {
            return Err(Bad(431, "header too large"));
        }
        match s.read(&mut chunk) {
            Ok(0) | Err(_) => return Err(Bad(400, "incomplete request")),
            Ok(n) => buf.extend_from_slice(&chunk[..n]),
        }
    };
    let head_text = std::str::from_utf8(&buf[..end]).map_err(|_| Bad(400, "header not utf-8"))?;
    let head: Vec<String> = head_text.split("\r\n").map(str::to_owned).collect();
    let mut len = 0usize;
    for h in &head[1..] {
        let Some((k, v)) = h.split_once(':') else {
            continue;
        };
        if k.eq_ignore_ascii_case("transfer-encoding") {
            return Err(Bad(
                411,
                "chunked bodies are not forwarded; send Content-Length",
            ));
        }
        if k.eq_ignore_ascii_case("content-length") {
            len = v
                .trim()
                .parse()
                .map_err(|_| Bad(400, "bad content-length"))?;
        }
    }
    if len > max_body {
        return Err(Bad(413, "body over budget"));
    }
    let mut body = buf[end + 4..].to_vec();
    while body.len() < len {
        match s.read(&mut chunk) {
            Ok(0) | Err(_) => return Err(Bad(400, "short body")),
            Ok(n) => body.extend_from_slice(&chunk[..n]),
        }
    }
    body.truncate(len);
    Ok(Request { head, body })
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

fn handle(
    mut client: UnixStream,
    upstream: Upstream,
    budget: DoorBudget,
    over: bool,
) -> Option<DoorRequest> {
    let _ = client.set_read_timeout(Some(budget.io_timeout));
    let _ = client.set_write_timeout(Some(budget.io_timeout));
    let req = match read_request(&mut client, budget.max_body_bytes) {
        Ok(r) => r,
        // A wake-up connect or a garbled request: nothing reached the model; not logged as one.
        Err(Bad(400, _)) => return None,
        Err(Bad(code, why)) => {
            answer(&mut client, code, "Refused", &refusal(why));
            return Some(DoorRequest {
                bytes: 0,
                sha256: Sha256Hex::digest(b""),
                fate: DoorFate::Refused(code),
            });
        }
    };
    let record = |fate| DoorRequest {
        bytes: req.body.len() as u64,
        sha256: Sha256Hex::digest(&req.body),
        fate,
    };
    if over {
        answer(
            &mut client,
            429,
            "Too Many Requests",
            &refusal("door request budget exhausted"),
        );
        return Some(record(DoorFate::Refused(429)));
    }
    let Ok(mut up) = TcpStream::connect_timeout(&upstream.addr(), budget.io_timeout) else {
        answer(
            &mut client,
            503,
            "Service Unavailable",
            &refusal("model unreachable"),
        );
        return Some(record(DoorFate::Unreachable));
    };
    let _ = up.set_read_timeout(Some(budget.io_timeout));
    let _ = up.set_write_timeout(Some(budget.io_timeout));
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
        answer(
            &mut client,
            503,
            "Service Unavailable",
            &refusal("model unreachable"),
        );
        return Some(record(DoorFate::Unreachable));
    }
    let _ = std::io::copy(&mut up, &mut client);
    Some(record(DoorFate::Forwarded))
}

#[cfg(test)]
mod tests {
    type R = Result<(), Box<dyn std::error::Error>>;
    use super::*;
    use std::net::TcpListener;
    use std::sync::mpsc;

    fn sock(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("hee4-door-{}-{name}.sock", std::process::id()))
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
