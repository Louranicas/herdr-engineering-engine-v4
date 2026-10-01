//! Trusted local Julia composition. This TH-DEV adapter grants no task authority
//! and does not provide namespace, memory or credential isolation.
use super::{Dataset, Invalid, MAX_REPORT, Report};
use crate::worker::process::{self as worker, Interruption, ProcessReport, ProcessSpec, Refusal};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs::{self, File};
use std::io::Read;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const FILES: [&str; 6] = [
    "Project.toml",
    "Manifest.toml",
    "src/HabitatAnalysis.jl",
    "src/Evaluate.jl",
    "src/Cohesion.jl",
    "bin/analysis.jl",
];
const CLEANUP_RESERVE: Duration = Duration::from_secs(10);

/// Protected caller configuration, never decoded from a request or child report.
/// Pins must originate from the admitted package owner. This value is no grant.
#[derive(Debug)]
pub struct JuliaProfile {
    /// Absolute path of the Julia executable.
    pub executable: PathBuf,
    /// Pinned digest of the executable, `sha256:` and 64 lowercase hex digits.
    pub executable_sha256: String,
    /// Canonical absolute Julia project directory holding the pinned files.
    pub project: PathBuf,
    /// Pinned digest of each of the six project files, keyed by relative path.
    pub project_files: BTreeMap<String, String>,
    /// Canonical absolute scratch directory with mode 0700; the child's cwd,
    /// `HOME`, `TMPDIR` and first depot.
    pub scratch: PathBuf,
    /// Canonical absolute read-only package depot, searched after scratch.
    pub dependency_depot: PathBuf,
}
/// Why an exchange produced no accepted report. Only `Ok(Report)` is success.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Failure {
    /// The profile was refused: a non-canonical or missing path, scratch mode
    /// other than 0700, an unpinned override file, or a pin mismatch.
    Profile,
    /// A filesystem operation failed, with its OS error number when known.
    Io(Option<i32>),
    /// The system clock is before the Unix epoch or out of range.
    Clock,
    /// The window was refused or ran out: start in the future, over 60 s, too
    /// short for the cleanup reserve, or passed.
    Deadline,
    /// The cancellation flag was observed.
    Cancelled,
    /// The dataset no longer validates at launch (for example it went stale).
    Input(Invalid),
    /// The process owner refused to start the child.
    Launch(Refusal),
    /// The child was interrupted by the process owner.
    Interrupted(Interruption),
    /// Custody did not settle: leader unreaped, group not settled, cleanup
    /// pending, or a stream not at EOF, failed or truncated.
    Unsettled,
    /// The child exited nonzero or by signal without a typed refusal of this
    /// dataset; a crash and an unbound error body both land here.
    Nonzero,
    /// The child exited 0 but wrote to stderr.
    Diagnostic,
    /// The child exited 0 cleanly but its report was refused.
    Response(Invalid),
    /// The Julia entrypoint refused this dataset: exit status 2, empty stderr
    /// and an error object bound to [`super::Dataset::digest`].
    Refused(super::JuliaCode),
}
/// Every started child returns its original custody, raw streams and signals,
/// including `PendingChild` on unknown cleanup. Dropping it is not settlement.
#[derive(Debug)]
pub struct Exchange {
    /// The accepted report, or the first reason there is none.
    pub outcome: Result<Report, Failure>,
    /// The child's raw custody report, present whenever a child was started.
    pub process: Option<ProcessReport>,
    /// The profile re-check after the child, when it failed.
    pub postflight: Option<Failure>,
    /// Wall time from the caller's start.
    pub wall_elapsed: Duration,
    /// This process owner measures wall time, not CPU usage; unknown stays unknown.
    pub cpu_elapsed: Option<Duration>,
}

