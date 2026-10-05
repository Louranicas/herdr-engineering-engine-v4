//! The deep-diff-forge observation adapter: one `rank.v0` run over a diff becomes one tier-0
//! observation, sealed to the exact diff bytes. Local process only; no network.

use std::fmt;
use std::io::{Read as _, Write as _};
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use hee4_contracts::{Evidence, Observation, Outcome, RefusalText, Sha256Hex, ToolId};
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
/// How often the deadline loop looks at the child.
const POLL: Duration = Duration::from_millis(10);

/// Why no observation was produced. None of these is a pass.
#[derive(Debug, thiserror::Error)]
pub enum AdapterError {
    /// The process could not be started or waited on.
    #[error("could not run {BIN}: {0}")]
    Spawn(#[from] std::io::Error),
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
    /// The deadline passed; the child was killed and reaped. Map it with [`timeout_observation`].
    #[error("{BIN} ran past its {budget:?} budget and was killed")]
    Timeout {
        /// The budget the caller passed.
        budget: Duration,
    },
    /// The ranking was empty: the run looked at nothing.
    #[error("rank.v0 ranked no files")]
    LookedAtNothing,
}

/// What the caller hands in for one task: the diff bytes it computed, or the fact that there
/// was no worktree to diff. This crate never runs git; the bytes are the caller's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Diff<'a> {
    /// The task has no worktree, so there is nothing to diff.
    NoWorktree,
    /// The diff the caller computed; empty bytes mean the worktree had no change.
    Bytes(&'a [u8]),
}

/// Why [`for_task`] produced no observation. A skip is not a refusal: the task goes on
/// without a deep-diff-forge observation, and the dispatcher prints the wire word.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Skip {
    /// [`Diff::NoWorktree`]: nothing to diff.
    NoWorktree,
    /// [`Diff::Bytes`] was empty: an empty diff never reaches `--require-files`.
    NoDiff,
    /// The binary was not found (`io::ErrorKind::NotFound` at spawn). A present-but-broken
    /// tool is not absent: every other spawn error stays [`AdapterError::Spawn`].
    ToolAbsent,
}

impl Skip {
    /// The wire word: `no_worktree`, `no_diff` or `tool_absent`.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::NoWorktree => "no_worktree",
            Self::NoDiff => "no_diff",
            Self::ToolAbsent => "tool_absent",
        }
    }
}

impl fmt::Display for Skip {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// What [`for_task`] hands the dispatcher: an observation to record, or a named skip.
/// Never a verdict (AP-29).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskObservation {
    /// A tier-0 observation bound to the subject; record it before `decide_and_seal`.
    Observed(Observation),
    /// No observation, by name.
    Skipped(Skip),
}

/// The one task-shaped call: deep-diff-forge from `PATH` over `diff` for `subject`.
///
/// - [`Diff::NoWorktree`] → `Skipped(NoWorktree)`, empty bytes → `Skipped(NoDiff)`; neither
///   spawns;
/// - a binary not found → `Skipped(ToolAbsent)`;
/// - a run past `budget` → `Observed` with [`timeout_observation`] (`Refused(timeout)` in
///   `decide`);
/// - exit 7 → `Observed` with [`Outcome::Refused`]; a pass → `Observed`, bound to
///   `subject.input_sha256`.
///
/// # Errors
/// Every other [`AdapterError`] unchanged: `Exit`, `Malformed`, `SealMismatch`,
/// `LookedAtNothing`, and a `Spawn` whose kind is not `NotFound`.
pub fn for_task(
    diff: Diff<'_>,
    subject: &Subject,
    clock: &impl Clock,
    budget: Duration,
) -> Result<TaskObservation, AdapterError> {
    for_task_with(Path::new(BIN), diff, subject, clock, budget)
}

