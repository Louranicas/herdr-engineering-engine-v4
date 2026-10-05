//! The synchronous dispatcher (RL-2, skeleton slice): one `admitted` task at a time,
//! route → namespace → permit → attempt → observe → `decide` → seal → settle.
//!
//! Every state change is `Store::apply`; the only verdict is `hee4_evidence::decide_and_seal`'s.

use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use hee4_contracts::bounds::MAX_VIEW_ITEMS;
use hee4_contracts::{
    AbandonReason, Brief, BriefField, Event, GitSha, Observation, Phase, ReceiptId, Resolution,
    Settlement, Sha256Hex, SourceId, TaskId, Verdict, VerifyLine,
};
use hee4_core::backup::{BackupError, SameDisk, backup_to};
use hee4_core::roster::RosterDefinition;
use hee4_core::{AttemptId, AttemptStart, StoreError};
use hee4_evidence::ddf::{self, AdapterError, Diff, TaskObservation};
use hee4_evidence::{Identities, Identity, Source, Subject, Why, decide_and_seal, observation_id};
use hee4_host::clock::{Clock, SystemClock};
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
    /// The due DC-22 backup failed; no `Dispatch` was applied and the task stays `admitted`.
    #[error("backup: {0}")]
    Backup(#[from] BackupFault),
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

/// DC-22 freshness: a backup older than this is stale, so the next dispatch takes one first
/// (plan/DECISIONS.md:199-204). Pending K0 field `backup.freshness_ms`; the one copy.
pub const DC22_FRESHNESS: Duration = Duration::from_mins(15);

/// DC-22 batch boundary: after this many dispatches since the last backup, the next dispatch
/// takes one first (plan/DECISIONS.md:199-204). Pending K0 field `backup.batch_tasks`; the one copy.
pub const DC22_BATCH_TASKS: u64 = 8;

/// The file under the backup root that gets one line per backup run (K6's, not K1's).
pub const BACKUP_LOG: &str = "backup.log";

/// Why a DC-22 backup is due: one variant per DC-22 bullet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackupTrigger {
    /// No backup has been taken into this root.
    Start,
    /// The last backup was taken by another build (`head` moved): before an upgrade serves.
    Upgrade,
    /// The last backup is older than [`DC22_FRESHNESS`].
    Stale,
    /// [`DC22_BATCH_TASKS`] dispatches since the last backup.
    Batch,
}

impl BackupTrigger {
    /// The `trigger=` word in `backup.log`.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Start => "start",
            Self::Upgrade => "upgrade",
            Self::Stale => "stale",
            Self::Batch => "batch",
        }
    }
}

/// The first 12 hex digits of a build commit, as `backup.log` records it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Head12([u8; 12]);

impl Head12 {
    /// The first 12 digits of `head`.
    #[must_use]
    pub fn of(head: &GitSha) -> Option<Self> {
        Self::parse(head.as_str().get(..12)?)
    }

    /// Exactly 12 lowercase hex digits, else `None`.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        let bytes: [u8; 12] = text.as_bytes().try_into().ok()?;
        bytes
            .iter()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(b))
            .then_some(Self(bytes))
    }

    /// Byte equality, usable in a `const fn`.
    #[must_use]
    pub const fn same(&self, other: &Self) -> bool {
        let mut i = 0;
        while i < 12 {
            if self.0[i] != other.0[i] {
                return false;
            }
            i += 1;
        }
        true
    }

    /// The 12 digits.
    #[must_use]
    pub fn as_str(&self) -> &str {
        std::str::from_utf8(&self.0).unwrap_or("unknown")
    }
}

/// The last successful backup in the root, as its `backup.log` line records it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LastBackup {
    /// When it started, since the Unix epoch (from the K1 id `b-<ts_ms hex>-<boot hex>`).
    pub ts: Duration,
    /// The build that took it.
    pub head: Head12,
}

/// Everything [`backup_due`] reads; time comes in as a value, never from a clock here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BackupFacts {
    /// The last successful backup, if any.
    pub last: Option<LastBackup>,
    /// Successful `Dispatch` events since it.
    pub dispatched_since: u64,
    /// Now, since the Unix epoch.
    pub now: Duration,
    /// This build.
    pub head: Head12,
}