/// Run one immutable dataset using the existing sole child/pipe owner. The caller
/// retains the original clock and supplies at most60s including10s cleanup reserve.
/// Exact package pins are checked before/after; same-UID concurrent hostile writes
/// are outside this development profile and require the worker isolation profile.
#[must_use]
pub fn analyze(
    profile: &JuliaProfile,
    dataset: &Dataset,
    start: Instant,
    deadline: Instant,
    cancelled: &AtomicBool,
) -> Exchange {
    let mut report = None;
    let mut postflight = None;
    let outcome = (|| {
        window(start, deadline, cancelled)?;
        let work_deadline = deadline
            .checked_sub(CLEANUP_RESERVE)
            .ok_or(Failure::Deadline)?;
        if work_deadline <= Instant::now() {
            return Err(Failure::Deadline);
        }
        validate_profile(profile, work_deadline, cancelled)?;
        Dataset::decode(dataset.raw(), unix_ms()?).map_err(Failure::Input)?;
        let spec = specification(profile, dataset);
        let observed = worker::run(&spec, work_deadline, cancelled).map_err(Failure::Launch)?;
        report = Some(observed);
        let observed = report.as_ref().ok_or(Failure::Unsettled)?;
        let result = classify(dataset, observed, cancelled);
        postflight = validate_profile(profile, deadline, cancelled).err();
        match result {
            Err(error) => Err(error),
            Ok(value) => {
                if let Some(error) = postflight {
                    return Err(error);
                }
                window(start, deadline, cancelled)?;
                Ok(value)
            }
        }
    })();
    Exchange {
        outcome,
        process: report,
        postflight,
        wall_elapsed: start.elapsed(),
        cpu_elapsed: None,
    }
}
fn classify(
    dataset: &Dataset,
    observed: &ProcessReport,
    cancelled: &AtomicBool,
) -> Result<Report, Failure> {
    if let Some(reason) = observed.interruption {
        return Err(Failure::Interrupted(reason));
    }
    if !observed.leader_reaped
        || !observed.process_group_settled
        || observed.pending.is_some()
        || !observed.stdout.eof
        || !observed.stderr.eof
        || observed.stdout.failed
        || observed.stderr.failed
        || observed.stdout.truncated
        || observed.stderr.truncated
    {
        return Err(Failure::Unsettled);
    }
    // Only the entrypoint's own refusal shape is typed: exit status 2, an empty
    // stderr and an error bound to this dataset's digest. Anything else is Nonzero.
    if observed.exit_code == Some(2)
        && observed.stderr.bytes.is_empty()
        && let Ok(code) = dataset.refusal(&observed.stdout.bytes)
    {
        return Err(Failure::Refused(code));
    }
    if observed.exit_code != Some(0) || observed.signal.is_some() {
        return Err(Failure::Nonzero);
    }
    if !observed.stderr.bytes.is_empty() {
        return Err(Failure::Diagnostic);
    }
    if cancelled.load(Ordering::Acquire) {
        return Err(Failure::Cancelled);
    }
    dataset
        .report(&observed.stdout.bytes, unix_ms()?)
        .map_err(Failure::Response)
}
fn window(start: Instant, deadline: Instant, cancelled: &AtomicBool) -> Result<(), Failure> {
    if cancelled.load(Ordering::Acquire) {
        return Err(Failure::Cancelled);
    }
    let now = Instant::now();
    if start > now
        || deadline <= now
        || deadline
            .checked_duration_since(start)
            .is_none_or(|d| d > Duration::from_secs(60))
    {
        return Err(Failure::Deadline);
    }
    Ok(())
}
fn unix_ms() -> Result<u64, Failure> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| Failure::Clock)?
        .as_millis()
        .try_into()
        .map_err(|_| Failure::Clock)
}
fn specification(profile: &JuliaProfile, dataset: &Dataset) -> ProcessSpec {
    let args = vec![
        OsString::from("--startup-file=no"),
        OsString::from(format!("--project={}", profile.project.display())),
        OsString::from("--check-bounds=yes"),
        OsString::from("--depwarn=error"),
        OsString::from("--threads=1"),
        OsString::from("--compiled-modules=no"),
        profile.project.join("bin/analysis.jl").into_os_string(),
    ];
    let environment = vec![
        ("LC_ALL", "C"),
        ("LANG", "C"),
        ("JULIA_LOAD_PATH", "@:@stdlib"),
        ("JULIA_PKG_OFFLINE", "true"),
        ("JULIA_PKG_PRECOMPILE_AUTO", "0"),
        ("JULIA_NUM_THREADS", "1"),
        ("OPENBLAS_NUM_THREADS", "1"),
    ];
    let mut environment: Vec<(OsString, OsString)> = environment
        .into_iter()
        .map(|(k, v)| (k.into(), v.into()))
        .collect();
    environment.push((
        "JULIA_DEPOT_PATH".into(),
        format!(
            "{}:{}",
            profile.scratch.display(),
            profile.dependency_depot.display()
        )
        .into(),
    ));
    environment.push(("HOME".into(), profile.scratch.clone().into_os_string()));
    environment.push(("TMPDIR".into(), profile.scratch.clone().into_os_string()));
    ProcessSpec {
        executable: profile.executable.clone(),
        arguments: args,
        directory: profile.scratch.clone(),
        environment,
        input: dataset.raw().to_vec(),
        stream_limit: MAX_REPORT,
    }
}
fn validate_profile(
    profile: &JuliaProfile,
    deadline: Instant,
    cancelled: &AtomicBool,
) -> Result<(), Failure> {
    for path in [
        &profile.project,
        &profile.scratch,
        &profile.dependency_depot,
    ] {
        if !path.is_absolute()
            || path.canonicalize().map_err(Failure::from)? != *path
            || !path.is_dir()
            || path.to_str().is_none_or(|s| s.contains(':'))
        {
            return Err(Failure::Profile);
        }
    }
    // Julia 1.12 selects these before the pinned conventional files, or loads
    // their unpinned preferences. Refuse aliases rather than silently executing
    // a different project/manifest configuration.
    for name in [
        "JuliaProject.toml",
        "JuliaManifest-v1.12.toml",
        "Manifest-v1.12.toml",
        "JuliaManifest.toml",
        "JuliaLocalPreferences.toml",
        "LocalPreferences.toml",
    ] {
        match fs::symlink_metadata(profile.project.join(name)) {
            Ok(_) => return Err(Failure::Profile),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(Failure::from(error)),
        }
    }
    if fs::metadata(&profile.scratch)
        .map_err(Failure::from)?
        .permissions()
        .mode()
        & 0o777
        != 0o700
        || profile.project_files.len() != FILES.len()
    {
        return Err(Failure::Profile);
    }
    pinned(
        &profile.executable,
        &profile.executable_sha256,
        256 * 1024 * 1024,
        deadline,
        cancelled,
    )?;
    for name in FILES {
        let expected = profile.project_files.get(name).ok_or(Failure::Profile)?;
        pinned(
            &profile.project.join(name),
            expected,
            1024 * 1024,
            deadline,
            cancelled,
        )?;
    }
    Ok(())
}
fn pinned(
    path: &Path,
    expected: &str,
    cap: u64,
    deadline: Instant,
    cancelled: &AtomicBool,
) -> Result<(), Failure> {
    if !path.is_absolute() || path.canonicalize().map_err(Failure::from)? != path {
        return Err(Failure::Profile);
    }
    let metadata = fs::symlink_metadata(path).map_err(Failure::from)?;
    if !metadata.is_file() || metadata.len() > cap {
        return Err(Failure::Profile);
    }
    let mut file = File::open(path).map_err(Failure::from)?;
    let mut hash = Sha256::new();
    let mut total = 0_u64;
    loop {
        if cancelled.load(Ordering::Acquire) {
            return Err(Failure::Cancelled);
        }
        if Instant::now() >= deadline {
            return Err(Failure::Deadline);
        }
        let mut chunk = [0_u8; 16_384];
        let n = file.read(&mut chunk).map_err(Failure::from)?;
        if n == 0 {
            break;
        }
        total += n as u64;
        if total > cap {
            return Err(Failure::Profile);
        }
        hash.update(&chunk[..n]);
    }
    let mut actual = String::from("sha256:");
    for b in hash.finalize() {
        actual.push(char::from(super::HEX[usize::from(b >> 4)]));
        actual.push(char::from(super::HEX[usize::from(b & 15)]));
    }
    if total != metadata.len() || actual != expected {
        return Err(Failure::Profile);
    }
    Ok(())
}
impl From<std::io::Error> for Failure {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error.raw_os_error())
    }
}

