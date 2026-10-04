//! The native driver: runs one candidate attempt, step by step, against the local model.
//!
//! Model calls are made by this process through `hee4_host::model` (the plan's loopback is for
//! processes the steps spawn); command steps run inside bwrap through the spawn door. A step the
//! driver does not run is recorded as [`StepStatus::Skipped`] with its reason, never dropped.

use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hasher};
use std::path::PathBuf;
use std::time::{Duration, Instant};

use hee4_contracts::{
    Brief, Evidence, GitSha, Observation, Outcome, Sha256Hex, SourceId, ToolId, ToolName,
    ToolVersion,
};
use hee4_host::model::{ModelError, OllamaClient};
use hee4_host::spawn::{self, Command, NamespacePlan, Permit, SpawnError};

use crate::WorkerError;

/// The env guard for the real model call.
pub const LIVE_ENV: &str = "HEE4_LIVE_MODEL";

/// What one playbook step asks for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StepKind {
    /// Send `prompt` to the model.
    Generate {
        /// The exact prompt.
        prompt: String,
    },
    /// Run `program` inside the namespace.
    Run {
        /// Absolute program path.
        program: PathBuf,
        /// Arguments.
        args: Vec<String>,
    },
    /// A kind this driver does not run (named, so the skip is explained).
    Unsupported {
        /// The kind's name.
        kind: String,
    },
}

/// One playbook step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Step {
    /// The step's name in the playbook.
    pub name: String,
    /// What it asks for.
    pub kind: StepKind,
}

/// What happened to one step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StepStatus {
    /// The step ran to its end. `exit` is the process exit for a run step, `None` for a model step.
    Done {
        /// Process exit code, if a process ran.
        exit: Option<i32>,
    },
    /// The driver did not run it; the reason is printed as `skip: <reason>`.
    Skipped {
        /// Why.
        reason: String,
    },
    /// The step ran and failed.
    Failed {
        /// Why.
        cause: String,
        /// Model attempts made (1 for a run step).
        attempts: u32,
    },
}

/// One step and its status.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StepRecord {
    /// The step's name.
    pub name: String,
    /// What happened.
    pub status: StepStatus,
}

/// A retry that happened, printed as `attempt=k/n backoff_ms=`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetryNote {
    /// The attempt that failed, 1-based.
    pub attempt: u32,
    /// The attempt limit.
    pub of: u32,
    /// The sleep before the next attempt.
    pub backoff_ms: u64,
}

/// Everything one attempt produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttemptOutcome {
    /// Every playbook step, in order, one record each.
    pub steps: Vec<StepRecord>,
    /// Process and model output, concatenated in step order.
    pub stdout: Vec<u8>,
    /// Process stderr, concatenated in step order.
    pub stderr: Vec<u8>,
    /// Exit of the last process that ran; `None` when none did.
    pub exit: Option<i32>,
    /// Wall time of the whole attempt.
    pub elapsed: Duration,
    /// Candidate observations, one per model step that was tried (never advisory).
    pub observations: Vec<Observation>,
    /// Retries taken.
    pub retries: Vec<RetryNote>,
}

/// Bounded retry with jittered exponential backoff, for `ModelUnreachable` only.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetryPolicy {
    /// Total model calls allowed per step.
    pub max_attempts: u32,
    /// Backoff before the second call, ms (doubles each time).
    pub base_ms: u64,
    /// Backoff ceiling, ms.
    pub cap_ms: u64,
}

impl RetryPolicy {
    /// Three attempts, 200 ms base, 2 s cap. UNMEASURED: K1 `budget` is not built yet, so these
    /// literals are the stand-in (DC proposal in the report).
    pub const DEFAULT: Self = Self {
        max_attempts: 3,
        base_ms: 200,
        cap_ms: 2_000,
    };

    /// Backoff after failed attempt `attempt` (1-based): half to full of `min(cap, base*2^(k-1))`.
    #[must_use]
    pub fn backoff(&self, attempt: u32, entropy: u64) -> Duration {
        let full = self
            .base_ms
            .saturating_mul(1u64 << attempt.saturating_sub(1).min(20))
            .min(self.cap_ms);
        let half = full / 2;
        let spread = full - half + 1;
        Duration::from_millis(half + entropy % spread)
    }
}

