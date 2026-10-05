//! The synchronous dispatcher (RL-2, skeleton slice): one `admitted` task at a time,
//! route → namespace → permit → attempt → observe → `decide` → seal → settle.
//!
//! Every state change is `Store::apply`; the only verdict is `hee4_evidence::decide_and_seal`'s.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use hee4_contracts::{
    AbandonReason, Brief, BriefField, Event, GitSha, Observation, Phase, ReceiptId, Resolution,
    Settlement, Sha256Hex, SourceId, TaskId, Verdict,
};
use hee4_core::StoreError;
use hee4_evidence::{Identities, Identity, Source, Subject, Why, decide_and_seal, observation_id};
use hee4_host::model::OllamaClient;
use hee4_host::model_door::Upstream;
use hee4_host::spawn::{self, Permit, SpawnScope};
use hee4_worker::namespace::{NamespaceTask, plan_for};
use hee4_worker::native::{Attempt, AttemptOutcome, LIVE_ENV, Step, StepKind, StepStatus};
use hee4_worker::route::{
    Availability, Capabilities, CapabilityFloor, ModelEntry, Roster, RouteInput, RouteRefusal,
    Selection, select,
};

use crate::actions::Engine;

/// The model URL (loopback only). The candidate never dials it: the attempt's model door does.
pub const MODEL_URL: &str = "http://127.0.0.1:11434";

/// What the dispatcher needs that is not in the ledger.
#[derive(Debug, Clone)]
pub struct Config {
    /// The task's declared baseline model (`HEE4_MODEL`, default below).
    pub model: String,
    /// `HEE4_LIVE_MODEL=1`.
    pub live: bool,
}

impl Config {
    /// Read `HEE4_MODEL` and `HEE4_LIVE_MODEL`.
    #[must_use]
    pub fn from_env() -> Self {
        Self {
            model: std::env::var("HEE4_MODEL").unwrap_or_else(|_| "qwen2.5-coder:7b".to_owned()),
            live: std::env::var(LIVE_ENV).as_deref() == Ok("1"),
        }
    }
}

