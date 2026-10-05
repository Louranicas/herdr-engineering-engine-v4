//! K5 policy over the K0h spawn door. `busctl` is spawned here and nowhere else (one door): every
//! call goes through `hee4_host::spawn::{plan, run}` under a permit whose scope lists the busctl
//! binary and the user bus, with the bus bound read-write as a listed socket and no network. The
//! runner holds no `Store` and no `Engine`; it returns values the actions (K6) commit through K1.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use hee4_contracts::{
    Evidence, GitSha, Observation, Outcome, Sha256Hex, ToolId, ToolName, ToolVersion,
};
use hee4_host::spawn::{self, Command, NamespacePlan, Permit, ReceiptId, SpawnOutcome, SpawnScope};
use hee4_worker::namespace::RO_BINDS;

/// Budgets stand-in (wave-3 app-runtime-budgets): the wall time one busctl child may take, and
/// the settle deadline of an act's read-back. No call site spells this number.
pub const PROBE_TIMEOUT: Duration = Duration::from_secs(5);
/// Budgets stand-in (wave-3 app-runtime-budgets): the most stdout bytes a probe child may
/// produce before the reply is `resource_exhausted`. No call site spells this number.
pub const PROBE_STDOUT_MAX: usize = 4096;
/// The busctl binary (MEASURED present on this host, systemd 261).
pub const BUSCTL: &str = "/usr/bin/busctl";
/// Env var pinning the busctl digest; absent, the digest measured at serve start is the pin.
pub const BUSCTL_SHA256_ENV: &str = "HEE4_BUSCTL_SHA256";
/// The observation source of every probe.
pub const SOURCE: &str = "service.probe";
/// The read that settles an unconfirmed act.
pub const SETTLING_READ: &str = "service.inspect";

const SYSTEMD_DEST: &str = "org.freedesktop.systemd1";
const MANAGER_PATH: &str = "/org/freedesktop/systemd1";
const MANAGER_IFACE: &str = "org.freedesktop.systemd1.Manager";
const UNIT_PATH_PREFIX: &str = "/org/freedesktop/systemd1/unit/";
const SETTLE_POLL: Duration = Duration::from_millis(200);

/// What one probe or act may spend.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProbeBudget {
    /// Kill the busctl child after this long; an act must settle within it too.
    pub timeout: Duration,
    /// Most stdout bytes the child may produce.
    pub stdout_max: usize,
}

impl ProbeBudget {
    /// The stand-in budget: [`PROBE_TIMEOUT`] and [`PROBE_STDOUT_MAX`].
    pub const DEFAULT: Self = Self {
        timeout: PROBE_TIMEOUT,
        stdout_max: PROBE_STDOUT_MAX,
    };
}

impl Default for ProbeBudget {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// The probe catalogue: which unit property a probe reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProbeId {
    /// `org.freedesktop.systemd1.Unit.ActiveState`.
    ActiveState,
    /// `org.freedesktop.systemd1.Unit.SubState`.
    SubState,
    /// `org.freedesktop.systemd1.Service.MainPID`.
    MainPid,
}

impl ProbeId {
    /// Every probe, in wire order.
    pub const ALL: [Self; 3] = [Self::ActiveState, Self::SubState, Self::MainPid];

    /// The probe named `wire`, if catalogued.
    #[must_use]
    pub fn parse(wire: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|p| p.as_str() == wire)
    }

    /// The wire spelling, also the evidence label.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ActiveState => "active_state",
            Self::SubState => "sub_state",
            Self::MainPid => "main_pid",
        }
    }

    const fn property(self) -> &'static str {
        match self {
            Self::ActiveState => "ActiveState",
            Self::SubState => "SubState",
            Self::MainPid => "MainPID",
        }
    }

    const fn interface(self) -> &'static str {
        match self {
            Self::ActiveState | Self::SubState => "org.freedesktop.systemd1.Unit",
            Self::MainPid => "org.freedesktop.systemd1.Service",
        }
    }
}

/// The action domain of `service.action`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnitAction {
    /// `StartUnit`; settles when `ActiveState` reads `active`.
    Start,
    /// `StopUnit`; settles when `ActiveState` reads `inactive`.
    Stop,
    /// `RestartUnit`; settles when `ActiveState` reads `active`.
    Restart,
}

impl UnitAction {
    /// Every action, in wire order.
    pub const ALL: [Self; 3] = [Self::Start, Self::Stop, Self::Restart];