/// [`for_task`] with an explicit binary.
///
/// # Errors
/// As [`for_task`].
pub fn for_task_with(
    bin: &Path,
    diff: Diff<'_>,
    subject: &Subject,
    clock: &impl Clock,
    budget: Duration,
) -> Result<TaskObservation, AdapterError> {
    match diff {
        Diff::NoWorktree => Ok(TaskObservation::Skipped(Skip::NoWorktree)),
        Diff::Bytes([]) => Ok(TaskObservation::Skipped(Skip::NoDiff)),
        Diff::Bytes(b) => match observe_with(bin, b, subject, clock, budget) {
            Ok(o) => Ok(TaskObservation::Observed(o)),
            Err(AdapterError::Spawn(e)) if e.kind() == std::io::ErrorKind::NotFound => {
                Ok(TaskObservation::Skipped(Skip::ToolAbsent))
            }
            Err(AdapterError::Timeout { budget }) => Ok(TaskObservation::Observed(
                timeout_observation(budget, b, subject)?,
            )),
            Err(e) => Err(e),
        },
    }
}

/// Run deep-diff-forge from `PATH` over `diff` and turn its ranking into an observation of
/// `subject`. The child is killed when `budget` of wall time has passed.
///
/// # Errors
/// [`AdapterError`]; a run past `budget` is [`AdapterError::Timeout`]. Exit 7 is not an error:
/// it is a tier-0 observation with [`Outcome::Refused`].
pub fn observe(
    diff: &[u8],
    subject: &Subject,
    clock: &impl Clock,
    budget: Duration,
) -> Result<Observation, AdapterError> {
    observe_with(Path::new(BIN), diff, subject, clock, budget)
}

/// The tier-0 observation for a run that hit its deadline: outcome `error`, `elapsed_ms` one
/// past `budget_ms`, one evidence item (the digest of the diff that was sent). `decide` reads
/// the elapsed-over-budget row first, so this yields `Refused(timeout)` and never a pass.
///
/// # Errors
/// [`AdapterError::Malformed`] if a fixed token fails to parse (a bug, not an input).
pub fn timeout_observation(
    budget: Duration,
    diff: &[u8],
    subject: &Subject,
) -> Result<Observation, AdapterError> {
    let bad = |e: hee4_contracts::Refusal| AdapterError::Malformed(e.to_string());
    let budget_ms = u64::try_from(budget.as_millis()).unwrap_or(u64::MAX);
    Ok(Observation {
        source: "deep-diff-forge.rank".parse().map_err(bad)?,
        input_sha256: subject.input_sha256,
        tool: ToolId {
            name: "deep-diff-forge".parse().map_err(bad)?,
            version: "unknown".parse().map_err(bad)?,
        },
        head_sha: subject.head_sha.clone(),
        outcome: Outcome::Error,
        evidence: vec![Evidence {
            label: "deadline".parse().map_err(bad)?,
            sha256: Sha256Hex::digest(diff),
        }],
        advisory: false,
        elapsed_ms: budget_ms.saturating_add(1),
        budget_ms,
    })
}

/// The longest reason kept, below `RefusalText`'s 512-byte cap.
const REASON_MAX: usize = 500;

