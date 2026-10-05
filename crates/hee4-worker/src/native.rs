//! The native driver: runs one candidate attempt, step by step.
//!
//! This process makes no model call. Command steps run inside bwrap with no network; when the
//! plan carries a model door, [`Attempt::run`] serves it for the whole attempt
//! (`hee4_host::model_door`), the candidate finds it at `$HEE4_MODEL_SOCKET`, and every request
//! through it becomes `model_request` evidence on one observation. A step the driver does not run
//! is recorded as [`StepStatus::Skipped`] with its reason, never dropped.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use hee4_contracts::{
    Brief, BriefField, Evidence, GitSha, Observation, Outcome, Sha256Hex, SourceId, ToolId,
    ToolName, ToolVersion,
};
use hee4_host::model_door::{self, DoorBudget, DoorFate, DoorRequest, Upstream};
use hee4_host::spawn::{self, Command, NamespacePlan, Permit, SpawnError};

use crate::WorkerError;

/// The env guard for a live attempt (one whose candidate may reach the real model).
pub const LIVE_ENV: &str = "HEE4_LIVE_MODEL";

/// What one playbook step asks for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StepKind {
    /// Run `program` inside the namespace.
    Run {
        /// Absolute program path.
        program: PathBuf,
        /// Arguments.
        args: Vec<String>,
    },
    /// A kind this driver does not run (named, so the skip is explained). A `model:` line is one
    /// of these (`kind: "model"`): the driver makes no model call; a `Run` step's own `sh:`
    /// command talks to the model through the door.
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
    /// The step ran to its end. `exit` is the process exit.
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
        /// Process attempts made (always 1).
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

/// Everything one attempt produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttemptOutcome {
    /// Every playbook step, in order, one record each.
    pub steps: Vec<StepRecord>,
    /// Process stdout, concatenated in step order.
    pub stdout: Vec<u8>,
    /// Process stderr, concatenated in step order.
    pub stderr: Vec<u8>,
    /// Exit of the last process that ran; `None` when none did.
    pub exit: Option<i32>,
    /// Wall time of the whole attempt.
    pub elapsed: Duration,
    /// Candidate observations: one for the model door when any request went through it (never
    /// advisory); none otherwise, since a source that saw nothing is not evidence.
    pub observations: Vec<Observation>,
    /// Every request the door saw, with its body byte count (the contracts' `Evidence` has no
    /// byte field; DC proposal).
    pub model_requests: Vec<DoorRequest>,
}

/// One candidate attempt's configuration.
#[derive(Debug, Clone)]
pub struct Attempt {
    /// The model the router chose.
    pub model: String,
    /// The commit the candidate is built on.
    pub head_sha: GitSha,
    /// Limits on the model door.
    pub door_budget: DoorBudget,
}

impl Attempt {
    /// An attempt with [`DoorBudget::DEFAULT`]: [`Attempt::with_budget`] with the contracts'
    /// default door.
    #[must_use]
    pub fn new(model: &str, head_sha: GitSha) -> Self {
        Self::with_budget(model, head_sha, DoorBudget::DEFAULT)
    }

    /// An attempt whose model door runs under `door` (a `hee4_contracts::DoorBudget`, handed
    /// down by the loader). Not checked here: `run` refuses a budget that fails the contracts'
    /// `door.*` checks through `model_door::serve` (`DoorError::Budget`).
    #[must_use]
    pub fn with_budget(model: &str, head_sha: GitSha, door: DoorBudget) -> Self {
        Self {
            model: model.to_owned(),
            head_sha,
            door_budget: door,
        }
    }