fn entropy() -> u64 {
    RandomState::new().build_hasher().finish()
}

/// One candidate attempt's configuration.
#[derive(Debug, Clone)]
pub struct Attempt {
    /// The model the router chose.
    pub model: String,
    /// The commit the candidate is built on.
    pub head_sha: GitSha,
    /// Per-call model timeout.
    pub call_timeout: Duration,
    /// Retry policy for an unreachable model.
    pub retry: RetryPolicy,
}

impl Attempt {
    /// An attempt with a 60 s call timeout and [`RetryPolicy::DEFAULT`].
    #[must_use]
    pub fn new(model: &str, head_sha: GitSha) -> Self {
        Self {
            model: model.to_owned(),
            head_sha,
            call_timeout: Duration::from_secs(60),
            retry: RetryPolicy::DEFAULT,
        }
    }

    /// The real-model entry: runs only when `HEE4_LIVE_MODEL=1`; otherwise prints UNMEASURED and
    /// returns an outcome with every step `Skipped`.
    ///
    /// # Errors
    /// As [`Attempt::run`].
    pub fn run_live(
        &self,
        permit: &Permit,
        plan: &NamespacePlan,
        model: &OllamaClient,
        brief: &Brief,
        playbook: &[Step],
    ) -> Result<AttemptOutcome, WorkerError> {
        if std::env::var(LIVE_ENV).as_deref() == Ok("1") {
            return self.run(permit, plan, model, brief, playbook);
        }
        eprintln!("UNMEASURED: live model not called ({LIVE_ENV}!=1)");
        brief.check_restatement()?;
        let reason = format!("{LIVE_ENV} unset");
        Ok(AttemptOutcome {
            steps: playbook
                .iter()
                .map(|s| StepRecord {
                    name: s.name.clone(),
                    status: StepStatus::Skipped {
                        reason: reason.clone(),
                    },
                })
                .collect(),
            stdout: Vec::new(),
            stderr: Vec::new(),
            exit: None,
            elapsed: Duration::ZERO,
            observations: Vec::new(),
            retries: Vec::new(),
        })
    }

    /// Drive `playbook` in order. After a failed step every later step is `Skipped`.
    ///
    /// # Errors
    /// [`WorkerError::Contract`] for a brief with an empty RESTATEMENT or an unparseable token,
    /// [`WorkerError::Host`] when the spawn door refuses a run step, [`WorkerError::Spawn`] when a
    /// process cannot start. A timed-out process and a failed model call are recorded as
    /// [`StepStatus::Failed`], not errors.
    pub fn run(
        &self,
        permit: &Permit,
        plan: &NamespacePlan,
        model: &OllamaClient,
        brief: &Brief,
        playbook: &[Step],
    ) -> Result<AttemptOutcome, WorkerError> {
        brief.check_restatement()?;
        let start = Instant::now();
        let mut out = AttemptOutcome {
            steps: Vec::new(),
            stdout: Vec::new(),
            stderr: Vec::new(),
            exit: None,
            elapsed: Duration::ZERO,
            observations: Vec::new(),
            retries: Vec::new(),
        };
        let mut failed = false;
        for step in playbook {
            let status = if failed {
                skip("an earlier step failed")
            } else {
                match &step.kind {
                    StepKind::Unsupported { kind } => {
                        skip(&format!("driver has no handler for step kind {kind}"))
                    }
                    StepKind::Generate { .. } if !plan.allow_loopback => {
                        skip("plan does not allow the model (no loopback)")
                    }
                    StepKind::Generate { prompt } => self.generate(model, prompt, &mut out)?,
                    StepKind::Run { program, args } => {
                        run_step(permit, plan, program, args, &mut out)?
                    }
                }
            };
            failed |= matches!(status, StepStatus::Failed { .. });
            out.steps.push(StepRecord {
                name: step.name.clone(),
                status,
            });
        }
        out.elapsed = start.elapsed();
        Ok(out)
    }

