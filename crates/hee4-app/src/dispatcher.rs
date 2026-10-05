//! The synchronous dispatcher (RL-2, skeleton slice): one `admitted` task at a time,
//! route → namespace → permit → attempt → observe → `decide` → seal → settle.
//!
//! Every state change is `Store::apply`; the only verdict is `hee4_evidence::decide_and_seal`'s.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use hee4_contracts::bounds::MAX_VIEW_ITEMS;
use hee4_contracts::{
    AbandonReason, Brief, BriefField, Event, GitSha, Observation, Phase, ReceiptId, Resolution,
    Settlement, Sha256Hex, SourceId, TaskId, Verdict, VerifyLine,
};
use hee4_core::roster::RosterDefinition;
use hee4_core::{AttemptId, AttemptStart, StoreError};
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

/// The playbook named by the brief's VERIFY field: one step per line of
/// [`hee4_contracts::VerifyLine::parse_all`], the grammar's only home. The mapping is
/// exhaustive (no wildcard arm): `Shell` runs `/bin/sh -c <command>` in the namespace, so the
/// line can use `$HEE4_MODEL_SOCKET`; `Exec` runs the bare argv there; `Unsupported` (a
/// `model:` line is `Unsupported{kind: "model"}`) is a named skip.
#[must_use]
pub fn playbook(verify: &str) -> Vec<Step> {
    VerifyLine::parse_all(verify)
        .into_iter()
        .enumerate()
        .map(|(i, line)| Step {
            name: format!("verify-{}", i + 1),
            kind: match line {
                VerifyLine::Shell { command } => StepKind::Run {
                    program: PathBuf::from("/bin/sh"),
                    args: vec!["-c".to_owned(), command],
                },
                VerifyLine::Exec { program, args } => StepKind::Run {
                    program: PathBuf::from(program),
                    args,
                },
                VerifyLine::Unsupported { kind } => StepKind::Unsupported { kind },
            },
        })
        .collect()
}

/// Whether any step wants the model door: a `sh:` step, whose command may reach the model
/// through `$HEE4_MODEL_SOCKET`. The one home for this predicate; `step` and `task.preview`
/// both read it, so preview probes availability exactly as dispatch does.
#[must_use]
pub(crate) fn wants_model(steps: &[Step]) -> bool {
    steps.iter().any(|s| match &s.kind {
        StepKind::Run { program, .. } => program == Path::new("/bin/sh"),
        StepKind::Unsupported { .. } => false,
    })
}

