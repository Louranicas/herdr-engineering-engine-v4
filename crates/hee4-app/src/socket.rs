//! The control socket: a 0700 directory, a 0600 socket, one JSON frame per line each way.

use hee4_contracts::Budgets;
use serde_json::Value;
use std::fs;
use std::io::{BufRead as _, BufReader, Read as _, Write as _};
use std::os::unix::fs::{MetadataExt as _, PermissionsExt as _};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::actions::{Engine, Outcome, answer};
use crate::wire::{self, Code, Fault};

/// Why the socket could not be bound.
#[derive(Debug, thiserror::Error)]
pub enum BindError {
    /// Another live server answers on the path (custody held).
    #[error("socket {0} is held by a live server")]
    Held(PathBuf),
    /// The directory belongs to another uid.
    #[error("socket dir {0} is not owned by this uid")]
    NotOwner(PathBuf),
    /// IO.
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}

/// How the peer is checked: the kernel's `SO_PEERCRED` uid, read through `rustix`'s safe
/// `getsockopt` wrapper, compared with this process's uid before any frame is read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PeerCheck {
    /// `SO_PEERCRED` uid equals the process uid, or the connection is refused `forbidden`.
    SoPeercred,
}

/// The peer check in force.
pub const PEER_CHECK: PeerCheck = PeerCheck::SoPeercred;

/// The comparison the peer check makes. `ours` is `None` when the process uid could not be
/// read; that refuses too (fail closed).
///
/// # Errors
/// [`Code::Forbidden`] at `/`.
pub fn peer_allowed(peer_uid: u32, ours: Option<u32>) -> Result<(), Fault> {
    if ours == Some(peer_uid) {
        Ok(())
    } else {
        Err(Fault::new(
            Code::Forbidden,
            "/",
            "peer uid is not the server's uid",
        ))
    }
}

/// A connection whose peer passed [`peer_allowed`]. The only way to build one is
/// [`Admitted::check`], so no frame is read from an unchecked peer.
#[derive(Debug)]
pub struct Admitted {
    stream: UnixStream,
    /// The peer's uid, from the kernel.
    pub uid: u32,
    /// The peer's pid, from the kernel.
    pub pid: i32,
}

impl Admitted {
    /// Read `SO_PEERCRED` and compare its uid with the process uid.
    ///
    /// # Errors
    /// `Err(Some(fault))` for a foreign uid; `Err(None)` when `getsockopt` itself failed.
    pub fn check(stream: UnixStream) -> Result<Self, (UnixStream, Option<Fault>)> {
        let Ok(cred) = rustix::net::sockopt::socket_peercred(&stream) else {
            return Err((stream, None));
        };
        let uid = cred.uid.as_raw();
        match peer_allowed(uid, crate::process_uid()) {
            Ok(()) => Ok(Self {
                stream,
                uid,
                pid: cred.pid.as_raw_pid(),
            }),
            Err(f) => Err((stream, Some(f))),
        }
    }
}

/// Bind `path`: create its directory 0700 (refusing one owned by another uid), replace a stale
/// socket only when nothing answers on it, bind, chmod 0600.
///
/// # Errors
/// [`BindError`].
pub fn bind(path: &Path) -> Result<UnixListener, BindError> {
    let dir = path.parent().unwrap_or(Path::new("/"));
    fs::create_dir_all(dir)?;
    if Some(fs::metadata(dir)?.uid()) != crate::process_uid() {
        return Err(BindError::NotOwner(dir.to_path_buf()));
    }
    fs::set_permissions(dir, fs::Permissions::from_mode(0o700))?;
    if path.exists() {
        if UnixStream::connect(path).is_ok() {
            return Err(BindError::Held(path.to_path_buf()));
        }
        fs::remove_file(path)?;
    }
    let listener = UnixListener::bind(path)?;
    fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;
    Ok(listener)
}

/// Frees one connection slot when the serving thread ends, however it ends.
struct Slot(Arc<AtomicUsize>);

impl Drop for Slot {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::AcqRel);
    }
}

