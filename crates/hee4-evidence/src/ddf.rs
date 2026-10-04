//! The deep-diff-forge observation adapter: one `rank.v0` run over a diff becomes one tier-0
//! observation, sealed to the exact diff bytes. Local process only; no network.

use std::io::Write as _;
use std::path::Path;
use std::process::{Command, Stdio};

use hee4_contracts::{Evidence, Observation, Outcome, Sha256Hex, ToolId};
use hee4_host::clock::Clock;
use serde_json::Value;

use crate::decide::Subject;

/// The binary, looked up on `PATH`.
pub const BIN: &str = "deep-diff-forge";
/// The flags every gate run passes (`README.md`, "Gates should pass both" guards).
pub const ARGS: [&str; 5] = [
    "--stdin-patch",
    "--rank",
    "--json",
    "--require-files",
    "--require-hunks",
];
/// The schema the adapter admits.
pub const SCHEMA: &str = "deep-diff-forge.rank.v0";
/// Wall time allowed for one run, in milliseconds. Reported, not enforced (see `FLOW.md`).
pub const BUDGET_MS: u64 = 60_000;

/// Why no observation was produced. None of these is a pass.
#[derive(Debug, thiserror::Error)]
pub enum AdapterError {
    /// The process could not be started or waited on.
    #[error("could not run {BIN}: {0}")]
    Spawn(#[from] std::io::Error),
    /// Exit 7: the input contract refused the diff (0 files or 0 hunks).
    #[error("refused: {stderr_line}")]
    Refused {
        /// The first stderr line, as the tool printed it.
        stderr_line: String,
    },
    /// Any other non-zero exit.
    #[error("{BIN} exited {code:?}: {stderr_line}")]
    Exit {
        /// The exit code, if any.
        code: Option<i32>,
        /// The first stderr line.
        stderr_line: String,
    },
    /// Stdout was not a `rank.v0` document with the fields the adapter needs.
    #[error("malformed {SCHEMA} output: {0}")]
    Malformed(String),
    /// The tool sealed different bytes than the adapter sent.
    #[error("input_sha256 mismatch: sent {sent}, tool sealed {sealed}")]
    SealMismatch {
        /// Digest of the bytes written to stdin.
        sent: Sha256Hex,
        /// Digest the tool reported.
        sealed: Sha256Hex,
    },
    /// The ranking was empty: the run looked at nothing.
    #[error("rank.v0 ranked no files")]
    LookedAtNothing,
}

/// Run deep-diff-forge from `PATH` over `diff` and turn its ranking into an observation of
/// `subject`.
///
/// # Errors
/// [`AdapterError`]; exit 7 is [`AdapterError::Refused`].
pub fn observe(
    diff: &[u8],
    subject: &Subject,
    clock: &impl Clock,
) -> Result<Observation, AdapterError> {
    observe_with(Path::new(BIN), diff, subject, clock)
}

/// [`observe`] with an explicit binary.
///
/// # Errors
/// As [`observe`].
pub fn observe_with(
    bin: &Path,
    diff: &[u8],
    subject: &Subject,
    clock: &impl Clock,
) -> Result<Observation, AdapterError> {
    let started = clock.now();
    let mut child = Command::new(bin)
        .args(ARGS)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let stdin = child.stdin.take();
    let output = std::thread::scope(|s| {
        let writer = s.spawn(move || stdin.map_or(Ok(()), |mut w| w.write_all(diff)));
        let output = child.wait_with_output();
        // A tool that exits before reading all of stdin breaks the pipe; its exit code speaks.
        let _ = writer.join();
        output
    })?;
    let elapsed_ms =
        u64::try_from(clock.now().saturating_sub(started).as_millis()).unwrap_or(u64::MAX);
    let stderr_line = String::from_utf8_lossy(&output.stderr)
        .lines()
        .next()
        .unwrap_or("")
        .to_owned();
    match output.status.code() {
        Some(0) => {}
        Some(7) => return Err(AdapterError::Refused { stderr_line }),
        code => return Err(AdapterError::Exit { code, stderr_line }),
    }
    let doc: Value = serde_json::from_slice(&output.stdout)
        .map_err(|e| AdapterError::Malformed(e.to_string()))?;
    let field = |k: &str| {
        doc.get(k)
            .ok_or_else(|| AdapterError::Malformed(format!("missing `{k}`")))
    };
    let text = |v: &Value, k: &str| {
        v.get(k)
            .and_then(Value::as_str)
            .map(str::to_owned)
            .ok_or_else(|| AdapterError::Malformed(format!("missing string `{k}`")))
    };
    if text(&doc, "schema")? != SCHEMA {
        return Err(AdapterError::Malformed(format!("schema is not {SCHEMA}")));
    }
    let bad = |e: hee4_contracts::Refusal| AdapterError::Malformed(e.to_string());
    let sealed: Sha256Hex = text(&doc, "input_sha256")?.parse().map_err(bad)?;
    let sent = Sha256Hex::digest(diff);
    if sealed != sent {
        return Err(AdapterError::SealMismatch { sent, sealed });
    }
    let tool = field("tool")?;
    let tool = ToolId {
        name: text(tool, "name")?.parse().map_err(bad)?,
        version: text(tool, "version")?.parse().map_err(bad)?,
    };
    let ranked = field("ranked")?
        .as_array()
        .ok_or_else(|| AdapterError::Malformed("`ranked` is not an array".into()))?;
    if ranked.is_empty() {
        return Err(AdapterError::LookedAtNothing);
    }
    Ok(Observation {
        source: "deep-diff-forge.rank".parse().map_err(bad)?,
        input_sha256: sealed,
        tool,
        head_sha: subject.head_sha.clone(),
        outcome: Outcome::Pass,
        evidence: vec![Evidence {
            label: "rank.v0".parse().map_err(bad)?,
            sha256: Sha256Hex::digest(&output.stdout),
        }],
        advisory: false,
        elapsed_ms,
        budget_ms: BUDGET_MS,
    })
}