/// The DC-22 rule, pure and total: no backup → `Start`; another build's → `Upgrade`; older
/// than [`DC22_FRESHNESS`] (strictly) → `Stale`; at least [`DC22_BATCH_TASKS`] dispatches since
/// → `Batch`; else none.
#[must_use]
pub const fn backup_due(facts: &BackupFacts) -> Option<BackupTrigger> {
    let Some(last) = &facts.last else {
        return Some(BackupTrigger::Start);
    };
    if !last.head.same(&facts.head) {
        return Some(BackupTrigger::Upgrade);
    }
    if facts.now.saturating_sub(last.ts).as_millis() > DC22_FRESHNESS.as_millis() {
        return Some(BackupTrigger::Stale);
    }
    if facts.dispatched_since >= DC22_BATCH_TASKS {
        return Some(BackupTrigger::Batch);
    }
    None
}

/// Why a backup run failed or the backup root could not be opened.
#[derive(Debug, thiserror::Error)]
pub enum BackupFault {
    /// K1's `backup_to` refused.
    #[error("{0}")]
    Take(#[from] BackupError),
    /// The root or `backup.log` could not be created, read or appended.
    #[error("{BACKUP_LOG} at {path}: {source}")]
    Log {
        /// The path.
        path: PathBuf,
        /// The OS's answer.
        #[source]
        source: std::io::Error,
    },
    /// The build commit is `unknown`, so no backup can name its head.
    #[error("head unknown: the build has no commit")]
    HeadUnknown,
}

/// The `reason=` word for a K1 backup or restore refusal.
#[must_use]
pub const fn backup_error_name(e: &BackupError) -> &'static str {
    match e {
        BackupError::Store(_) => "store",
        BackupError::Io { .. } => "io",
        BackupError::Json(_) => "manifest_json",
        BackupError::Manifest { .. } => "manifest",
        BackupError::ObjectsOverBound { .. } => "objects_over_bound",
        BackupError::SameDevice { .. } => "same_device",
        BackupError::Incomplete { .. } => "incomplete",
        BackupError::TargetOccupied { .. } => "target_occupied",
        BackupError::DigestMismatch { .. } => "digest_mismatch",
        BackupError::ObjectsMissing { .. } => "objects_missing",
        _ => "backup_error",
    }
}

/// The DC-22 backup state the dispatcher thread owns: the root, its open `backup.log` (held
/// open so a FAIL line still lands when the root itself refuses), the last good backup and the
/// dispatches since. No thread, timer or daemon (V4-6): it runs only when `serve` or `step`
/// calls it.
#[derive(Debug)]
pub struct Backups {
    root: PathBuf,
    log: fs::File,
    last: Option<LastBackup>,
    dispatched_since: u64,
    head: Head12,
    failing: bool,
}

/// `(ts, head)` from a `backup id=b-<ts_ms hex>-.. ... head=<12> verdict=PASS` line.
fn parse_pass_line(line: &str) -> Option<LastBackup> {
    if !line.starts_with("backup ") || !line.contains(" verdict=PASS") {
        return None;
    }
    let word = |key: &str| {
        line.split(' ')
            .find_map(|w| w.strip_prefix(key))
            .map(str::to_owned)
    };
    let id = word("id=")?;
    let ts_hex = id.strip_prefix("b-")?.split('-').next()?;
    let ts_ms = u64::from_str_radix(ts_hex, 16).ok()?;
    Some(LastBackup {
        ts: Duration::from_millis(ts_ms),
        head: Head12::parse(&word("head=")?)?,
    })
}

impl Backups {
    /// Create `root` if absent, open `<root>/backup.log` for append, and read its last PASS
    /// line as the last backup (so `Upgrade` needs no manifest field).
    ///
    /// # Errors
    /// [`BackupFault::Log`] when the root or the log cannot be created or read.
    pub fn open(root: &Path, head: Head12) -> Result<Self, BackupFault> {
        let at = |path: &Path| {
            let path = path.to_path_buf();
            move |source| BackupFault::Log { path, source }
        };
        fs::create_dir_all(root).map_err(at(root))?;
        let path = root.join(BACKUP_LOG);
        let log = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(at(&path))?;
        let text = fs::read_to_string(&path).map_err(at(&path))?;
        let last = text.lines().rev().find_map(parse_pass_line);
        Ok(Self {
            root: root.to_path_buf(),
            log,
            last,
            dispatched_since: 0,
            head,
            failing: false,
        })
    }