    /// The action named `wire`, if in the domain.
    #[must_use]
    pub fn parse(wire: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|a| a.as_str() == wire)
    }

    /// The wire spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Start => "start",
            Self::Stop => "stop",
            Self::Restart => "restart",
        }
    }

    /// The manager method.
    #[must_use]
    pub const fn method(self) -> &'static str {
        match self {
            Self::Start => "StartUnit",
            Self::Stop => "StopUnit",
            Self::Restart => "RestartUnit",
        }
    }

    /// The `ActiveState` the act settles at.
    #[must_use]
    pub const fn settled_state(self) -> &'static str {
        match self {
            Self::Start | Self::Restart => "active",
            Self::Stop => "inactive",
        }
    }
}

/// Why a probe produced no observation.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ProbeFault {
    /// The busctl binary's digest now is not the pinned one (`unavailable`, because
    /// "busctl digest").
    #[error("busctl digest {found} is not the pinned {expected}")]
    Digest {
        /// The pin (env or serve-start measure), or `unmeasured`.
        expected: String,
        /// The digest read now, or why it could not be read.
        found: String,
    },
    /// This build has no git head, so an observation cannot name a commit (`unavailable`).
    #[error("build head is unknown")]
    HeadUnknown,
    /// `$XDG_RUNTIME_DIR/bus` is absent, so busctl has nothing to connect to (`unavailable`).
    #[error("user bus absent (XDG_RUNTIME_DIR unset)")]
    NoBus,
    /// The child wrote more stdout than the budget allows (`resource_exhausted`).
    #[error("probe stdout {bytes} B over the {bound} B bound")]
    StdoutOverBound {
        /// Bytes the child wrote.
        bytes: usize,
        /// The bound.
        bound: usize,
    },
    /// The spawn door refused the plan or the work dir could not be made (`internal`).
    #[error("spawn door: {0}")]
    Door(String),
    /// An observation field did not parse (`internal`).
    #[error("observation field: {0}")]
    Field(String),
}

/// Why an act produced no outcome.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ActFault {
    /// The digest, head, bus, bound or door refusal a probe would raise.
    #[error(transparent)]
    Probe(#[from] ProbeFault),
    /// The manager answered the call with an error naming the unit as absent (`Unit X not
    /// found.` / `not loaded.`): nothing happened (`not_found` at `/body/unit_id`).
    #[error("manager refused: {0}")]
    UnitAbsent(String),
    /// The manager answered the call with another unit error (`Unit X is masked.`, an invalid
    /// name): nothing happened (`unavailable`, because "manager refused").
    #[error("manager refused: {0}")]
    ManagerRefused(String),
    /// The call was sent but its effect was not read back as settled: `effect_unknown`, retry
    /// after the read `settling_read` names. Nothing is committed.
    #[error("effect unknown ({detail}); settle by {settling_read}")]
    EffectUnknown {
        /// The read that settles it.
        settling_read: &'static str,
        /// What was seen.
        detail: String,
    },
}

/// What a settled act produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionOutcome {
    /// The manager's job object path for the call.
    pub owner_job_id: String,
    /// The `ActiveState` read back after the call.
    pub observed_state: String,
    /// The read-back as an observation (committed as the service's useful health).
    pub health: Observation,
}

/// The one spawner of busctl. `probe` reads one property into an observation; `act` calls the
/// manager and reads `ActiveState` back until it settles.
pub trait ServiceRunner: Send + Sync {
    /// One bounded property read of `unit`, sealed to `input` (the request body's digest).
    ///
    /// # Errors
    /// [`ProbeFault`].
    fn probe(
        &self,
        unit: &str,
        probe: ProbeId,
        input: &Sha256Hex,
        budget: &ProbeBudget,
    ) -> Result<Observation, ProbeFault>;

    /// One managed act on `unit` with its read-back.
    ///
    /// # Errors
    /// [`ActFault`].
    fn act(
        &self,
        unit: &str,
        action: UnitAction,
        input: &Sha256Hex,
        budget: &ProbeBudget,
    ) -> Result<ActionOutcome, ActFault>;
}

/// The systemd object path of `unit`: `/org/freedesktop/systemd1/unit/` + the bus-label escape
/// (ASCII alphanumerics kept, except a leading digit; every other byte as `_` + two lower hex
/// digits). MEASURED: `hee4.service` is `hee4_2eservice`.
#[must_use]
pub fn unit_object_path(unit: &str) -> String {
    let mut path = String::from(UNIT_PATH_PREFIX);
    for (i, b) in unit.bytes().enumerate() {
        let plain = b.is_ascii_alphabetic() || (b.is_ascii_digit() && i > 0);
        if plain {
            path.push(char::from(b));
        } else {
            let _ = write!(path, "_{b:02x}");
        }
    }
    path
}