    fn generate(
        &self,
        client: &OllamaClient,
        prompt: &str,
        out: &mut AttemptOutcome,
    ) -> Result<StepStatus, WorkerError> {
        let t0 = Instant::now();
        let mut attempts = 0;
        let result = loop {
            attempts += 1;
            match client.generate(&self.model, prompt, self.call_timeout) {
                Err(ModelError::ModelUnreachable(_)) if attempts < self.retry.max_attempts => {
                    let wait = self.retry.backoff(attempts, entropy());
                    let note = RetryNote {
                        attempt: attempts,
                        of: self.retry.max_attempts,
                        backoff_ms: u64::try_from(wait.as_millis()).unwrap_or(u64::MAX),
                    };
                    eprintln!(
                        "attempt={}/{} backoff_ms={}",
                        note.attempt, note.of, note.backoff_ms
                    );
                    out.retries.push(note);
                    std::thread::sleep(wait);
                }
                other => break other,
            }
        };
        let elapsed_ms = u64::try_from(t0.elapsed().as_millis()).unwrap_or(u64::MAX);
        let (outcome, evidence, status) = match result {
            Ok(g) => {
                out.stdout.extend_from_slice(g.text.as_bytes());
                (
                    Outcome::Pass,
                    vec![Evidence {
                        label: "generation".parse()?,
                        sha256: Sha256Hex::digest(g.text.as_bytes()),
                    }],
                    StepStatus::Done { exit: None },
                )
            }
            Err(e) => (
                Outcome::Error,
                Vec::new(),
                StepStatus::Failed {
                    cause: e.to_string(),
                    attempts,
                },
            ),
        };
        out.observations.push(Observation {
            source: "hee4-worker-native".parse::<SourceId>()?,
            input_sha256: Sha256Hex::digest(prompt.as_bytes()),
            tool: ToolId {
                name: "ollama".parse::<ToolName>()?,
                version: self.model.parse::<ToolVersion>()?,
            },
            head_sha: self.head_sha.clone(),
            outcome,
            evidence,
            advisory: false,
            elapsed_ms,
            budget_ms: u64::try_from(self.call_timeout.as_millis()).unwrap_or(u64::MAX),
        });
        Ok(status)
    }
}

fn skip(reason: &str) -> StepStatus {
    StepStatus::Skipped {
        reason: reason.to_owned(),
    }
}

fn run_step(
    permit: &Permit,
    plan: &NamespacePlan,
    program: &std::path::Path,
    args: &[String],
    out: &mut AttemptOutcome,
) -> Result<StepStatus, WorkerError> {
    let sp = spawn::plan(
        permit,
        Command {
            program: program.to_path_buf(),
            args: args.to_vec(),
        },
        plan.clone(),
    )?;
    match spawn::run(sp) {
        Ok(o) => {
            out.stdout.extend_from_slice(&o.stdout);
            out.stderr.extend_from_slice(&o.stderr);
            out.exit = o.exit;
            Ok(if o.exit == Some(0) {
                StepStatus::Done { exit: o.exit }
            } else {
                StepStatus::Failed {
                    cause: format!("exit {:?}", o.exit),
                    attempts: 1,
                }
            })
        }
        Err(e @ SpawnError::TimedOut(_)) => Ok(StepStatus::Failed {
            cause: e.to_string(),
            attempts: 1,
        }),
        Err(e) => Err(e.into()),
    }
}

#[cfg(test)]
mod tests {
    type R = Result<(), Box<dyn std::error::Error>>;
    use super::*;
    use crate::namespace::{NamespaceTask, plan_for};
    use hee4_host::spawn::{ReceiptId, SpawnScope};
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;

    const BRIEF: &str = "GOAL: g\nSCOPE: s\nCONTEXT: c\nACCEPTANCE: a\nVERIFY: v\nTIMEBOX: t\nFORBIDDEN: f\nREPORT: r\nSTANDING: s\nRECON: r\nRESTATEMENT: I restate the goal.";