/// Why a dispatch step stopped short of a verdict (the task is moved by `Store::apply` first).
#[derive(Debug, thiserror::Error)]
pub enum DispatchError {
    /// The ledger refused or failed.
    #[error("ledger: {0}")]
    Store(#[from] StoreError),
    /// Sealing failed.
    #[error("seal: {0}")]
    Seal(#[from] hee4_evidence::SealError),
    /// An id did not parse.
    #[error("contract: {0}")]
    Contract(#[from] hee4_contracts::Refusal),
}

/// The playbook named by the brief's VERIFY field: one step per non-empty line. An absolute
/// path runs in the namespace as a bare argv; `sh: <line>` runs `/bin/sh -c <line>` there, so the
/// line can use `$HEE4_MODEL_SOCKET`; `model: <prompt>` is a recorded skip (the driver makes no
/// model call; use `sh:`); anything else is a named skip.
#[must_use]
pub fn playbook(verify: &str) -> Vec<Step> {
    verify
        .lines()
        .map(|l| l.trim().trim_start_matches("- ").trim_matches('`'))
        .filter(|l| !l.is_empty())
        .enumerate()
        .map(|(i, line)| {
            let kind = if let Some(prompt) = line.strip_prefix("model:") {
                StepKind::Generate {
                    prompt: prompt.trim().to_owned(),
                }
            } else if let Some(cmd) = line.strip_prefix("sh:") {
                StepKind::Run {
                    program: PathBuf::from("/bin/sh"),
                    args: vec!["-c".to_owned(), cmd.trim().to_owned()],
                }
            } else if line.starts_with('/') {
                let mut words = line.split_whitespace().map(str::to_owned);
                let program = PathBuf::from(words.next().unwrap_or_default());
                StepKind::Run {
                    program,
                    args: words.collect(),
                }
            } else {
                StepKind::Unsupported {
                    kind: line.split_whitespace().next().unwrap_or("").to_owned(),
                }
            };
            Step {
                name: format!("verify-{}", i + 1),
                kind,
            }
        })
        .collect()
}

/// TIMEBOX as `<n>s` or `<n> min`; otherwise 120 s.
#[must_use]
pub fn timebox(text: &str) -> Duration {
    let t = text.trim();
    let digits: String = t.chars().take_while(char::is_ascii_digit).collect();
    let n: u64 = digits.parse().unwrap_or(0);
    let unit = t[digits.len()..].trim_start();
    match (n, unit.chars().next()) {
        (0, _) => Duration::from_secs(120),
        (n, Some('m')) => Duration::from_secs(n * 60),
        (n, _) => Duration::from_secs(n),
    }
}

/// The first `admitted` task, oldest id first.
fn next_admitted(engine: &Engine) -> Result<Option<TaskId>, StoreError> {
    let store = engine.store();
    for task in store.task_ids()? {
        if store.phase(&task)? == Some(Phase::Admitted) {
            return Ok(Some(task));
        }
    }
    Ok(None)
}

fn apply(engine: &Engine, task: &TaskId, event: Event) -> Result<Phase, StoreError> {
    let phase = engine.store().apply(task, event)?;
    eprintln!(
        "dispatch task={task} event={event:?} phase={}",
        phase.as_str()
    );
    Ok(phase)
}

fn abandon(engine: &Engine, task: &TaskId, reason: AbandonReason) -> Result<Phase, StoreError> {
    eprintln!("dispatch task={task} abandon reason={reason:?}");
    apply(engine, task, Event::Resolve(Resolution::Abandon(reason)))
}

/// The abandon reason for a refused route: `floor_unmet` only when the capability floor was
/// the cause. No wildcard: a new `RouteRefusal` variant must be classified here.
const fn route_reason(r: &RouteRefusal) -> AbandonReason {
    AbandonReason::RouteRefused {
        floor_unmet: match r {
            RouteRefusal::NoCapableModel { .. } => true,
            RouteRefusal::OnlyPreviousAttempt(_)
            | RouteRefusal::NoBaseline { .. }
            | RouteRefusal::NoDecision => false,
        },
    }
}

/// Route by floor over a one-row roster of the declared model. Availability is probed only
/// when the model is needed; otherwise it is `Unknown`, which routes to the declared baseline.
pub(crate) fn route(
    cfg: &Config,
    client: &OllamaClient,
    needs_model: bool,
) -> Result<Selection, RouteRefusal> {
    let availability = if needs_model {
        if client.tags().is_ok() {
            Availability::Up
        } else {
            Availability::Down
        }
    } else {
        Availability::Unknown
    };
    let roster = Roster {
        models: vec![ModelEntry {
            name: cfg.model.clone(),
            caps: Capabilities {
                ctx_tokens: 32_768,
                json_mode: true,
                tool_use: false,
                local: true,
            },
            availability,
            cost_milli: 0,
            latency_ms: 0,
            quality: 0,
        }],
    };
    let input = RouteInput {
        floor: CapabilityFloor {
            ctx_tokens: 0,
            json_mode: false,
            tool_use: false,
            local_only: true,
        },
        cost_ceiling_milli: None,
        deadline_ms: None,
        quality_floor: None,
        baseline: Some(cfg.model.clone()),
        previous_attempt: None,
    };
    select(&input, &roster)
}

/// Dispatch one `admitted` task to a terminal or parked phase. `Ok(None)`: nothing to do.
///
/// # Errors
/// [`DispatchError`] when the ledger fails; the task stays where the last `apply` left it, and
/// startup reconcile owns it after a restart.
pub fn step(engine: &Engine, cfg: &Config) -> Result<Option<(TaskId, Phase)>, DispatchError> {
    let Some(task) = next_admitted(engine)? else {
        return Ok(None);
    };
    let Some(brief) = fs::read_to_string(engine.brief_path(&task))
        .ok()
        .and_then(|t| Brief::parse(&t).ok())
    else {
        return Ok(Some((
            task.clone(),
            abandon(engine, &task, AbandonReason::BriefUnreadable)?,
        )));
    };
    let steps = playbook(brief.get(BriefField::Verify));
    let wants_model = steps.iter().any(|s| match &s.kind {
        StepKind::Generate { .. } => true,
        StepKind::Run { program, .. } => program == Path::new("/bin/sh"),
        StepKind::Unsupported { .. } => false,
    });
    if wants_model && !cfg.live {
        eprintln!("UNMEASURED: live model not called ({LIVE_ENV}!=1); model steps skip");
    }
    let needs_model = wants_model && cfg.live;
    let client = OllamaClient::new(MODEL_URL);
    let selection = match route(cfg, &client, needs_model) {
        Ok(s) => s,
        Err(r) => {
            return Ok(Some((
                task.clone(),
                abandon(engine, &task, route_reason(&r))?,
            )));
        }
    };
    let ns = match NamespaceTask::with_door_root(
        task.clone(),
        engine.work(),
        engine.doors(),
        needs_model,
        timebox(brief.get(BriefField::Timebox)),
    ) {
        Ok(ns) => ns,
        Err(e) => {
            eprintln!("dispatch task={task} cause={e}");
            return Ok(Some((
                task.clone(),
                abandon(engine, &task, AbandonReason::NamespaceRefused)?,
            )));
        }
    };
    let plan = plan_for(&ns);
    if let Err(e) = fs::create_dir_all(ns.work_dir()) {
        eprintln!("dispatch task={task} cause={e}");
        return Ok(Some((
            task.clone(),
            abandon(engine, &task, AbandonReason::WorkDirUnavailable)?,
        )));
    }
    let Ok(head) = crate::HEAD.parse::<GitSha>() else {
        return Ok(Some((
            task.clone(),
            abandon(engine, &task, AbandonReason::HeadUnknown)?,
        )));
    };
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    let receipt_id: ReceiptId = format!("r-{task}-{nanos:x}").parse()?;
    let programs = steps
        .iter()
        .filter_map(|s| match &s.kind {
            StepKind::Run { program, .. } => Some(program.clone()),
            _ => None,
        })
        .collect();
    let permit = Permit::mint(
        spawn::ReceiptId(receipt_id.to_string()),
        SpawnScope::of_programs(programs),
    );

    let upstream = match Upstream::parse(MODEL_URL) {
        Ok(u) => u,
        Err(e) => {
            eprintln!("dispatch task={task} cause={e}");
            return Ok(Some((
                task.clone(),
                abandon(engine, &task, AbandonReason::NoPermit)?,
            )));
        }
    };

    apply(engine, &task, Event::Dispatch)?;
    let attempt =
        Attempt::new(&selection.model, head.clone()).run(&permit, &plan, upstream, &brief, &steps);
    let outcome = match attempt {
        Ok(o) => o,
        Err(e) => {
            eprintln!("dispatch task={task} attempt_error={e}");
            apply(engine, &task, Event::Settle(Settlement::NotReady))?;
            return Ok(Some((task.clone(), apply(engine, &task, Event::Stop)?)));
        }
    };
    settle_and_decide(engine, &task, receipt_id, &permit, &brief, head, &outcome)
}

/// After the attempt: settle, ledger each observation, `decide_and_seal`, append, decide.
#[allow(clippy::too_many_arguments)]
fn settle_and_decide(
    engine: &Engine,
    task: &TaskId,
    receipt_id: ReceiptId,
    permit: &Permit,
    brief: &Brief,
    head: GitSha,
    outcome: &AttemptOutcome,
) -> Result<Option<(TaskId, Phase)>, DispatchError> {
    for s in &outcome.steps {
        eprintln!("dispatch task={task} step={} status={:?}", s.name, s.status);
    }
    if outcome
        .steps
        .iter()
        .any(|s| matches!(s.status, StepStatus::Failed { .. }))
    {
        apply(engine, task, Event::Settle(Settlement::NotReady))?;
        return Ok(Some((task.clone(), apply(engine, task, Event::Stop)?)));
    }
    apply(engine, task, Event::Settle(Settlement::Ready))?;
    for obs in &outcome.observations {
        let id = observation_id(task, obs)?;
        engine.store().record_observation(task, &id, obs)?;
        apply(engine, task, Event::Observe)?;
    }
    let receipt = seal(
        engine,
        task,
        receipt_id,
        permit,
        brief,
        head,
        &outcome.observations,
    )?;
    let verdict = receipt.decision().verdict;
    eprintln!(
        "dispatch task={task} verdict={verdict:?} receipt={}",
        receipt.hash_self()
    );
    let mut phase = apply(engine, task, Event::Decide(verdict))?;
    if verdict == Verdict::Pass {
        phase = apply(engine, task, Event::Accept)?;
    }
    Ok(Some((task.clone(), phase)))
}

#[allow(clippy::too_many_arguments)]
fn seal(
    engine: &Engine,
    task: &TaskId,
    id: ReceiptId,
    permit: &Permit,
    brief: &Brief,
    head: GitSha,
    obs: &[Observation],
) -> Result<hee4_contracts::Receipt, DispatchError> {
    let store = engine.store();
    let source = |name: &str, bytes: &[u8]| -> Result<Identity, DispatchError> {
        Ok(Identity::Wired(Source {
            name: name.parse::<SourceId>()?,
            digest: Sha256Hex::digest(bytes),
        }))
    };
    let collector = match store.epoch() {
        Ok(epoch) if !epoch.is_empty() => source("hee4-ledger", epoch.as_bytes())?,
        _ => Identity::Unavailable(Why::Unreadable {
            what: "ledger epoch".into(),
        }),
    };
    let ids = Identities {
        collector,
        locks: source("hee4-permit", format!("{permit:?}").as_bytes())?,
        standards: source("gate.toml", crate::GATE_TOML)?,
    };
    // The subject's input is the VERIFY text; every observation carries its digest.
    let input = brief.get(BriefField::Verify);
    let subject = Subject {
        task_id: task.clone(),
        head_sha: head,
        input_sha256: Sha256Hex::digest(input.as_bytes()),
    };
    let receipt = decide_and_seal(store.chain_head(task)?, id, &ids, obs, &subject)?;
    store.append_receipt(&receipt)?;
    Ok(receipt)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sh_line_becomes_a_shell_run_step() {
        let steps = playbook("sh: echo \"$HEE4_MODEL_SOCKET\" | wc -c\nmodel: hi\n");
        assert_eq!(
            steps[0].kind,
            StepKind::Run {
                program: "/bin/sh".into(),
                args: vec!["-c".into(), "echo \"$HEE4_MODEL_SOCKET\" | wc -c".into()]
            }
        );
        assert!(matches!(steps[1].kind, StepKind::Generate { .. }));
    }

    #[test]
    fn verify_lines_become_steps() {
        let steps = playbook("/usr/bin/true\nmodel: say hi\n`cargo test`\n");
        assert_eq!(steps.len(), 3);
        assert_eq!(
            steps[0].kind,
            StepKind::Run {
                program: "/usr/bin/true".into(),
                args: vec![]
            }
        );
        assert_eq!(
            steps[1].kind,
            StepKind::Generate {
                prompt: "say hi".into()
            }
        );
        assert_eq!(
            steps[2].kind,
            StepKind::Unsupported {
                kind: "cargo".into()
            }
        );
    }

    #[test]
    fn timebox_parses_seconds_and_minutes() {
        assert_eq!(timebox("10s"), Duration::from_secs(10));
        assert_eq!(timebox("60 min"), Duration::from_secs(3600));
        assert_eq!(timebox("soon"), Duration::from_secs(120));
    }
}