/// The value of a `busctl` reply line: `s "active"` is `active`, `o "/p"` is `/p`, `u 2055515` is
/// `2055515`.
fn property_value(stdout: &[u8]) -> Option<String> {
    let line = std::str::from_utf8(stdout).ok()?.lines().next()?.trim();
    let (ty, rest) = line.split_once(' ')?;
    match ty {
        "s" | "o" => rest
            .strip_prefix('"')
            .and_then(|r| r.strip_suffix('"'))
            .map(str::to_owned),
        _ => Some(rest.to_owned()),
    }
}

/// A definite refusal in a failed `busctl call`'s stderr: the manager's error reply about the
/// unit (MEASURED on systemd 261: `Call failed: Unit X not loaded.`, `... not found.`, `Call
/// failed: Unit name X is not valid.`). Anything else (a timeout, a dropped connection, an
/// unparsed line) is `None`: the call may have taken effect.
fn call_refusal(stderr: &[u8]) -> Option<ActFault> {
    let line = std::str::from_utf8(stderr).ok()?.lines().next()?.trim();
    let reply = line.strip_prefix("Call failed: ")?;
    if !reply.starts_with("Unit ") {
        return None;
    }
    let detail = reply.to_owned();
    Some(
        if reply.ends_with(" not found.") || reply.ends_with(" not loaded.") {
            ActFault::UnitAbsent(detail)
        } else {
            ActFault::ManagerRefused(detail)
        },
    )
}

/// The object path of a `busctl call` reply `o "/org/freedesktop/systemd1/job/N"`.
fn job_path(stdout: &[u8]) -> Option<String> {
    let value = property_value(stdout)?;
    let line = std::str::from_utf8(stdout).ok()?.lines().next()?.trim();
    (line.starts_with("o ") && value.starts_with('/')).then_some(value)
}

fn millis(d: Duration) -> u64 {
    u64::try_from(d.as_millis()).unwrap_or(u64::MAX)
}

fn nanos_now() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos())
}

/// Run busctl with `args` through the spawn door: a fresh permit for the busctl binary (and the
/// bus when given), the K2 read-only binds, a work dir `<work>/service/<nanos>` that is removed
/// after the run, the bus as the one listed socket, no model door, no network.
///
/// # Errors
/// [`ProbeFault::Door`] when the work dir cannot be made or the door refuses the plan.
fn run_busctl(
    work_root: &Path,
    bus: Option<&Path>,
    args: &[&str],
    timeout: Duration,
) -> Result<Result<SpawnOutcome, spawn::SpawnError>, ProbeFault> {
    let nanos = nanos_now();
    let work = work_root.join("service").join(format!("{nanos:x}"));
    std::fs::create_dir_all(&work).map_err(|e| ProbeFault::Door(e.to_string()))?;
    let sockets: Vec<PathBuf> = bus.map(Path::to_path_buf).into_iter().collect();
    let permit = Permit::mint(
        ReceiptId(format!("r-service-{nanos:x}")),
        SpawnScope {
            programs: vec![PathBuf::from(BUSCTL)],
            sockets: sockets.clone(),
        },
    );
    let ro_binds: Vec<PathBuf> = RO_BINDS.iter().map(PathBuf::from).collect();
    let mut listed_mounts = ro_binds.clone();
    listed_mounts.push(work.clone());
    listed_mounts.extend(sockets.iter().cloned());
    let ns = NamespacePlan {
        listed_mounts,
        ro_binds,
        work_dir: work.clone(),
        model_door: None,
        sockets,
        timeout,
    };
    let command = Command {
        program: PathBuf::from(BUSCTL),
        args: args.iter().map(|a| (*a).to_owned()).collect(),
    };
    let plan = spawn::plan(&permit, command, ns).map_err(|e| ProbeFault::Door(e.to_string()))?;
    let outcome = spawn::run(plan);
    let _ = std::fs::remove_dir_all(&work);
    Ok(outcome)
}

/// One property read, before it becomes an observation.
struct Read {
    outcome: Outcome,
    stdout: Vec<u8>,
    value: Option<String>,
    elapsed: Duration,
}