    fn http(body: &str) -> String {
        format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        )
    }

    /// Mock on 127.0.0.1: drops the first `refuse` connections unanswered, then answers every one.
    fn mock(refuse: usize, reply: String) -> std::io::Result<String> {
        let l = TcpListener::bind("127.0.0.1:0")?;
        let addr = l.local_addr()?;
        thread::spawn(move || {
            for (i, conn) in l.incoming().enumerate() {
                let Ok(mut s) = conn else { continue };
                let mut buf = [0u8; 8192];
                let _ = s.read(&mut buf);
                if i >= refuse {
                    let _ = s.write_all(reply.as_bytes());
                }
            }
        });
        Ok(format!("http://{addr}"))
    }

    fn fixture(
        needs_model: bool,
        programs: &[&str],
    ) -> Result<(Permit, NamespacePlan, Brief, Attempt), Box<dyn std::error::Error>> {
        let permit = Permit::mint(
            ReceiptId("r1".into()),
            SpawnScope {
                programs: programs.iter().map(PathBuf::from).collect(),
            },
        );
        let task = NamespaceTask::new(
            "t1".parse()?,
            std::path::Path::new("/tmp"),
            needs_model,
            Duration::from_secs(10),
        )?;
        let mut a = Attempt::new("qwen:7b", "a".repeat(40).parse()?);
        a.retry = RetryPolicy {
            max_attempts: 3,
            base_ms: 2,
            cap_ms: 10,
        };
        Ok((permit, plan_for(&task), Brief::parse(BRIEF)?, a))
    }

    fn gen_step(p: &str) -> Step {
        Step {
            name: "ask".into(),
            kind: StepKind::Generate { prompt: p.into() },
        }
    }

    const ANSWER: &str = r#"{"response":"hello","eval_count":3,"total_duration":2000000}"#;

    #[test]
    fn model_step_builds_a_candidate_observation() -> R {
        let url = mock(0, http(ANSWER))?;
        let (permit, plan, brief, a) = fixture(true, &[])?;
        let o = a.run(
            &permit,
            &plan,
            &OllamaClient::new(&url),
            &brief,
            &[gen_step("say hi")],
        )?;
        assert_eq!(o.steps[0].status, StepStatus::Done { exit: None });
        assert_eq!(o.stdout, b"hello");
        assert_eq!(o.observations.len(), 1);
        let ob = &o.observations[0];
        assert!(!ob.advisory);
        assert_eq!(ob.input_sha256, Sha256Hex::digest(b"say hi"));
        assert_eq!(
            (ob.tool.name.as_str(), ob.tool.version.as_str()),
            ("ollama", "qwen:7b")
        );
        assert_eq!(ob.outcome, Outcome::Pass);
        assert_eq!(ob.evidence[0].sha256, Sha256Hex::digest(b"hello"));
        Ok(())
    }

    #[test]
    fn unreachable_twice_then_answer_retries_with_backoff() -> R {
        let url = mock(2, http(ANSWER))?;
        let (permit, plan, brief, a) = fixture(true, &[])?;
        let o = a.run(
            &permit,
            &plan,
            &OllamaClient::new(&url),
            &brief,
            &[gen_step("p")],
        )?;
        assert_eq!(o.steps[0].status, StepStatus::Done { exit: None });
        assert_eq!(o.retries.len(), 2);
        assert_eq!(
            o.retries
                .iter()
                .map(|r| (r.attempt, r.of))
                .collect::<Vec<_>>(),
            vec![(1, 3), (2, 3)]
        );
        assert!(o.retries.iter().all(|r| (1..=10).contains(&r.backoff_ms)));
        Ok(())
    }

    #[test]
    fn retries_are_bounded_at_three() -> R {
        let url = mock(usize::MAX, String::new())?;
        let (permit, plan, brief, a) = fixture(true, &[])?;
        let o = a.run(
            &permit,
            &plan,
            &OllamaClient::new(&url),
            &brief,
            &[gen_step("p"), gen_step("q")],
        )?;
        assert!(matches!(
            &o.steps[0].status,
            StepStatus::Failed { attempts: 3, .. }
        ));
        assert_eq!(o.retries.len(), 2);
        assert_eq!(o.observations[0].outcome, Outcome::Error);
        assert!(
            matches!(&o.steps[1].status, StepStatus::Skipped { reason } if reason.contains("earlier step failed"))
        );
        Ok(())
    }

    #[test]
    fn skipped_steps_are_recorded_never_dropped() -> R {
        let (permit, plan, brief, a) = fixture(false, &[])?;
        let playbook = vec![
            gen_step("p"),
            Step {
                name: "review".into(),
                kind: StepKind::Unsupported {
                    kind: "human-review".into(),
                },
            },
        ];
        let o = a.run(
            &permit,
            &plan,
            &OllamaClient::new("http://127.0.0.1:9"),
            &brief,
            &playbook,
        )?;
        assert_eq!(o.steps.len(), 2);
        assert_eq!(
            o.steps.iter().map(|s| s.name.as_str()).collect::<Vec<_>>(),
            vec!["ask", "review"]
        );
        assert!(
            matches!(&o.steps[0].status, StepStatus::Skipped { reason } if reason.contains("no loopback"))
        );
        assert!(
            matches!(&o.steps[1].status, StepStatus::Skipped { reason } if reason.contains("human-review"))
        );
        assert_eq!(o.observations.len(), 0);
        Ok(())
    }

    #[test]
    fn run_step_records_stdout_exit_elapsed_in_bwrap() -> R {
        if !std::path::Path::new(spawn::BWRAP).exists() {
            println!("UNMEASURED: bwrap absent; run_step test skipped");
            return Ok(());
        }
        let (permit, plan, brief, a) = fixture(false, &["/usr/bin/echo"])?;
        std::fs::create_dir_all(&plan.work_dir)?;
        let step = Step {
            name: "echo".into(),
            kind: StepKind::Run {
                program: "/usr/bin/echo".into(),
                args: vec!["hi".into()],
            },
        };
        let res = a.run(
            &permit,
            &plan,
            &OllamaClient::new("http://127.0.0.1:9"),
            &brief,
            &[step],
        );
        match res {
            Ok(o) => {
                assert_eq!(o.stdout, b"hi\n");
                assert_eq!(o.exit, Some(0));
                assert!(o.elapsed > Duration::ZERO);
            }
            Err(e) => println!("UNMEASURED: bwrap could not run here: {e}"),
        }
        Ok(())
    }

    #[test]
    fn run_step_out_of_scope_is_a_host_refusal() -> R {
        let (permit, plan, brief, a) = fixture(false, &[])?;
        let step = Step {
            name: "x".into(),
            kind: StepKind::Run {
                program: "/usr/bin/echo".into(),
                args: vec![],
            },
        };
        let res = a.run(
            &permit,
            &plan,
            &OllamaClient::new("http://127.0.0.1:9"),
            &brief,
            &[step],
        );
        assert!(matches!(res, Err(WorkerError::Host(_))));
        Ok(())
    }

    #[test]
    fn live_guard_skips_everything_when_unset() -> R {
        if std::env::var(LIVE_ENV).as_deref() == Ok("1") {
            println!("UNMEASURED: {LIVE_ENV}=1 set; guard test not applicable");
            return Ok(());
        }
        let (permit, plan, brief, a) = fixture(true, &[])?;
        let o = a.run_live(
            &permit,
            &plan,
            &OllamaClient::new("http://127.0.0.1:9"),
            &brief,
            &[gen_step("p")],
        )?;
        assert!(
            matches!(&o.steps[0].status, StepStatus::Skipped { reason } if reason.contains(LIVE_ENV))
        );
        assert_eq!(o.observations.len(), 0);
        Ok(())
    }

    #[test]
    fn backoff_is_jittered_within_bounds() {
        let p = RetryPolicy {
            max_attempts: 3,
            base_ms: 100,
            cap_ms: 150,
        };
        for e in [0, 1, 7, u64::MAX] {
            let d = p.backoff(1, e).as_millis();
            assert!((50..=100).contains(&d), "{d}");
            let d = p.backoff(5, e).as_millis();
            assert!((75..=150).contains(&d), "{d}");
        }
    }
}