#[cfg(test)]
mod classify_order_tests {
    use super::{Dataset, Failure, Interruption, ProcessReport, classify};
    use crate::numerical::JuliaCode;
    use crate::worker::process::{SignalFacts, Stream};
    use std::sync::atomic::AtomicBool;
    use std::time::{Duration, Instant};

    type Checked = Result<(), Box<dyn std::error::Error>>;
    type Unsettle = fn(&mut ProcessReport);
    // Inside J01's own [cutoff, expires] window, so the fixture decodes unchanged.
    const NOW_MS: u64 = 1_769_999_995_000;

    fn stream(bytes: &[u8]) -> Stream {
        Stream {
            bytes: bytes.to_vec(),
            observed_bytes: u64::try_from(bytes.len()).unwrap_or(u64::MAX),
            eof: true,
            truncated: false,
            failed: false,
        }
    }
    /// A leader that wrote a refusal bound to `dataset` and exited 2, fully settled.
    fn refusing(dataset: &Dataset) -> ProcessReport {
        let body = format!(
            r#"{{"protocol":"hee3.analysis","version":1,"kind":"error","request_sha256":"{}","binding":null,"code":"stale","diagnostic":"analysis refusal"}}"#,
            dataset.digest()
        );
        ProcessReport {
            started_at: Instant::now(),
            leader_pid: 4242,
            exit_code: Some(2),
            signal: None,
            interruption: None,
            interruption_observed_at: None,
            stdout: stream(body.as_bytes()),
            stderr: stream(b""),
            elapsed: Duration::from_millis(7),
            signals: SignalFacts::default(),
            leader_reaped: true,
            process_group_settled: true,
            observer_ready: true,
            pending: None,
        }
    }