/// The real runner: busctl through bwrap with the user bus bound.
#[derive(Debug)]
pub struct BusctlRunner {
    work: PathBuf,
    bus: Option<PathBuf>,
    pin: Option<Sha256Hex>,
    version: ToolVersion,
}

impl BusctlRunner {
    /// A runner over `work` (the engine's work root), `bus` (`$XDG_RUNTIME_DIR/bus`, see
    /// [`BusctlRunner::bus_from_env`]), the digest `pin` every call is checked against, and
    /// the tool version recorded in every observation.
    #[must_use]
    pub const fn new(
        work: PathBuf,
        bus: Option<PathBuf>,
        pin: Option<Sha256Hex>,
        version: ToolVersion,
    ) -> Self {
        Self {
            work,
            bus,
            pin,
            version,
        }
    }

    /// `$XDG_RUNTIME_DIR/bus` when the variable is set and the path is a socket now.
    #[must_use]
    pub fn bus_from_env() -> Option<PathBuf> {
        use std::os::unix::fs::FileTypeExt as _;
        let bus = std::env::var_os("XDG_RUNTIME_DIR")
            .filter(|v| !v.is_empty())
            .map(|v| PathBuf::from(v).join("bus"))?;
        std::fs::metadata(&bus)
            .is_ok_and(|m| m.file_type().is_socket())
            .then_some(bus)
    }

    /// SHA-256 of the busctl binary's bytes, now.
    ///
    /// # Errors
    /// The read.
    pub fn measure_digest() -> std::io::Result<Sha256Hex> {
        Ok(Sha256Hex::digest(&std::fs::read(BUSCTL)?))
    }

    /// The first line of `busctl --version`, run through the door with no socket, with its
    /// whitespace folded to `-` so it is one token (`systemd-261-(261.2-1-arch)` here).
    ///
    /// # Errors
    /// [`ProbeFault::Door`] when the door refuses; [`ProbeFault::Field`] when the child failed
    /// or its first line is not a token.
    pub fn measure_version(work_root: &Path) -> Result<ToolVersion, ProbeFault> {
        let out = run_busctl(work_root, None, &["--version"], PROBE_TIMEOUT)?
            .map_err(|e| ProbeFault::Field(format!("busctl --version: {e}")))?;
        if out.exit != Some(0) {
            return Err(ProbeFault::Field(format!(
                "busctl --version exit {:?}: {}",
                out.exit,
                String::from_utf8_lossy(&out.stderr)
            )));
        }
        let folded = String::from_utf8_lossy(&out.stdout)
            .lines()
            .next()
            .unwrap_or_default()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join("-");
        folded
            .parse()
            .map_err(|e| ProbeFault::Field(format!("busctl version {folded:?}: {e}")))
    }

    fn check_digest(&self) -> Result<(), ProbeFault> {
        let expected = self
            .pin
            .map_or_else(|| "unmeasured".to_owned(), |p| p.to_string());
        let found = match Self::measure_digest() {
            Ok(d) => d,
            Err(e) => {
                return Err(ProbeFault::Digest {
                    expected,
                    found: format!("unreadable: {e}"),
                });
            }
        };
        if self.pin == Some(found) {
            Ok(())
        } else {
            Err(ProbeFault::Digest {
                expected,
                found: found.to_string(),
            })
        }
    }

    fn head() -> Result<GitSha, ProbeFault> {
        crate::HEAD.parse().map_err(|_| ProbeFault::HeadUnknown)
    }

    fn bus(&self) -> Result<&Path, ProbeFault> {
        self.bus.as_deref().ok_or(ProbeFault::NoBus)
    }

    fn read(&self, unit: &str, probe: ProbeId, budget: &ProbeBudget) -> Result<Read, ProbeFault> {
        let bus = self.bus()?;
        let path = unit_object_path(unit);
        let args = [
            "--user",
            "get-property",
            SYSTEMD_DEST,
            path.as_str(),
            probe.interface(),
            probe.property(),
        ];
        let started = Instant::now();
        let out = run_busctl(&self.work, Some(bus), &args, budget.timeout)?;
        let elapsed = started.elapsed();
        match out {
            Err(_) => Ok(Read {
                outcome: Outcome::Error,
                stdout: Vec::new(),
                value: None,
                elapsed,
            }),
            Ok(out) => {
                if out.stdout.len() > budget.stdout_max {
                    return Err(ProbeFault::StdoutOverBound {
                        bytes: out.stdout.len(),
                        bound: budget.stdout_max,
                    });
                }
                let value = (out.exit == Some(0))
                    .then(|| property_value(&out.stdout))
                    .flatten();
                Ok(Read {
                    outcome: if value.is_some() {
                        Outcome::Pass
                    } else {
                        Outcome::Fail
                    },
                    stdout: out.stdout,
                    value,
                    elapsed,
                })
            }
        }
    }