    /// The live entry: runs only when `HEE4_LIVE_MODEL=1`; otherwise prints UNMEASURED and
    /// returns an outcome with every step `Skipped`.
    ///
    /// # Errors
    /// As [`Attempt::run`].
    pub fn run_live(
        &self,
        permit: &Permit,
        plan: &NamespacePlan,
        upstream: Upstream,
        brief: &Brief,
        playbook: &[Step],
    ) -> Result<AttemptOutcome, WorkerError> {
        if std::env::var(LIVE_ENV).as_deref() == Ok("1") {
            return self.run(permit, plan, upstream, brief, playbook);
        }
        eprintln!("UNMEASURED: live attempt not run ({LIVE_ENV}!=1)");
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
            model_requests: Vec::new(),
        })
    }

    /// Drive `playbook` in order. After a failed step every later step is `Skipped`. When the
    /// plan carries a model door, the door forwards to `upstream` for the whole attempt and is
    /// closed (socket removed) before this returns.
    ///
    /// # Errors
    /// [`WorkerError::Contract`] for a brief with an empty RESTATEMENT or an unparseable token,
    /// [`WorkerError::Door`] when the door cannot open, [`WorkerError::Host`] when the spawn door
    /// refuses a run step, [`WorkerError::Spawn`] when a process cannot start. A timed-out or
    /// failing process is recorded as [`StepStatus::Failed`], not an error.
    pub fn run(
        &self,
        permit: &Permit,
        plan: &NamespacePlan,
        upstream: Upstream,
        brief: &Brief,
        playbook: &[Step],
    ) -> Result<AttemptOutcome, WorkerError> {
        brief.check_restatement()?;
        let start = Instant::now();
        let door = match &plan.model_door {
            Some(path) => Some(model_door::serve(path, upstream, self.door_budget)?),
            None => None,
        };
        let input = Sha256Hex::digest(brief.get(BriefField::Verify).as_bytes());
        let mut out = AttemptOutcome {
            steps: Vec::new(),
            stdout: Vec::new(),
            stderr: Vec::new(),
            exit: None,
            elapsed: Duration::ZERO,
            observations: Vec::new(),
            model_requests: Vec::new(),
        };
        let mut failed = false;
        for step in playbook {
            let status = if failed {
                skip("an earlier step failed")
            } else {
                match &step.kind {
                    StepKind::Unsupported { kind } => skip(&format!(
                        "driver has no handler for step kind {kind}; use a `sh:` step, whose \
                         command reaches the model through HEE4_MODEL_SOCKET"
                    )),
                    StepKind::Run { program, args } => {
                        let t0 = Instant::now();
                        let before = (out.stdout.len(), out.exit);
                        out.exit = None;
                        let status = run_step(permit, plan, program, args, &mut out)?;
                        let stdout = out.stdout.get(before.0..).unwrap_or_default().to_vec();
                        let ob = self.command_observation(
                            input,
                            out.exit,
                            &stdout,
                            t0.elapsed(),
                            plan.timeout,
                        )?;
                        out.observations.push(ob);
                        status
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
        if let Some(door) = door {
            out.model_requests = door.close();
            let budget_ms = u64::try_from(plan.timeout.as_millis()).unwrap_or(u64::MAX);
            if let Some(ob) =
                self.door_observation(&out.model_requests, input, out.elapsed, budget_ms)?
            {
                out.observations.push(ob);
            }
        }
        Ok(out)
    }

    /// The tier-0 observation for one `Run` step: `Pass` only on exit 0, `Fail` otherwise (a
    /// timeout has no exit and is `Fail`). It always carries `exit` and `stdout` evidence, so a
    /// ran-and-passed step is never an empty source.
    ///
    /// # Errors
    /// [`WorkerError::Contract`] when a token does not parse.
    pub fn command_observation(
        &self,
        input: Sha256Hex,
        exit: Option<i32>,
        stdout: &[u8],
        elapsed: Duration,
        budget: Duration,
    ) -> Result<Observation, WorkerError> {
        let exit_text = exit.map_or_else(|| "none".to_owned(), |c| c.to_string());
        Ok(Observation {
            source: "hee4-worker-native".parse::<SourceId>()?,
            input_sha256: input,
            tool: ToolId {
                name: "command".parse::<ToolName>()?,
                version: "bwrap".parse::<ToolVersion>()?,
            },
            head_sha: self.head_sha.clone(),
            outcome: if exit == Some(0) {
                Outcome::Pass
            } else {
                Outcome::Fail
            },
            evidence: vec![
                Evidence {
                    label: "exit".parse()?,
                    sha256: Sha256Hex::digest(exit_text.as_bytes()),
                },
                Evidence {
                    label: "stdout".parse()?,
                    sha256: Sha256Hex::digest(stdout),
                },
            ],
            advisory: false,
            elapsed_ms: u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX),
            budget_ms: u64::try_from(budget.as_millis()).unwrap_or(u64::MAX),
        })
    }

    /// One observation for the door's log: one `model_request` evidence per request, `Pass` only
    /// when every request was forwarded. `input_sha256` is `input`, the subject's input (the
    /// digest of the brief's VERIFY text), so `decide` can reconcile it; the request digests are
    /// the evidence.
    /// `None` when no request came through.
    ///
    /// # Errors
    /// [`WorkerError::Contract`] when a token does not parse (the model name as a tool version).
    pub fn door_observation(
        &self,
        requests: &[DoorRequest],
        input: Sha256Hex,
        elapsed: Duration,
        budget_ms: u64,
    ) -> Result<Option<Observation>, WorkerError> {
        if requests.is_empty() {
            return Ok(None);
        }
        let evidence = requests
            .iter()
            .map(|r| {
                Ok(Evidence {
                    label: "model_request".parse()?,
                    sha256: r.sha256,
                })
            })
            .collect::<Result<Vec<_>, WorkerError>>()?;
        let all_forwarded = requests.iter().all(|r| r.fate == DoorFate::Forwarded);
        Ok(Some(Observation {
            source: "hee4-worker-native".parse::<SourceId>()?,
            input_sha256: input,
            tool: ToolId {
                name: "model-door".parse::<ToolName>()?,
                version: self.model.parse::<ToolVersion>()?,
            },
            head_sha: self.head_sha.clone(),
            outcome: if all_forwarded {
                Outcome::Pass
            } else {
                Outcome::Error
            },
            evidence,
            advisory: false,
            elapsed_ms: u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX),
            budget_ms,
        }))
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
    use std::path::Path;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::thread;

    const BRIEF: &str = "GOAL: g\nSCOPE: s\nCONTEXT: c\nACCEPTANCE: a\nVERIFY: v\nTIMEBOX: t\nFORBIDDEN: f\nREPORT: r\nSTANDING: s\nRECON: r\nRESTATEMENT: I restate the goal.";
    const TAGS: &str = r#"{"models":[{"name":"mock:1"}]}"#;

    /// Mock upstream on 127.0.0.1 answering every connection with `TAGS`; counts connections.
    fn mock() -> std::io::Result<(Upstream, u16, Arc<AtomicUsize>)> {
        let l = TcpListener::bind("127.0.0.1:0")?;
        let port = l.local_addr()?.port();
        let hits = Arc::new(AtomicUsize::new(0));
        let h = Arc::clone(&hits);
        thread::spawn(move || {
            for conn in l.incoming() {
                let Ok(mut s) = conn else { continue };
                h.fetch_add(1, Ordering::SeqCst);
                let mut buf = [0u8; 8192];
                let _ = s.read(&mut buf);
                let _ = s.write_all(
                    format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{TAGS}",
                        TAGS.len()
                    )
                    .as_bytes(),
                );
            }
        });
        let up = Upstream::parse(&format!("http://127.0.0.1:{port}"))
            .map_err(|e| std::io::Error::other(e.to_string()))?;
        Ok((up, port, hits))
    }

    fn closed() -> Result<Upstream, Box<dyn std::error::Error>> {
        Ok(Upstream::parse("http://127.0.0.1:9")?)
    }

    /// A fresh work root per test (`name`), so concurrent tests never share a door path.
    fn fixture(
        name: &str,
        needs_model: bool,
        programs: &[&str],
    ) -> Result<(Permit, NamespacePlan, Brief, Attempt), Box<dyn std::error::Error>> {
        let root = std::env::temp_dir().join(format!("hee4-nat-{}-{name}", std::process::id()));
        std::fs::create_dir_all(&root)?;
        let permit = Permit::mint(
            ReceiptId("r1".into()),
            SpawnScope::of_programs(programs.iter().map(PathBuf::from).collect()),
        );
        let task = NamespaceTask::new("t1".parse()?, &root, needs_model, Duration::from_secs(20))?;
        std::fs::create_dir_all(task.work_dir())?;
        let a = Attempt::new("qwen:7b", "a".repeat(40).parse()?);
        Ok((permit, plan_for(&task), Brief::parse(BRIEF)?, a))
    }

    fn run_step(name: &str, program: &str, args: &[&str]) -> Step {
        Step {
            name: name.into(),
            kind: StepKind::Run {
                program: program.into(),
                args: args.iter().map(|a| (*a).to_owned()).collect(),
            },
        }
    }

    /// A `model:` line as the playbook parses it: the kind is the literal "model".
    fn model_step() -> Step {
        Step {
            name: "ask".into(),
            kind: StepKind::Unsupported {
                kind: "model".into(),
            },
        }
    }

    fn sandbox_tools() -> bool {
        let ok = ["/usr/bin/curl", "/usr/bin/sh", spawn::BWRAP]
            .iter()
            .all(|p| Path::new(p).exists());
        if !ok {
            println!("UNMEASURED: bwrap, sh or curl absent; sandbox test skipped");
        }
        ok
    }

    /// Acceptance 3 and 4: in a real `--unshare-net` bwrap sandbox the candidate reaches the
    /// mock model only through the door (found via `$HEE4_MODEL_SOCKET`), and direct TCP to the
    /// mock's own live port, or to 127.0.0.1:11434, fails. Each forwarded request is evidence.
    #[test]
    fn sandbox_reaches_model_only_through_door() -> R {
        if !sandbox_tools() {
            return Ok(());
        }
        let (up, port, hits) = mock()?;
        let (permit, plan, brief, a) = fixture("door", true, &["/usr/bin/sh", "/usr/bin/curl"])?;
        let door = plan.model_door.clone().ok_or("no door in plan")?;
        let through = run_step(
            "through-door",
            "/usr/bin/sh",
            &[
                "-c",
                r#"/usr/bin/curl -sS -m 5 --unix-socket "$HEE4_MODEL_SOCKET" http://model/api/tags"#,
            ],
        );
        let direct = format!("http://127.0.0.1:{port}/api/tags");
        let direct_step = run_step("direct-mock", "/usr/bin/curl", &["-sS", "-m", "3", &direct]);
        let o = a.run(&permit, &plan, up, &brief, &[through, direct_step])?;
        let stdout = String::from_utf8_lossy(&o.stdout);
        let stderr = String::from_utf8_lossy(&o.stderr);
        println!(
            "SANDBOX positive: curl --unix-socket {} http://model/api/tags -> {:?} stdout={stdout}",
            door.display(),
            o.steps[0].status
        );
        println!(
            "SANDBOX negative: curl {direct} -> {:?} stderr={}",
            o.steps[1].status,
            stderr.trim()
        );
        assert_eq!(o.steps[0].status, StepStatus::Done { exit: Some(0) });
        assert_eq!(stdout, TAGS);
        assert!(matches!(o.steps[1].status, StepStatus::Failed { .. }));
        assert_eq!(
            hits.load(Ordering::SeqCst),
            1,
            "only the door's forward hit the mock"
        );
        assert!(!door.exists(), "door socket removed after the attempt");
        assert_eq!(o.model_requests.len(), 1);
        assert_eq!(o.model_requests[0].bytes, 0);
        assert_eq!(o.model_requests[0].fate, DoorFate::Forwarded);
        let doors: Vec<_> = o
            .observations
            .iter()
            .filter(|ob| ob.tool.name.as_str() == "model-door")
            .collect();
        assert_eq!(doors.len(), 1);
        let ob = doors[0];
        assert_eq!(ob.outcome, Outcome::Pass);
        assert!(!ob.advisory);
        assert_eq!(
            (ob.tool.name.as_str(), ob.tool.version.as_str()),
            ("model-door", "qwen:7b")
        );
        assert_eq!(ob.evidence.len(), 1);
        assert_eq!(ob.evidence[0].label.as_str(), "model_request");
        assert_eq!(ob.evidence[0].sha256, Sha256Hex::digest(b""));

        let (permit, plan, brief, a) = fixture("ollama", true, &["/usr/bin/curl"])?;
        let ollama = run_step(
            "direct-11434",
            "/usr/bin/curl",
            &["-sS", "-m", "3", "http://127.0.0.1:11434/"],
        );
        let o = a.run(&permit, &plan, up, &brief, &[ollama])?;
        println!(
            "SANDBOX negative: curl http://127.0.0.1:11434/ -> {:?} stderr={}",
            o.steps[0].status,
            String::from_utf8_lossy(&o.stderr).trim()
        );
        assert!(matches!(o.steps[0].status, StepStatus::Failed { .. }));
        assert!(
            o.observations
                .iter()
                .all(|ob| ob.tool.name.as_str() == "command" && ob.outcome == Outcome::Fail),
            "no request, no door observation; the failed command is a Fail"
        );
        Ok(())
    }

    #[test]
    fn new_is_with_budget_default() -> R {
        let head: GitSha = "b".repeat(40).parse()?;
        assert_eq!(
            Attempt::new("m:1", head.clone()).door_budget,
            DoorBudget::DEFAULT
        );
        let b = hee4_contracts::Budgets::parse(r#"{"door":{"max_requests":1,"pool":2}}"#)?.door;
        let a = Attempt::with_budget("m:1", head.clone(), b);
        assert_eq!(a.door_budget, b);
        assert_eq!((a.model.as_str(), a.head_sha), ("m:1", head));
        Ok(())
    }

    /// Sibling of the sandbox test, gated the same way: an attempt built with
    /// `with_budget(.., max_requests: 1)` whose candidate curls the door twice has its second
    /// request refused `429` at the door, recorded by name in `model_requests`.
    #[test]
    fn sandbox_second_request_is_refused_429_under_with_budget() -> R {
        if !sandbox_tools() {
            return Ok(());
        }
        let (up, _port, hits) = mock()?;
        let (permit, plan, brief, a) = fixture("door429", true, &["/usr/bin/sh", "/usr/bin/curl"])?;
        let one = hee4_contracts::Budgets::parse(r#"{"door":{"max_requests":1}}"#)?.door;
        let a = Attempt::with_budget(&a.model, a.head_sha.clone(), one);
        let twice = run_step(
            "twice",
            "/usr/bin/sh",
            &[
                "-c",
                r#"/usr/bin/curl -sS -m 5 --unix-socket "$HEE4_MODEL_SOCKET" http://model/api/tags; /usr/bin/curl -sS -m 5 --unix-socket "$HEE4_MODEL_SOCKET" http://model/api/tags"#,
            ],
        );
        let o = a.run(&permit, &plan, up, &brief, &[twice])?;
        println!(
            "SANDBOX 429: two curls through the door under max_requests=1 -> {:?} stdout={}",
            o.steps[0].status,
            String::from_utf8_lossy(&o.stdout)
        );
        assert_eq!(o.steps[0].status, StepStatus::Done { exit: Some(0) });
        assert_eq!(
            hits.load(Ordering::SeqCst),
            1,
            "only the first request reached the mock"
        );
        let fates: Vec<DoorFate> = o.model_requests.iter().map(|r| r.fate).collect();
        assert_eq!(fates, vec![DoorFate::Forwarded, DoorFate::Refused(429)]);
        assert_eq!(o.model_requests[1].reason, "door request budget exhausted");
        let doors: Vec<_> = o
            .observations
            .iter()
            .filter(|ob| ob.tool.name.as_str() == "model-door")
            .collect();
        assert_eq!(doors.len(), 1);
        assert_eq!(doors[0].outcome, Outcome::Error);
        Ok(())
    }

    #[test]
    fn door_observation_records_each_request() -> R {
        let (_, _, _, a) = fixture("obs", false, &[])?;
        let reqs = vec![
            DoorRequest {
                bytes: 3,
                sha256: Sha256Hex::digest(b"abc"),
                fate: DoorFate::Forwarded,
                label: "forwarded",
                reason: "",
            },
            DoorRequest {
                bytes: 0,
                sha256: Sha256Hex::digest(b""),
                fate: DoorFate::Unreachable,
                label: "unreachable",
                reason: "",
            },
        ];
        let ob = a
            .door_observation(
                &reqs,
                Sha256Hex::digest(b"v"),
                Duration::from_millis(5),
                100,
            )?
            .ok_or("no observation")?;
        assert_eq!(ob.evidence.len(), 2);
        assert!(
            ob.evidence
                .iter()
                .all(|e| e.label.as_str() == "model_request")
        );
        assert_eq!(ob.evidence[0].sha256, Sha256Hex::digest(b"abc"));
        assert_eq!(ob.outcome, Outcome::Error);
        assert_eq!(
            a.door_observation(&[], Sha256Hex::digest(b"v"), Duration::ZERO, 0)?,
            None
        );
        Ok(())
    }

    #[test]
    fn skipped_steps_are_recorded_never_dropped() -> R {
        let (permit, plan, brief, a) = fixture("skip", false, &[])?;
        let playbook = vec![
            model_step(),
            Step {
                name: "review".into(),
                kind: StepKind::Unsupported {
                    kind: "human-review".into(),
                },
            },
        ];
        let o = a.run(&permit, &plan, closed()?, &brief, &playbook)?;
        assert_eq!(
            o.steps.iter().map(|s| s.name.as_str()).collect::<Vec<_>>(),
            vec!["ask", "review"]
        );
        assert!(matches!(
            &o.steps[0].status,
            StepStatus::Skipped { reason }
                if reason.contains("step kind model")
                    && reason.contains("sh:")
                    && reason.contains("HEE4_MODEL_SOCKET")
        ));
        assert!(
            matches!(&o.steps[1].status, StepStatus::Skipped { reason } if reason.contains("human-review"))
        );
        assert_eq!(o.observations.len(), 0);
        Ok(())
    }

    #[test]
    fn run_step_records_stdout_exit_elapsed_in_bwrap() -> R {
        if !Path::new(spawn::BWRAP).exists() {
            println!("UNMEASURED: bwrap absent; run_step test skipped");
            return Ok(());
        }
        let (permit, plan, brief, a) = fixture("echo", false, &["/usr/bin/echo"])?;
        let step = run_step("echo", "/usr/bin/echo", &["hi"]);
        match a.run(&permit, &plan, closed()?, &brief, &[step]) {
            Ok(o) => {
                assert_eq!(o.stdout, b"hi\n");
                assert_eq!(o.exit, Some(0));
                assert!(o.elapsed > Duration::ZERO);
                assert_eq!(o.observations.len(), 1);
                assert_eq!(o.observations[0].outcome, Outcome::Pass);
                assert_eq!(o.observations[0].evidence[0].label.as_str(), "exit");
            }
            Err(e) => println!("UNMEASURED: bwrap could not run here: {e}"),
        }
        Ok(())
    }

    #[test]
    fn run_step_out_of_scope_is_a_host_refusal() -> R {
        let (permit, plan, brief, a) = fixture("scope", false, &[])?;
        let step = run_step("x", "/usr/bin/echo", &[]);
        let res = a.run(&permit, &plan, closed()?, &brief, &[step]);
        assert!(matches!(res, Err(WorkerError::Host(_))));
        Ok(())
    }

    #[test]
    fn live_guard_skips_everything_when_unset() -> R {
        if std::env::var(LIVE_ENV).as_deref() == Ok("1") {
            println!("UNMEASURED: {LIVE_ENV}=1 set; guard test not applicable");
            return Ok(());
        }
        let (permit, plan, brief, a) = fixture("live", true, &[])?;
        let o = a.run_live(&permit, &plan, closed()?, &brief, &[model_step()])?;
        assert!(
            matches!(&o.steps[0].status, StepStatus::Skipped { reason } if reason.contains(LIVE_ENV))
        );
        assert_eq!(o.observations.len(), 0);
        Ok(())
    }
}