    /// NUM-01: a refusal is typed only after the interruption and settlement gates.
    /// The settled case is asserted first so each negative differs from a typed
    /// refusal in exactly one fact. The `pending` disjunct is not exercised: a
    /// `PendingChild` is constructible only by the worker owner.
    #[test]
    fn refusal_is_typed_only_after_interruption_and_settlement() -> Checked {
        let dataset = Dataset::decode(include_bytes!("../../tests/fixtures/t21/J01.json"), NOW_MS)
            .map_err(|invalid| format!("J01 fixture: {invalid:?}"))?;
        let never = AtomicBool::new(false);
        assert_eq!(
            classify(&dataset, &refusing(&dataset), &never).err(),
            Some(Failure::Refused(JuliaCode::Stale)),
            "settled refusal"
        );
        let unsettled: [(&str, Unsettle); 8] = [
            ("leader not reaped", |r| r.leader_reaped = false),
            ("group not settled", |r| r.process_group_settled = false),
            ("stdout not at eof", |r| r.stdout.eof = false),
            ("stderr not at eof", |r| r.stderr.eof = false),
            ("stdout failed", |r| r.stdout.failed = true),
            ("stderr failed", |r| r.stderr.failed = true),
            ("stdout truncated", |r| r.stdout.truncated = true),
            ("stderr truncated", |r| r.stderr.truncated = true),
        ];
        for (fact, unsettle) in unsettled {
            let mut observed = refusing(&dataset);
            unsettle(&mut observed);
            assert_eq!(
                classify(&dataset, &observed, &never).err(),
                Some(Failure::Unsettled),
                "{fact}"
            );
        }
        let mut observed = refusing(&dataset);
        observed.interruption = Some(Interruption::ResidualGroup);
        observed.process_group_settled = false;
        assert_eq!(
            classify(&dataset, &observed, &never).err(),
            Some(Failure::Interrupted(Interruption::ResidualGroup)),
            "interrupted and unsettled"
        );
        Ok(())
    }
}