    /// One property read as an observation plus the parsed value (the act's settle loop reads
    /// the value; the wire sees the observation).
    fn observe(
        &self,
        unit: &str,
        probe: ProbeId,
        input: &Sha256Hex,
        budget: &ProbeBudget,
    ) -> Result<(Observation, Option<String>), ProbeFault> {
        self.check_digest()?;
        let head_sha = Self::head()?;
        let field = |e: hee4_contracts::Refusal| ProbeFault::Field(e.to_string());
        let read = self.read(unit, probe, budget)?;
        let observation = Observation {
            source: SOURCE.parse().map_err(field)?,
            input_sha256: *input,
            tool: ToolId {
                name: "busctl".parse::<ToolName>().map_err(field)?,
                version: self.version.clone(),
            },
            head_sha,
            outcome: read.outcome,
            evidence: vec![Evidence {
                label: probe.as_str().parse().map_err(field)?,
                sha256: Sha256Hex::digest(&read.stdout),
            }],
            advisory: false,
            elapsed_ms: millis(read.elapsed),
            budget_ms: millis(budget.timeout),
        };
        Ok((observation, read.value))
    }
}

impl ServiceRunner for BusctlRunner {
    fn probe(
        &self,
        unit: &str,
        probe: ProbeId,
        input: &Sha256Hex,
        budget: &ProbeBudget,
    ) -> Result<Observation, ProbeFault> {
        self.observe(unit, probe, input, budget).map(|(o, _)| o)
    }