    /// The root.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// The facts [`backup_due`] reads, at `now`.
    #[must_use]
    pub const fn facts(&self, now: Duration) -> BackupFacts {
        BackupFacts {
            last: self.last,
            dispatched_since: self.dispatched_since,
            now,
            head: self.head,
        }
    }

    /// Count one applied `Dispatch`.
    pub const fn dispatched(&mut self) {
        self.dispatched_since = self.dispatched_since.saturating_add(1);
    }

    /// Run K1's `backup_to` into the root (the ledger's device refused) and append one line to
    /// `backup.log`; on PASS reset the last backup and the dispatch count. A FAIL line is
    /// written once per failing streak (each retry still prints it to the serve log), then the
    /// error is returned.
    ///
    /// # Errors
    /// [`BackupFault::Take`] when K1 refused; [`BackupFault::Log`] when the line could not be
    /// appended.
    pub fn take(
        &mut self,
        engine: &Engine,
        trigger: BackupTrigger,
        clock: &impl Clock,
    ) -> Result<(), BackupFault> {
        let now = clock.now();
        let age = self.last.map_or_else(
            || "none".to_owned(),
            |l| now.saturating_sub(l.ts).as_secs().to_string(),
        );
        let taken = backup_to(&engine.store(), engine.work(), &self.root, SameDisk::Refuse);
        let head = self.head.as_str().to_owned();
        let trigger = trigger.name();
        let (line, result) = match taken {
            Ok(report) => (
                format!(
                    "backup id={} objects={} age_s={age} trigger={trigger} head={head} verdict=PASS",
                    report.id, report.objects_n
                ),
                Ok(report),
            ),
            Err(e) => (
                format!(
                    "backup id=none objects=none age_s={age} trigger={trigger} head={head} verdict=FAIL reason={}",
                    backup_error_name(&e)
                ),
                Err(e),
            ),
        };
        eprintln!("{line}");
        let first_failure = result.is_err() && !self.failing;
        if result.is_ok() || first_failure {
            writeln!(self.log, "{line}").map_err(|source| BackupFault::Log {
                path: self.root.join(BACKUP_LOG),
                source,
            })?;
        }
        match result {
            Ok(report) => {
                self.failing = false;
                self.dispatched_since = 0;
                self.last = Some(LastBackup {
                    ts: u64::try_from(report.ts_ms).map_or(now, Duration::from_millis),
                    head: self.head,
                });
                Ok(())
            }
            Err(e) => {
                self.failing = true;
                Err(BackupFault::Take(e))
            }
        }
    }

    /// Take the backup [`backup_due`] names now, if any.
    ///
    /// # Errors
    /// As [`Backups::take`].
    pub fn take_if_due(&mut self, engine: &Engine, clock: &impl Clock) -> Result<(), BackupFault> {
        match backup_due(&self.facts(clock.now())) {
            Some(trigger) => self.take(engine, trigger, clock),
            None => Ok(()),
        }
    }
}