/// The tier-0 observation for exit 7: outcome `Refused{reason}` (the first stderr line, control
/// characters dropped, cut to [`REASON_MAX`] bytes; a fixed sentence when nothing is left), one
/// evidence item (the digest of all of stderr). `decide` maps it to `Refused(Invalid)`.
fn refused_observation(
    stderr_line: &str,
    stderr: &[u8],
    subject: &Subject,
    elapsed_ms: u64,
    budget: Duration,
) -> Result<Observation, AdapterError> {
    let bad = |e: hee4_contracts::Refusal| AdapterError::Malformed(e.to_string());
    let mut reason: String = stderr_line.chars().filter(|c| !c.is_control()).collect();
    while reason.len() > REASON_MAX {
        reason.pop();
    }
    if reason.trim().is_empty() {
        reason = format!("{BIN} exited 7 without a message");
    }
    let reason: RefusalText = reason.parse().map_err(bad)?;
    Ok(Observation {
        source: "deep-diff-forge.rank".parse().map_err(bad)?,
        input_sha256: subject.input_sha256,
        tool: ToolId {
            name: BIN.parse().map_err(bad)?,
            version: "unknown".parse().map_err(bad)?,
        },
        head_sha: subject.head_sha.clone(),
        outcome: Outcome::Refused { reason },
        evidence: vec![Evidence {
            label: "refusal".parse().map_err(bad)?,
            sha256: Sha256Hex::digest(stderr),
        }],
        advisory: false,
        elapsed_ms,
        budget_ms: u64::try_from(budget.as_millis()).unwrap_or(u64::MAX),
    })
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
    budget: Duration,
) -> Result<Observation, AdapterError> {
    let started = clock.now();
    let mut child = Command::new(bin)
        .args(ARGS)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    // Owned, detached threads: a grandchild that keeps a pipe open must not hold us past the
    // deadline. Draining while we poll keeps a chatty child from blocking on a full pipe.
    let stdin = child.stdin.take();
    let input = diff.to_vec();
    let writer = std::thread::spawn(move || stdin.map_or(Ok(()), |mut w| w.write_all(&input)));
    let drain = |pipe: Option<Box<dyn std::io::Read + Send>>| {
        std::thread::spawn(move || {
            let mut buf = Vec::new();
            if let Some(mut p) = pipe {
                let _ = p.read_to_end(&mut buf);
            }
            buf
        })
    };
    let out_t = drain(child.stdout.take().map(|p| Box::new(p) as _));
    let err_t = drain(child.stderr.take().map(|p| Box::new(p) as _));
    let deadline = Instant::now() + budget;
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break status;
        }
        if Instant::now() >= deadline {
            child.kill()?;
            child.wait()?;
            return Err(AdapterError::Timeout { budget });
        }
        std::thread::sleep(POLL);
    };
    // A tool that exits before reading all of stdin breaks the pipe; its exit code speaks.
    let _ = writer.join();
    let output = std::process::Output {
        status,
        stdout: out_t.join().unwrap_or_default(),
        stderr: err_t.join().unwrap_or_default(),
    };
    let elapsed_ms =
        u64::try_from(clock.now().saturating_sub(started).as_millis()).unwrap_or(u64::MAX);
    let stderr_line = String::from_utf8_lossy(&output.stderr)
        .lines()
        .next()
        .unwrap_or("")
        .to_owned();
    match output.status.code() {
        Some(0) => {}
        Some(7) => {
            return refused_observation(&stderr_line, &output.stderr, subject, elapsed_ms, budget);
        }
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
    pass_observation(tool, &output.stdout, sealed, subject, elapsed_ms, budget)
}

/// The tier-0 Pass observation for a sealed, non-empty ranking. It is bound to the subject's
/// `input_sha256` (the VERIFY digest, V4-94) so `decide` reconciles it; the diff the tool
/// sealed is evidence (`diff`, checked equal to the bytes sent by the caller of this function),
/// beside `rank.v0` = the digest of stdout.
///
/// # Errors
/// [`AdapterError::Malformed`] if a fixed token fails to parse (a bug, not an input).
fn pass_observation(
    tool: ToolId,
    stdout: &[u8],
    sealed: Sha256Hex,
    subject: &Subject,
    elapsed_ms: u64,
    budget: Duration,
) -> Result<Observation, AdapterError> {
    let bad = |e: hee4_contracts::Refusal| AdapterError::Malformed(e.to_string());
    Ok(Observation {
        source: "deep-diff-forge.rank".parse().map_err(bad)?,
        input_sha256: subject.input_sha256,
        tool,
        head_sha: subject.head_sha.clone(),
        outcome: Outcome::Pass,
        evidence: vec![
            Evidence {
                label: "rank.v0".parse().map_err(bad)?,
                sha256: Sha256Hex::digest(stdout),
            },
            Evidence {
                label: "diff".parse().map_err(bad)?,
                sha256: sealed,
            },
        ],
        advisory: false,
        elapsed_ms,
        budget_ms: u64::try_from(budget.as_millis()).unwrap_or(u64::MAX),
    })
}
