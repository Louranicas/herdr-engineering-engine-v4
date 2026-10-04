//! The control socket: a 0700 directory, a 0600 socket, one JSON frame per line each way.

use std::fs;
use std::io::{BufRead as _, BufReader, Read as _, Write as _};
use std::os::unix::fs::{MetadataExt as _, PermissionsExt as _};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use serde_json::Value;

use crate::actions::{Engine, handle};
use crate::wire::{self, Code, Fault, MAX_FRAME_BYTES};

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

/// How the peer was checked. `SO_PEERCRED` needs `getsockopt` (unsafe, forbidden here) or the
/// unstable `UnixStream::peer_cred`; the skeleton relies on the 0700 dir and says so.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PeerCheck {
    /// Not measured: no safe `SO_PEERCRED` reader is available to this crate.
    Unmeasured,
}

/// The peer check in force.
pub const PEER_CHECK: PeerCheck = PeerCheck::Unmeasured;

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

/// Serve connections forever, one thread per connection.
pub fn serve(listener: &UnixListener, engine: &Arc<Engine>) {
    for conn in listener.incoming() {
        let Ok(stream) = conn else { continue };
        let engine = Arc::clone(engine);
        std::thread::spawn(move || {
            if let Err(e) = connection(stream, &engine) {
                eprintln!("socket connection ended: {e}");
            }
        });
    }
}

fn connection(stream: UnixStream, engine: &Engine) -> std::io::Result<()> {
    stream.set_read_timeout(Some(Duration::from_secs(60)))?;
    stream.set_write_timeout(Some(Duration::from_secs(10)))?;
    let mut writer = stream.try_clone()?;
    let mut reader = BufReader::new(stream);
    loop {
        let mut line = String::new();
        let n = (&mut reader)
            .take(MAX_FRAME_BYTES as u64 + 1)
            .read_line(&mut line)?;
        if n == 0 {
            return Ok(());
        }
        if !line.ends_with('\n') {
            // Oversize or EOF inside a record: close without dispatch (Socket and IPC Map).
            return Ok(());
        }
        let reply = handle(engine, line.trim_end());
        writer.write_all(format!("{reply}\n").as_bytes())?;
    }
}

/// The CLI half: send one frame, read one reply.
///
/// # Errors
/// IO, or a reply that is not JSON (reported as an `internal` fault frame).
pub fn request(path: &Path, frame: &Value) -> std::io::Result<Value> {
    let mut stream = UnixStream::connect(path)?;
    stream.set_read_timeout(Some(Duration::from_secs(30)))?;
    stream.write_all(format!("{frame}\n").as_bytes())?;
    let mut line = String::new();
    BufReader::new(stream).read_line(&mut line)?;
    Ok(serde_json::from_str(&line).unwrap_or_else(|_| {
        wire::error("", &Fault::new(Code::Internal, "/", "reply was not JSON"))
    }))
}