/// The brief's TIMEBOX under `budgets.attempt` ([`timebox`]): the attempt's and deep-diff-forge's.
fn brief_timebox(brief: &Brief, budgets: &hee4_contracts::Budgets) -> Duration {
    timebox(
        brief.get(BriefField::Timebox),
        budgets.attempt.timebox_default(),
        budgets.attempt.deadline(),
    )
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
/// With `backups`, the DC-22 backup [`backup_due`] names is taken immediately before
/// `apply(Dispatch)`.
///
/// # Errors
/// [`DispatchError`] when the ledger fails; the task stays where the last `apply` left it, and
/// startup reconcile owns it after a restart. [`DispatchError::Backup`] when the due backup
/// failed: no `Dispatch` was applied, the task stays `admitted` and the next step retries.
pub fn step(
    engine: &Engine,
    cfg: &Config,
    backups: Option<&mut Backups>,
) -> Result<Option<(TaskId, Phase)>, DispatchError> {
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
    let budget = brief_timebox(&brief, &budgets);
    let (ns, generation) = match workspace(engine, &task, needs_model, budget)? {
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

    dispatch(engine, &task, backups)?;
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
    let sealing = Sealing {
        receipt_id,
        permit: &permit,
        subject: &subject_of(&task, head, &brief),
        work_dir: ns.work_dir(),
        budget,
    };
    settle_and_decide(engine, &sealing, &outcome)
}

/// The DC-22 backup [`backup_due`] names, then `apply(Dispatch)`, then count it. A failed
/// backup returns before `Dispatch`: the task stays `admitted`.
fn dispatch(
    engine: &Engine,
    task: &TaskId,
    mut backups: Option<&mut Backups>,
) -> Result<(), DispatchError> {
    if let Some(b) = backups.as_deref_mut() {
        b.take_if_due(engine, &SystemClock)?;
    }
    apply(engine, task, Event::Dispatch)?;
    if let Some(b) = backups {
        b.dispatched();
    }
    Ok(())
}

/// The one subject: every observation and the receipt are bound to it. Its input is the
/// VERIFY text's digest, which every observation carries.
fn subject_of(task: &TaskId, head: GitSha, brief: &Brief) -> Subject {
    Subject {
        task_id: task.clone(),
        head_sha: head,
        input_sha256: Sha256Hex::digest(brief.get(BriefField::Verify).as_bytes()),
    }
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

/// What `settle_and_decide` seals against: one subject for every observation and the receipt.
struct Sealing<'a> {
    receipt_id: ReceiptId,
    permit: &'a Permit,
    subject: &'a Subject,
    /// The per-generation workspace (`ns.work_dir()`), diffed for deep-diff-forge.
    work_dir: &'a Path,
    /// The attempt's timebox, deep-diff-forge's budget too.
    budget: Duration,
}

/// After the attempt: settle, ask deep-diff-forge over the workspace diff, ledger each
/// observation, `decide_and_seal`, append, decide.
fn settle_and_decide(
    engine: &Engine,
    sealing: &Sealing<'_>,
    outcome: &AttemptOutcome,
) -> Result<Option<(TaskId, Phase)>, DispatchError> {
    let task = &sealing.subject.task_id;
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
    let mut observations = outcome.observations.clone();
    observations.extend(ddf_observation(sealing));
    for obs in &observations {
        let id = observation_id(task, obs)?;
        engine.store().record_observation(task, &id, obs)?;
        apply(engine, task, Event::Observe)?;
    }
    let receipt = seal(engine, sealing, &observations)?;
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

/// Why the workspace could not be diffed.
#[derive(Debug, thiserror::Error)]
pub enum DiffFault {
    /// `git` could not be run.
    #[error("git: {0}")]
    Spawn(#[from] std::io::Error),
    /// `git diff` exited non-zero.
    #[error("git diff exited {code:?}")]
    Exit {
        /// The exit code, if any.
        code: Option<i32>,
    },
}

/// The workspace's diff: `None` when `<ws>/.git` is absent (no worktree); else
/// `git diff --cached` (a fresh `git init` has no HEAD, so staged files show only here)
/// followed by `git diff`. Empty bytes mean a clean worktree.
///
/// # Errors
/// [`DiffFault`] when git cannot run or exits non-zero.
pub fn workspace_diff(ws: &Path) -> Result<Option<Vec<u8>>, DiffFault> {
    if !ws.join(".git").exists() {
        return Ok(None);
    }
    let mut bytes = Vec::new();
    for cached in [true, false] {
        let mut cmd = Command::new("git");
        cmd.arg("-C")
            .arg(ws)
            .args(["diff", "--no-color", "--no-ext-diff"]);
        if cached {
            cmd.arg("--cached");
        }
        let out = cmd.output()?;
        if !out.status.success() {
            return Err(DiffFault::Exit {
                code: out.status.code(),
            });
        }
        bytes.extend_from_slice(&out.stdout);
    }
    Ok(Some(bytes))
}

/// The `AdapterError` variant name for the skip line.
const fn adapter_error_name(e: &AdapterError) -> &'static str {
    match e {
        AdapterError::Spawn(_) => "spawn",
        AdapterError::Exit { .. } => "exit",
        AdapterError::Malformed(_) => "malformed",
        AdapterError::SealMismatch { .. } => "seal_mismatch",
        AdapterError::Timeout { .. } => "timeout",
        AdapterError::LookedAtNothing => "looked_at_nothing",
    }
}

/// K4's `ddf::for_task` over the workspace diff, bound to the one subject. An observation of any
/// outcome is returned for recording (the lattice reads it; this function never does); every
/// skip is a log line and nothing else: never a refusal, never an abandon.
fn ddf_observation(sealing: &Sealing<'_>) -> Option<Observation> {
    let task = &sealing.subject.task_id;
    let skipped = |reason: &str| {
        eprintln!("dispatch task={task} ddf=skipped reason={reason}");
        None
    };
    let bytes = match workspace_diff(sealing.work_dir) {
        Ok(bytes) => bytes,
        Err(e) => {
            eprintln!("dispatch task={task} ddf git_error={e}");
            let kind = match e {
                DiffFault::Spawn(_) => "spawn",
                DiffFault::Exit { .. } => "exit",
            };
            return skipped(&format!("git_error:{kind}"));
        }
    };
    let diff = bytes.as_deref().map_or(Diff::NoWorktree, Diff::Bytes);
    match ddf::for_task(diff, sealing.subject, &SystemClock, sealing.budget) {
        Ok(TaskObservation::Observed(obs)) => {
            eprintln!(
                "dispatch task={task} ddf=observed tool={} {}",
                obs.tool.name, obs.tool.version
            );
            Some(obs)
        }
        Ok(TaskObservation::Skipped(skip)) => skipped(skip.name()),
        Err(e) => {
            eprintln!("dispatch task={task} ddf adapter_error={e}");
            skipped(&format!("adapter_error:{}", adapter_error_name(&e)))
        }
    }
}

fn seal(
    engine: &Engine,
    sealing: &Sealing<'_>,
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
        locks: source("hee4-permit", format!("{:?}", sealing.permit).as_bytes())?,
        standards: source("gate.toml", crate::GATE_TOML)?,
    };
    let task = &sealing.subject.task_id;
    let receipt = decide_and_seal(
        store.chain_head(task)?,
        sealing.receipt_id.clone(),
        &ids,
        obs,
        sealing.subject,
    )?;
    store.append_receipt(&receipt)?;
    Ok(receipt)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn head(c: u8) -> Head12 {
        Head12([c; 12])
    }

    fn facts(last: Option<(u64, u8)>, dispatched_since: u64, now_s: u64) -> BackupFacts {
        BackupFacts {
            last: last.map(|(ts, c)| LastBackup {
                ts: Duration::from_secs(ts),
                head: head(c),
            }),
            dispatched_since,
            now: Duration::from_secs(now_s),
            head: head(b'a'),
        }
    }

    #[test]
    fn backup_due_start_when_no_backup() {
        assert_eq!(backup_due(&facts(None, 0, 0)), Some(BackupTrigger::Start));
        assert_eq!(backup_due(&facts(None, 99, 5)), Some(BackupTrigger::Start));
    }

    #[test]
    fn backup_due_upgrade_when_head_moved() {
        assert_eq!(
            backup_due(&facts(Some((100, b'b')), 0, 100)),
            Some(BackupTrigger::Upgrade)
        );
        // A moved head outranks a stale or batched backup.
        assert_eq!(
            backup_due(&facts(Some((0, b'b')), 8, 10_000)),
            Some(BackupTrigger::Upgrade)
        );
    }

    #[test]
    fn backup_due_stale_after_fifteen_minutes() {
        assert_eq!(DC22_FRESHNESS, Duration::from_mins(15));
        // Exactly 15 minutes old is still fresh; one millisecond more is stale.
        assert_eq!(backup_due(&facts(Some((1000, b'a')), 0, 1900)), None);
        let mut f = facts(Some((1000, b'a')), 0, 1900);
        f.now += Duration::from_millis(1);
        assert_eq!(backup_due(&f), Some(BackupTrigger::Stale));
        // Stale outranks batch.
        assert_eq!(
            backup_due(&facts(Some((1000, b'a')), 8, 1901)),
            Some(BackupTrigger::Stale)
        );
    }

    #[test]
    fn backup_due_batch_at_eight_dispatches() {
        assert_eq!(DC22_BATCH_TASKS, 8);
        assert_eq!(backup_due(&facts(Some((1000, b'a')), 7, 1000)), None);
        assert_eq!(
            backup_due(&facts(Some((1000, b'a')), 8, 1000)),
            Some(BackupTrigger::Batch)
        );
        assert_eq!(
            backup_due(&facts(Some((1000, b'a')), 9, 1000)),
            Some(BackupTrigger::Batch)
        );
    }

    #[test]
    fn backup_due_none_when_fresh() {
        assert_eq!(backup_due(&facts(Some((1000, b'a')), 0, 1000)), None);
        // A clock behind the backup saturates to zero age: fresh, never a wrap.
        assert_eq!(backup_due(&facts(Some((1000, b'a')), 0, 10)), None);
    }

    #[test]
    fn a_pass_line_round_trips_its_ts_and_head() {
        let line = "backup id=b-0000000003e8-0000000a objects=2 age_s=none trigger=start head=aaaaaaaaaaaa verdict=PASS";
        assert_eq!(
            parse_pass_line(line),
            Some(LastBackup {
                ts: Duration::from_millis(1000),
                head: head(b'a')
            })
        );
        assert_eq!(parse_pass_line(&line.replace("PASS", "FAIL")), None);
    }

    type R<T> = Result<T, Box<dyn std::error::Error>>;

    fn scratch(name: &str) -> R<PathBuf> {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos());
        let dir =
            std::env::temp_dir().join(format!("hee4-app-{name}-{}-{nanos}", std::process::id()));
        fs::create_dir_all(&dir)?;
        Ok(dir)
    }

    fn subject() -> R<Subject> {
        Ok(Subject {
            task_id: "t-000000000000000000000000".parse()?,
            head_sha: "0123456789abcdef0123456789abcdef01234567".parse()?,
            input_sha256: Sha256Hex::digest(b"sh: true"),
        })
    }

    #[test]
    fn workspace_diff_without_git_dir_is_no_worktree() -> R<()> {
        let ws = scratch("no-worktree")?;
        fs::write(ws.join("f"), "x")?;
        assert_eq!(workspace_diff(&ws)?, None);
        let skip = ddf::for_task(
            Diff::NoWorktree,
            &subject()?,
            &SystemClock,
            Duration::from_secs(5),
        )?;
        assert_eq!(skip, TaskObservation::Skipped(ddf::Skip::NoWorktree));
        let _ = fs::remove_dir_all(&ws);
        Ok(())
    }

    #[test]
    fn workspace_diff_of_a_clean_worktree_is_no_diff() -> R<()> {
        let ws = scratch("clean")?;
        let init = Command::new("git")
            .arg("-C")
            .arg(&ws)
            .args(["init", "-q", "."])
            .status()?;
        assert!(init.success());
        let bytes = workspace_diff(&ws)?.ok_or("no worktree")?;
        assert!(bytes.is_empty(), "{}", String::from_utf8_lossy(&bytes));
        let skip = ddf::for_task(
            Diff::Bytes(&bytes),
            &subject()?,
            &SystemClock,
            Duration::from_secs(5),
        )?;
        assert_eq!(skip, TaskObservation::Skipped(ddf::Skip::NoDiff));
        // A staged file in the same HEAD-less worktree is in the diff.
        fs::write(ws.join("f"), "x\n")?;
        let add = Command::new("git")
            .arg("-C")
            .arg(&ws)
            .args(["add", "f"])
            .status()?;
        assert!(add.success());
        let staged = workspace_diff(&ws)?.ok_or("no worktree")?;
        assert!(String::from_utf8_lossy(&staged).contains("+x"));
        let _ = fs::remove_dir_all(&ws);
        Ok(())
    }

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