/// Serve connections forever, one thread per connection, at most `socket.max_connections`
/// (the engine's budgets) at once; the next is refused `too_many_connections`, no thread.
pub fn serve(listener: &UnixListener, engine: &Arc<Engine>) {
    let budget = engine.budgets().socket;
    let cap = usize::try_from(budget.max_connections).unwrap_or(usize::MAX);
    let open = Arc::new(AtomicUsize::new(0));
    for conn in listener.incoming() {
        let Ok(mut stream) = conn else { continue };
        if open.load(Ordering::Acquire) >= cap {
            let fault = Fault::new(
                Code::TooManyConnections,
                "/",
                format!(
                    "{cap} connections are open (socket.max_connections={})",
                    budget.max_connections
                ),
            );
            let _ = stream.set_write_timeout(Some(budget.refusal_write()));
            let _ = stream.write_all(format!("{}\n", wire::error("", &fault)).as_bytes());
            continue;
        }
        open.fetch_add(1, Ordering::AcqRel);
        let slot = Slot(Arc::clone(&open));
        let engine = Arc::clone(engine);
        std::thread::spawn(move || {
            let _slot = slot;
            match Admitted::check(stream) {
                Ok(peer) => {
                    if let Err(e) = connection(peer, &engine) {
                        eprintln!("socket connection ended: {e}");
                    }
                }
                Err((mut stream, fault)) => {
                    let fault = fault.unwrap_or_else(|| {
                        Fault::new(Code::Forbidden, "/", "SO_PEERCRED unreadable")
                    });
                    eprintln!("socket refused peer: {}", fault.message);
                    let _ = stream.write_all(format!("{}\n", wire::error("", &fault)).as_bytes());
                }
            }
        });
    }
}

fn connection(peer: Admitted, engine: &Arc<Engine>) -> std::io::Result<()> {
    let budget = engine.budgets().socket;
    let stream = peer.stream;
    stream.set_read_timeout(Some(budget.read_deadline()))?;
    stream.set_write_timeout(Some(budget.write_deadline()))?;
    let mut writer = stream.try_clone()?;
    let mut reader = BufReader::new(stream);
    loop {
        let mut line = String::new();
        let n = (&mut reader)
            .take(budget.frame_bytes.saturating_add(1))
            .read_line(&mut line)?;
        if n == 0 {
            return Ok(());
        }
        if !line.ends_with('\n') {
            // EOF inside a record closes silently; a full-bound read with no LF is oversize:
            // refuse by name, then close. Neither is dispatched.
            if u64::try_from(n).unwrap_or(u64::MAX) > budget.frame_bytes {
                let fault = Fault::new(
                    Code::FrameTooLarge,
                    "/",
                    format!(
                        "request line over {} bytes (socket.frame_bytes)",
                        budget.frame_bytes
                    ),
                );
                writer.write_all(format!("{}\n", wire::error("", &fault)).as_bytes())?;
            }
            return Ok(());
        }
        match answer(engine, line.trim_end()) {
            Outcome::Frame(reply) => writer.write_all(format!("{reply}\n").as_bytes())?,
            Outcome::Subscribe { ack, since_seq } => {
                writer.write_all(format!("{ack}\n").as_bytes())?;
                // The connection is a stream from here on; requests after it are not read.
                return crate::stream::run(writer, Arc::clone(engine), since_seq);
            }
        }
    }
}

/// The CLI half: send one frame, read one reply.
///
/// # Errors
/// IO, or a reply that is not JSON (reported as an `internal` fault frame).
pub fn request(path: &Path, frame: &Value) -> std::io::Result<Value> {
    let mut stream = UnixStream::connect(path)?;
    // The CLI reads no budgets file: the contracts' default client read deadline.
    stream.set_read_timeout(Some(Budgets::DEFAULT.socket.client_read()))?;
    stream.write_all(format!("{frame}\n").as_bytes())?;
    let mut line = String::new();
    BufReader::new(stream).read_line(&mut line)?;
    Ok(serde_json::from_str(&line).unwrap_or_else(|_| {
        wire::error("", &Fault::new(Code::Internal, "/", "reply was not JSON"))
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn peer_uid_comparison_is_typed() {
        assert_eq!(peer_allowed(1000, Some(1000)), Ok(()));
        for (peer, ours) in [(0, Some(1000)), (1000, Some(0)), (1000, None)] {
            let refused = peer_allowed(peer, ours).map_err(|f| f.code);
            assert_eq!(refused, Err(Code::Forbidden), "peer={peer} ours={ours:?}");
        }
    }

    #[test]
    fn a_real_same_uid_connection_is_admitted() -> Result<(), Box<dyn std::error::Error>> {
        let dir = std::env::temp_dir().join(format!("hee4-peer-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let listener = bind(&dir.join("s.sock"))?;
        let client = UnixStream::connect(dir.join("s.sock"))?;
        let (server_side, _) = listener.accept()?;
        let peer = Admitted::check(server_side).map_err(|(_, f)| format!("refused: {f:?}"))?;
        assert_eq!(Some(peer.uid), crate::process_uid());
        assert_eq!(u32::try_from(peer.pid)?, std::process::id());
        drop(client);
        fs::remove_dir_all(&dir)?;
        Ok(())
    }
}