    fn act(
        &self,
        unit: &str,
        action: UnitAction,
        input: &Sha256Hex,
        budget: &ProbeBudget,
    ) -> Result<ActionOutcome, ActFault> {
        self.check_digest()?;
        Self::head()?;
        let bus = self.bus()?;
        let started = Instant::now();
        let args = [
            "--user",
            "call",
            SYSTEMD_DEST,
            MANAGER_PATH,
            MANAGER_IFACE,
            action.method(),
            "ss",
            unit,
            "replace",
        ];
        let unknown = |detail: String| ActFault::EffectUnknown {
            settling_read: SETTLING_READ,
            detail,
        };
        let out = run_busctl(&self.work, Some(bus), &args, budget.timeout)?
            .map_err(|e| unknown(format!("{} call: {e}", action.method())))?;
        if out.exit != Some(0) {
            if let Some(refused) = call_refusal(&out.stderr) {
                return Err(refused);
            }
            return Err(unknown(format!(
                "{} exit {:?}: {}",
                action.method(),
                out.exit,
                String::from_utf8_lossy(&out.stderr).trim()
            )));
        }
        let owner_job_id = job_path(&out.stdout).ok_or_else(|| {
            unknown(format!(
                "{} reply is not a job path: {}",
                action.method(),
                String::from_utf8_lossy(&out.stdout).trim()
            ))
        })?;
        let deadline = started + budget.timeout;
        loop {
            // The call reached the manager: any read-back fault leaves the effect unknown.
            let (health, value) = self
                .observe(unit, ProbeId::ActiveState, input, budget)
                .map_err(|e| unknown(format!("{unit} read-back: {e}")))?;
            if value.as_deref() == Some(action.settled_state()) {
                return Ok(ActionOutcome {
                    owner_job_id,
                    observed_state: action.settled_state().to_owned(),
                    health,
                });
            }
            if Instant::now() >= deadline {
                return Err(unknown(format!(
                    "{unit} ActiveState read {value:?}, not {} within {:?}",
                    action.settled_state(),
                    budget.timeout
                )));
            }
            std::thread::sleep(SETTLE_POLL);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn service_unit_path_escapes_hee4_drive() {
        assert_eq!(
            unit_object_path("hee4-drive.service"),
            "/org/freedesktop/systemd1/unit/hee4_2ddrive_2eservice"
        );
        assert_eq!(
            unit_object_path("hee4.service"),
            "/org/freedesktop/systemd1/unit/hee4_2eservice"
        );
        assert_eq!(
            unit_object_path("1st_unit.service"),
            "/org/freedesktop/systemd1/unit/_31st_5funit_2eservice"
        );
    }

    #[test]
    fn service_property_and_job_lines_parse() {
        assert_eq!(property_value(b"s \"active\"\n").as_deref(), Some("active"));
        assert_eq!(property_value(b"u 2055515\n").as_deref(), Some("2055515"));
        assert_eq!(property_value(b""), None);
        assert_eq!(property_value(b"garbage"), None);
        assert_eq!(
            job_path(b"o \"/org/freedesktop/systemd1/job/42\"\n").as_deref(),
            Some("/org/freedesktop/systemd1/job/42")
        );
        assert_eq!(job_path(b"s \"active\"\n"), None);
        assert_eq!(ProbeId::parse("main_pid"), Some(ProbeId::MainPid));
        assert_eq!(ProbeId::parse("MainPID"), None);
        assert_eq!(UnitAction::parse("restart"), Some(UnitAction::Restart));
        assert_eq!(UnitAction::parse("reload"), None);
    }

    #[test]
    fn service_call_refusals_parse_only_manager_unit_errors() {
        assert_eq!(
            call_refusal(b"Call failed: Unit hee4-nonexistent-xyz.service not loaded.\n"),
            Some(ActFault::UnitAbsent(
                "Unit hee4-nonexistent-xyz.service not loaded.".into()
            ))
        );
        assert!(matches!(
            call_refusal(b"Call failed: Unit hee4-nonexistent-xyz.service not found.\n"),
            Some(ActFault::UnitAbsent(_))
        ));
        assert!(matches!(
            call_refusal(b"Call failed: Unit name bad name is not valid.\n"),
            Some(ActFault::ManagerRefused(_))
        ));
        assert_eq!(call_refusal(b"Call failed: Connection timed out\n"), None);
        assert_eq!(
            call_refusal(b"Failed to connect to bus: No such file\n"),
            None
        );
        assert_eq!(call_refusal(b""), None);
    }

    /// Against the real busctl through the real door, reading `hee4.service`'s `ActiveState`
    /// (a read; `inactive` on a host without the unit is still a Pass). Prints UNMEASURED
    /// and passes when the binary, the bus or the build head is absent.
    #[test]
    fn service_real_busctl_probe_reads_active_state() -> Result<(), Box<dyn std::error::Error>> {
        let Some(bus) = BusctlRunner::bus_from_env() else {
            println!("UNMEASURED: $XDG_RUNTIME_DIR/bus absent; real busctl probe skipped");
            return Ok(());
        };
        if !Path::new(BUSCTL).exists() || !Path::new(spawn::BWRAP).exists() {
            println!(
                "UNMEASURED: {BUSCTL} or {} absent; real busctl probe skipped",
                spawn::BWRAP
            );
            return Ok(());
        }
        if crate::HEAD.parse::<GitSha>().is_err() {
            println!("UNMEASURED: build head unknown; real busctl probe skipped");
            return Ok(());
        }
        let work = std::env::temp_dir().join(format!("hee4-runner-{}", std::process::id()));
        std::fs::create_dir_all(&work)?;
        let version = BusctlRunner::measure_version(&work)?;
        println!("busctl_version={version}");
        let runner = BusctlRunner::new(
            work.clone(),
            Some(bus),
            Some(BusctlRunner::measure_digest()?),
            version,
        );
        let input = Sha256Hex::digest(b"{}");
        let (obs, value) = runner.observe(
            "hee4.service",
            ProbeId::ActiveState,
            &input,
            &ProbeBudget::DEFAULT,
        )?;
        println!("active_state={value:?} elapsed_ms={}", obs.elapsed_ms);
        assert_eq!(obs.outcome, Outcome::Pass, "{obs:?}");
        assert_eq!(obs.evidence[0].label.as_str(), "active_state");
        assert!(value.is_some_and(|v| {
            ["active", "inactive", "failed", "activating", "deactivating"].contains(&v.as_str())
        }));
        let wrong = BusctlRunner::new(
            work.clone(),
            runner.bus.clone(),
            Some(Sha256Hex::GENESIS),
            runner.version.clone(),
        );
        assert!(matches!(
            wrong.probe(
                "hee4.service",
                ProbeId::ActiveState,
                &input,
                &ProbeBudget::DEFAULT
            ),
            Err(ProbeFault::Digest { .. })
        ));
        let _ = std::fs::remove_dir_all(&work);
        Ok(())
    }
}