/// TIMEBOX as `<n>s` or `<n> min`; otherwise `default` (the dispatcher passes
/// `attempt.timebox_default_ms`). The result never exceeds `ceiling` (the dispatcher passes
/// `attempt.deadline_ms`, the hard deadline): a count too large for `u64`, or minutes whose
/// seconds overflow, saturate and are clamped, never wrapped or panicked on.
#[must_use]
pub fn timebox(text: &str, default: Duration, ceiling: Duration) -> Duration {
    let t = text.trim();
    let digits: String = t.chars().take_while(char::is_ascii_digit).collect();
    let unit = t[digits.len()..].trim_start();
    // All ASCII digits, so the only parse failure is overflow: past any ceiling.
    let n: u64 = if digits.is_empty() {
        0
    } else {
        digits.parse().unwrap_or(u64::MAX)
    };
    let asked = match (n, unit.chars().next()) {
        (0, _) => default,
        (n, Some('m')) => Duration::from_secs(n.saturating_mul(60)),
        (n, _) => Duration::from_secs(n),
    };
    asked.min(ceiling)
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

/// The roster `route` selects over, read from the ledger, and the ids of the disabled model
/// records and of `model:` ids of another kind (`task.preview`'s `exclusions`). One `ModelEntry` per eligible record (kind `model`,
/// not disabled; name = the id without `model:`; caps and figures from the definition;
/// availability `Unknown` until `route` probes). The ledger is read under one `engine.store()`
/// lock and released before anything else. When the roster table is empty (the roster family's
/// hook never ran: a unit-test engine) the one-row roster of the declared model is built from
/// `actions::roster::default_definition`; when rows exist but none is eligible the roster is
/// empty and `route` refuses, so a disabled model is never re-enabled by a fallback.
pub(crate) fn roster_from_store(
    engine: &Engine,
    cfg: &Config,
) -> Result<(Roster, Vec<String>), StoreError> {
    let (count, eligible, excluded) = {
        let store = engine.store();
        (
            store.roster_count()?,
            store.roster_eligible()?,
            store.roster_excluded(MAX_VIEW_ITEMS)?,
        )
    };
    if count == 0 {
        let fallback = entry(&cfg.model, &crate::actions::roster::default_definition());
        return Ok((
            Roster {
                models: vec![fallback],
            },
            Vec::new(),
        ));
    }
    let roster = Roster {
        models: eligible
            .iter()
            .map(|r| {
                entry(
                    r.head.id.model_name().unwrap_or(r.head.id.as_str()),
                    &r.definition,
                )
            })
            .collect(),
    };
    Ok((roster, excluded))
}

/// A roster record's definition as the route's row; availability is `route`'s to set.
fn entry(name: &str, d: &RosterDefinition) -> ModelEntry {
    ModelEntry {
        name: name.to_owned(),
        caps: Capabilities {
            ctx_tokens: d.caps.ctx_tokens,
            json_mode: d.caps.json_mode,
            tool_use: d.caps.tool_use,
            local: d.caps.local,
        },
        availability: Availability::Unknown,
        cost_milli: d.cost_milli,
        latency_ms: d.latency_ms,
        quality: d.quality,
    }
}

/// Route by floor over `roster`, baseline = the declared model. When the model is needed, one
/// `tags` call (bounded by `tags_timeout`, the `model.tags_timeout_ms` budget) lists what the upstream holds and each row is `Up` exactly when its name is listed
/// (a name without a tag also matches `<name>:latest`), `Down` otherwise or when the call fails;
/// when it is not needed every row is `Unknown`, which routes to the declared baseline when it is
/// in the roster.
pub(crate) fn route(
    cfg: &Config,
    client: &OllamaClient,
    needs_model: bool,
    roster: &Roster,
    tags_timeout: Duration,
) -> Result<Selection, RouteRefusal> {
    if !needs_model {
        return route_with(cfg, roster, |_| Availability::Unknown);
    }
    let listed = client.tags_within(tags_timeout).unwrap_or_default();
    route_with(cfg, roster, |name| {
        let held = listed
            .iter()
            .any(|t| t == name || (!name.contains(':') && *t == format!("{name}:latest")));
        if held {
            Availability::Up
        } else {
            Availability::Down
        }
    })
}

/// The model the dispatcher would select now with no upstream call: every row `Up` when the
/// engine is live (the dispatcher's view whenever the upstream holds every eligible model),
/// `Unknown` otherwise. `roster.disable` reads it to find the attempts on a record; it is an
/// approximation until attempts record their model (K1-attempts-ledger).
pub(crate) fn route_as_dispatched(
    cfg: &Config,
    roster: &Roster,
) -> Result<Selection, RouteRefusal> {
    let availability = if cfg.live {
        Availability::Up
    } else {
        Availability::Unknown
    };
    route_with(cfg, roster, |_| availability)
}

/// `select` over `roster` with each row's availability from `availability(name)`.
fn route_with(
    cfg: &Config,
    roster: &Roster,
    availability: impl Fn(&str) -> Availability,
) -> Result<Selection, RouteRefusal> {
    let roster = Roster {
        models: roster
            .models
            .iter()
            .cloned()
            .map(|mut m| {
                m.availability = availability(&m.name);
                m
            })
            .collect(),
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
    let wants_model = wants_model(&steps);
    if wants_model && !cfg.live {
        eprintln!(
            "UNMEASURED: live model not called ({LIVE_ENV}!=1); `sh:` steps run without the door"
        );
    }
    let needs_model = wants_model && cfg.live;
    let budgets = *engine.budgets();
    let client = OllamaClient::new(MODEL_URL);
    let (roster, _) = roster_from_store(engine, cfg)?;
    let selection = match route(
        cfg,
        &client,
        needs_model,
        &roster,
        budgets.model.tags_timeout(),
    ) {
        Ok(s) => s,
        Err(r) => {
            return Ok(Some((
                task.clone(),
                abandon(engine, &task, route_reason(&r))?,
            )));
        }
    };
    let (ns, generation) = match workspace(
        engine,
        &task,
        needs_model,
        timebox(
            brief.get(BriefField::Timebox),
            budgets.attempt.timebox_default(),
            budgets.attempt.deadline(),
        ),
    )? {
        Ok(built) => built,
        Err(abandoned) => return Ok(Some((task.clone(), abandoned))),
    };
    let plan = plan_for(&ns);
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
    let permit = permit_for(&receipt_id, &steps);

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
    let start = AttemptStart {
        receipt_id: receipt_id.clone(),
        permit_id: permit.id().0,
        model: selection.model.clone(),
        head_sha: head.clone(),
        workspace: ns.work_dir().to_path_buf(),
        // No lease is issued: a lease is what R09 compares before licensing reuse, and nothing
        // here grants reuse of a workspace. `attempt.deadline_ms` is therefore not recorded.
        lease: None,
    };
    let id = match acknowledge(engine, &task, generation, &start)? {
        Ok(id) => id,
        Err(stopped) => return Ok(Some((task.clone(), stopped))),
    };
    let on_start = |pid: u32, start_ticks: u64| record_pid(engine, &id, pid, start_ticks);
    let attempt = Attempt::with_budget(&selection.model, head.clone(), budgets.door)
        .run(&permit, &plan, upstream, &brief, &steps, &on_start);
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

/// The attempt's permit: receipt `receipt_id`, scope = the `Run` steps' programs.
fn permit_for(receipt_id: &ReceiptId, steps: &[Step]) -> Permit {
    let programs = steps
        .iter()
        .filter_map(|s| match &s.kind {
            StepKind::Run { program, .. } => Some(program.clone()),
            StepKind::Unsupported { .. } => None,
        })
        .collect();
    Permit::mint(
        spawn::ReceiptId(receipt_id.to_string()),
        SpawnScope::of_programs(programs),
    )
}

/// S2: a fresh work dir per generation, `<work>/<task>/<generation>` as the namespace's work
/// root (the namespace appends `/<task>`). The generation the coming `Dispatch` opens is the
/// latest attempt's plus one; the row it opens is read back after `Dispatch` and must agree.
/// `Err(phase)` when the task was abandoned (`NamespaceRefused`, `WorkDirUnavailable`).
fn workspace(
    engine: &Engine,
    task: &TaskId,
    needs_model: bool,
    timebox: Duration,
) -> Result<Result<(NamespaceTask, u64), Phase>, StoreError> {
    let generation = engine
        .store()
        .latest_attempt(task)?
        .map_or(1, |row| row.generation + 1);
    let work_root = engine
        .work()
        .join(task.as_str())
        .join(generation.to_string());
    let ns = match NamespaceTask::with_door_root(
        task.clone(),
        &work_root,
        engine.doors(),
        needs_model,
        timebox,
    ) {
        Ok(ns) => ns,
        Err(e) => {
            eprintln!("dispatch task={task} cause={e}");
            return abandon(engine, task, AbandonReason::NamespaceRefused).map(Err);
        }
    };
    if let Err(e) = fs::create_dir_all(ns.work_dir()) {
        eprintln!("dispatch task={task} cause={e}");
        return abandon(engine, task, AbandonReason::WorkDirUnavailable).map(Err);
    }
    Ok(Ok((ns, generation)))
}

/// Read back the row `Dispatch` opened (generation `generation`) and record its start facts
/// (`Store::attempt_started`) before anything runs. `Err(phase)` when the task was moved instead: a leased workspace is abandoned
/// `WorkDirUnavailable` with the store's text printed; any other refusal is `Settle(NotReady)`
/// then `Stop`, never a silent run.
fn acknowledge(
    engine: &Engine,
    task: &TaskId,
    generation: u64,
    start: &AttemptStart,
) -> Result<Result<AttemptId, Phase>, StoreError> {
    let stop = |cause: &dyn std::fmt::Display| {
        eprintln!("dispatch task={task} attempt_started error={cause}");
        apply(engine, task, Event::Settle(Settlement::NotReady))?;
        apply(engine, task, Event::Stop).map(Err)
    };
    let id = match opened_attempt(engine, task, generation) {
        Ok(id) => id,
        Err(e) => return stop(&e),
    };
    let started = engine.store().attempt_started(&id, start);
    match started {
        Ok(()) => {
            eprintln!(
                "dispatch task={task} attempt={id} started workspace={}",
                start.workspace.display()
            );
            Ok(Ok(id))
        }
        Err(e @ StoreError::WorkspaceLeased { .. }) => {
            eprintln!("dispatch task={task} attempt_started error={e}");
            abandon(engine, task, AbandonReason::WorkDirUnavailable).map(Err)
        }
        Err(e) => stop(&e),
    }
}

/// The attempt's `on_start`: record the worker's `(pid, start_ticks)` on its row. A store
/// error is printed, not raised: the child is already running.
fn record_pid(engine: &Engine, id: &AttemptId, pid: u32, start_ticks: u64) {
    let task = id.task_id();
    eprintln!("dispatch task={task} attempt={id} pid={pid} start_ticks={start_ticks}");
    if let Err(e) = engine.store().attempt_pid(id, pid, start_ticks) {
        eprintln!("dispatch task={task} attempt={id} attempt_pid error={e}");
    }
}

/// Why the row `Dispatch` opened could not be read back as this generation's.
#[derive(Debug, thiserror::Error)]
enum OpenedFault {
    /// The ledger failed.
    #[error("ledger: {0}")]
    Store(#[from] StoreError),
    /// No running row for the task after `Dispatch`.
    #[error("no open attempt after dispatch")]
    Missing,
    /// The open row's generation is not the one the work dir was built for.
    #[error("open attempt {found} is not generation {expected}")]
    Generation {
        /// The row read back.
        found: AttemptId,
        /// The generation the work dir names.
        expected: u64,
    },
}

/// The id of the attempt `Dispatch` just opened for `task`, read from the ledger (never
/// formatted here), checked against the generation the work dir was built for.
fn opened_attempt(engine: &Engine, task: &TaskId, expected: u64) -> Result<AttemptId, OpenedFault> {
    let row = engine
        .store()
        .open_attempts()?
        .into_iter()
        .find(|row| row.task_id == *task)
        .ok_or(OpenedFault::Missing)?;
    if row.generation == expected {
        Ok(row.id)
    } else {
        Err(OpenedFault::Generation {
            found: row.id,
            expected,
        })
    }
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
        assert_eq!(
            steps[1].kind,
            StepKind::Unsupported {
                kind: "model".into()
            }
        );
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
            StepKind::Unsupported {
                kind: "model".into()
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
        let attempt = hee4_contracts::Budgets::DEFAULT.attempt;
        let (default, ceiling) = (attempt.timebox_default(), Duration::from_hours(24));
        assert_eq!(timebox("10s", default, ceiling), Duration::from_secs(10));
        assert_eq!(
            timebox("60 min", default, ceiling),
            Duration::from_secs(3600)
        );
        assert_eq!(timebox("soon", default, ceiling), Duration::from_secs(120));
        assert_eq!(
            timebox("soon", Duration::from_secs(7), ceiling),
            Duration::from_secs(7)
        );
    }

    /// `attempt.deadline_ms` bounds every timebox: minutes whose seconds overflow `u64`, a
    /// count wider than `u64`, a plain large count and a default above it all clamp to it.
    #[test]
    fn timebox_overflow_and_excess_clamp_to_the_attempt_deadline() {
        let attempt = hee4_contracts::Budgets::DEFAULT.attempt;
        let (default, ceiling) = (attempt.timebox_default(), attempt.deadline());
        assert_eq!(ceiling, Duration::from_mins(20));
        let overflow = format!("{} min", u64::MAX / 60 + 1);
        assert_eq!(timebox(&overflow, default, ceiling), ceiling);
        assert_eq!(
            timebox("99999999999999999999999 s", default, ceiling),
            ceiling
        );
        assert_eq!(timebox("60 min", default, ceiling), ceiling);
        assert_eq!(timebox("1201s", default, ceiling), ceiling);
        assert_eq!(
            timebox("1199s", default, ceiling),
            Duration::from_secs(1199)
        );
        assert_eq!(
            timebox("", Duration::from_secs(5000), ceiling),
            ceiling,
            "a default above the deadline is clamped too"
        );
        let one_ms = Duration::from_millis(1);
        assert_eq!(timebox("2s", default, one_ms), one_ms);
    }
}
