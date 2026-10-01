//! Actual local Julia IPC controls. TH-DEV, not namespace/admission qualification.
use habitat_engine::numerical::{
    Dataset, JuliaCode,
    process::{self, Failure, JuliaProfile},
};
use habitat_engine::worker::process::Interruption;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
static NEXT: AtomicU64 = AtomicU64::new(0);
const JULIA: &str = "/var/home/Louranicas/.julia/juliaup/julia-1.12.7+0.x64.linux.gnu/bin/julia";
const DEPOT: &str = "/var/home/Louranicas/.julia";
fn hash(path: &Path) -> String {
    let mut s = String::from("sha256:");
    for b in Sha256::digest(fs::read(path).unwrap()) {
        let digits = b"0123456789abcdef";
        s.push(char::from(digits[usize::from(b >> 4)]));
        s.push(char::from(digits[usize::from(b & 15)]));
    }
    s
}
fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis()
        .try_into()
        .unwrap()
}
fn dataset() -> Dataset {
    let mut q: serde_json::Value =
        serde_json::from_slice(include_bytes!("fixtures/t21/J01.json")).unwrap();
    q["cutoff_unix_ms"] = (now() - 1000).to_string().into();
    q["expires_unix_ms"] = (now() + 120_000).to_string().into();
    Dataset::decode(&serde_json::to_vec(&q).unwrap(), now()).unwrap()
}
struct Area(PathBuf);
impl Area {
    fn new() -> Self {
        let p = std::env::temp_dir().join(format!(
            "t21p-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&p).unwrap();
        fs::set_permissions(&p, fs::Permissions::from_mode(0o700)).unwrap();
        Self(p.canonicalize().unwrap())
    }
    fn profile(&self, script: Option<&str>) -> JuliaProfile {
        let original = Path::new(env!("CARGO_MANIFEST_DIR")).join("julia");
        let project = if let Some(text) = script {
            let p = self.0.join("project");
            fs::create_dir(&p).unwrap();
            fs::create_dir(p.join("src")).unwrap();
            fs::create_dir(p.join("bin")).unwrap();
            for n in [
                "Project.toml",
                "Manifest.toml",
                "src/HabitatAnalysis.jl",
                "src/Evaluate.jl",
                "src/Cohesion.jl",
            ] {
                fs::copy(original.join(n), p.join(n)).unwrap();
            }
            fs::write(p.join("bin/analysis.jl"), text).unwrap();
            p
        } else {
            original
        };
        let scratch = self.0.join("scratch");
        fs::create_dir(&scratch).unwrap();
        fs::set_permissions(&scratch, fs::Permissions::from_mode(0o700)).unwrap();
        let project_files = [
            "Project.toml",
            "Manifest.toml",
            "src/HabitatAnalysis.jl",
            "src/Evaluate.jl",
            "src/Cohesion.jl",
            "bin/analysis.jl",
        ]
        .into_iter()
        .map(|n| (n.into(), hash(&project.join(n))))
        .collect::<BTreeMap<_, _>>();
        JuliaProfile {
            executable: JULIA.into(),
            executable_sha256: hash(Path::new(JULIA)),
            project,
            project_files,
            scratch,
            dependency_depot: DEPOT.into(),
        }
    }
}
impl Drop for Area {
    fn drop(&mut self) {
        if !std::thread::panicking() {
            fs::remove_dir_all(&self.0).unwrap();
        }
    }
}
fn run(profile: &JuliaProfile, data: &Dataset) -> process::Exchange {
    let start = Instant::now();
    exchange(
        profile,
        data,
        start,
        start + Duration::from_secs(60),
        &AtomicBool::new(false),
    )
}
fn exchange(
    profile: &JuliaProfile,
    data: &Dataset,
    start: Instant,
    deadline: Instant,
    cancel: &AtomicBool,
) -> process::Exchange {
    let result = process::analyze(profile, data, start, deadline, cancel);
    retain(&result, profile, data);
    result
}
fn retain(result: &process::Exchange, profile: &JuliaProfile, data: &Dataset) {
    let root =
        PathBuf::from(std::env::var_os("T21_PROCESS_EVIDENCE").expect("required evidence root"));
    let label = std::thread::current()
        .name()
        .unwrap_or("unnamed")
        .replace(':', "_");
    let out = root.join(format!(
        "{}-{}",
        label,
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&out).unwrap();
    fs::write(out.join("request.json"), data.raw()).unwrap();
    let observed=result.process.as_ref().map(|p| {
        fs::write(out.join("stdout"),&p.stdout.bytes).unwrap();fs::write(out.join("stderr"),&p.stderr.bytes).unwrap();
        serde_json::json!({"leader_pid":p.leader_pid,"exit_code":p.exit_code,"signal":p.signal,
            "interruption":format!("{:?}",p.interruption),"elapsed_ns":p.elapsed.as_nanos().to_string(),
            "leader_reaped":p.leader_reaped,"group_settled":p.process_group_settled,"pending":p.pending.is_some(),
            "stdout_eof":p.stdout.eof,"stderr_eof":p.stderr.eof,"stdout_observed":p.stdout.observed_bytes,
            "stderr_observed":p.stderr.observed_bytes,"stdout_truncated":p.stdout.truncated,"stderr_truncated":p.stderr.truncated,
            "signals":format!("{:?}",p.signals)})
    });
    for name in profile.project_files.keys() {
        let dest = out.join("project-after").join(name);
        fs::create_dir_all(dest.parent().unwrap()).unwrap();
        fs::copy(profile.project.join(name), dest).unwrap();
    }
    let value = serde_json::json!({"request_sha256":data.digest(),"outcome":format!("{:?}",result.outcome),
        "postflight":format!("{:?}",result.postflight),"wall_ns":result.wall_elapsed.as_nanos().to_string(),
        "cpu_ns":result.cpu_elapsed.map(|d|d.as_nanos().to_string()),"process":observed,
        "executable":profile.executable,"executable_sha256":profile.executable_sha256,
        "project":profile.project,"project_pins_before":profile.project_files});
    fs::write(
        out.join("observation.json"),
        serde_json::to_vec_pretty(&value).unwrap(),
    )
    .unwrap();
}

fn settled(result: &process::Exchange) {
    let p = result.process.as_ref().unwrap();
    assert!(p.leader_reaped && p.process_group_settled && p.pending.is_none());
    assert!(p.stdout.eof && p.stderr.eof);
    assert!(result.cpu_elapsed.is_none());
}
#[test]
fn real_julia_returns_exact_independently_expected_dataset_report() {
    let a = Area::new();
    let p = a.profile(None);
    let d = dataset();
    let r = run(&p, &d);
    settled(&r);
    let report = r.outcome.unwrap();
    assert_eq!(report.counts.total, "5");
    assert_eq!(report.counts.known_usage_sum, "4");
    assert_eq!(report.mean_observed_ms.to_bits(), 30.0_f64.to_bits());
    assert_eq!(report.acceptance_fraction.to_bits(), 0.2_f64.to_bits());
    assert_eq!(report.request_sha256, d.digest());
}
#[test]
fn altered_project_pin_refuses_before_launch() {
    let a = Area::new();
    let mut p = a.profile(None);
    p.project_files
        .insert("Project.toml".into(), "sha256:wrong".into());
    let r = run(&p, &dataset());
    assert!(matches!(r.outcome, Err(Failure::Profile)));
    assert!(r.process.is_none());
}
#[test]
fn cancellation_before_launch_retains_no_fabricated_child() {
    let a = Area::new();
    let p = a.profile(None);
    let start = Instant::now();
    let r = exchange(
        &p,
        &dataset(),
        start,
        start + Duration::from_secs(60),
        &AtomicBool::new(true),
    );
    assert!(matches!(r.outcome, Err(Failure::Cancelled)));
    assert!(r.process.is_none());
}
#[test]
fn malformed_empty_truncated_multiple_and_nonfinite_responses_refuse() {
    for text in ["", "{", "{}{}", "{\"mean_observed_ms\":NaN}"] {
        let a = Area::new();
        let code = format!(
            "read(stdin); write(stdout, {})",
            serde_json::to_string(text).unwrap()
        );
        let p = a.profile(Some(&code));
        let r = run(&p, &dataset());
        settled(&r);
        assert!(
            matches!(r.outcome, Err(Failure::Response(_))),
            "{:?}",
            r.outcome
        );
    }
}
#[test]
fn nonzero_with_plausible_report_cannot_succeed() {
    let a = Area::new();
    let p=a.profile(Some("using HabitatAnalysis; raw=read(stdin); write(stdout, analyze(raw, UInt64(floor(time()*1000)))); exit(7)"));
    let r = run(&p, &dataset());
    settled(&r);
    assert!(matches!(r.outcome, Err(Failure::Nonzero)));
    assert_eq!(r.process.unwrap().exit_code, Some(7));
}
#[test]
fn stderr_diagnostic_prevents_success() {
    let a = Area::new();
    let p=a.profile(Some("using HabitatAnalysis; raw=read(stdin); write(stdout, analyze(raw, UInt64(floor(time()*1000)))); write(stderr,\"warning\")"));
    let r = run(&p, &dataset());
    settled(&r);
    assert!(matches!(r.outcome, Err(Failure::Diagnostic)));
}
#[test]
fn output_limit_retains_bounded_prefix_and_real_cleanup() {
    let a = Area::new();
    let p = a.profile(Some("read(stdin); write(stdout, repeat(\"x\", 100000))"));
    let r = run(&p, &dataset());
    settled(&r);
    assert!(matches!(
        r.outcome,
        Err(Failure::Interrupted(Interruption::OutputLimit))
    ));
    assert!(r.process.unwrap().stdout.bytes.len() <= 65536);
}
#[test]
fn timeout_uses_original_work_deadline_and_cleanup_reserve() {
    let a = Area::new();
    let p = a.profile(Some("read(stdin); sleep(30)"));
    let start = Instant::now();
    let r = exchange(
        &p,
        &dataset(),
        start,
        start + Duration::from_secs(11),
        &AtomicBool::new(false),
    );
    settled(&r);
    assert!(matches!(
        r.outcome,
        Err(Failure::Interrupted(Interruption::Timeout))
    ));
    assert!(r.wall_elapsed < Duration::from_secs(11));
}
#[test]
fn cancellation_during_real_child_keeps_original_custody() {
    let a = Area::new();
    let p = a.profile(Some("read(stdin); sleep(30)"));
    let cancel = AtomicBool::new(false);
    let start = Instant::now();
    let r = std::thread::scope(|s| {
        s.spawn(|| {
            std::thread::sleep(Duration::from_secs(1));
            cancel.store(true, Ordering::Release);
        });
        exchange(
            &p,
            &dataset(),
            start,
            start + Duration::from_secs(60),
            &cancel,
        )
    });
    settled(&r);
    assert!(matches!(
        r.outcome,
        Err(Failure::Interrupted(Interruption::Cancelled))
    ));
}
#[test]
fn descendant_in_owned_group_prevents_false_clean_success() {
    if std::env::var_os("T21_DESCENDANT_INNER").is_none() {
        joined_descendant_control();
        return;
    }
    let a = Area::new();
    let p = a.profile(Some("read(stdin); run(`/usr/bin/sleep 30`; wait=false)"));
    let r = run(&p, &dataset());
    settled(&r);
    assert!(matches!(
        r.outcome,
        Err(Failure::Interrupted(Interruption::ResidualGroup))
    ));
    // NUM-01 ordering: a bound refusal under exit 2 is typed only once the group
    // settled. The real entrypoint writes its refusal, leaves a descendant, exits 2.
    let a = Area::new();
    let lingering = "run(`/usr/bin/sleep 30`; wait=false); exit(2)";
    let p = a.profile(Some(
        &entrypoint(&[(RECEIVED, FUTURE), ("exit(2)", lingering)]).unwrap(),
    ));
    let d = dataset();
    let r = run(&p, &d);
    settled(&r);
    let observed = r.process.as_ref().unwrap();
    assert_eq!(observed.exit_code, Some(2), "leader exit");
    assert!(observed.stderr.bytes.is_empty(), "stderr");
    assert_eq!(
        d.refusal(&observed.stdout.bytes),
        Ok(JuliaCode::Stale),
        "stdout holds a refusal bound to this request"
    );
    assert_eq!(
        r.outcome.as_ref().err(),
        Some(&Failure::Interrupted(Interruption::ResidualGroup)),
        "refusal with a residual group"
    );
}

// The original negative control deliberately creates an orphan. Reuse the
// existing development owner in a joined test-only scope to reap that custody;
// retain its descendants=true observation instead of leaking it to the suite.
fn joined_descendant_control() {
    use habitat_engine::worker::process::{ProcessSpec, run as run_owned};
    let area = Area::new();
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let binary = std::env::current_exe().unwrap();
    let evidence = PathBuf::from(std::env::var_os("T21_PROCESS_EVIDENCE").unwrap());
    let output = evidence.join("descendant-control-owner");
    let spec = ProcessSpec {
        executable: "/usr/bin/python3".into(),
        arguments: vec![
            "-B".into(),
            root.join("tests/fixtures/t21/descendant-control.py")
                .into_os_string(),
            binary.clone().into_os_string(),
            hash(&binary).into(),
            output.clone().into_os_string(),
        ],
        directory: root.into(),
        environment: vec![
            ("LC_ALL".into(), "C".into()),
            ("TMPDIR".into(), area.0.clone().into_os_string()),
            ("T21_PROCESS_EVIDENCE".into(), evidence.into_os_string()),
            (
                "T21_JULIA_REPORT".into(),
                std::env::var_os("T21_JULIA_REPORT").unwrap(),
            ),
        ],
        input: Vec::new(),
        stream_limit: 2 * 1024 * 1024,
    };
    let observed = run_owned(
        &spec,
        Instant::now() + Duration::from_secs(90),
        &AtomicBool::new(false),
    )
    .unwrap();
    fs::create_dir_all(&output).unwrap();
    fs::write(
        output.join("outer-owner.txt"),
        format!("spec={spec:#?}\nresult={observed:#?}"),
    )
    .unwrap();
    assert!(observed.leader_reaped && observed.process_group_settled && observed.pending.is_none());
    assert!(
        observed.stdout.eof
            && observed.stderr.eof
            && !observed.stdout.failed
            && !observed.stderr.failed
            && !observed.stdout.truncated
            && !observed.stderr.truncated
    );
    assert!(observed.interruption.is_none() && observed.signal.is_none());
    assert_eq!(observed.exit_code, Some(0));
    assert!(observed.stderr.bytes.is_empty());
    let receipt: serde_json::Value = serde_json::from_slice(&observed.stdout.bytes).unwrap();
    assert_eq!(
        receipt["scope"],
        "T21 residual-descendant control joined by existing owner"
    );
    assert_eq!(receipt["accepted_control"], true);
}

#[test]
fn real_julia_handles_all_4096_rows_without_truncating_input() {
    let area = Area::new();
    let p = area.profile(None);
    let base = dataset();
    let mut q: serde_json::Value = serde_json::from_slice(base.raw()).unwrap();
    let mut rows = Vec::new();
    for i in 0..4096 {
        let mut r = q["observations"][0].clone();
        r["attempt_id"] = format!("30000000-0000-4000-8000-{i:012x}").into();
        rows.push(r);
    }
    q["observations"] = rows.into();
    q["shape"]["rows"] = 4096.into();
    let d = Dataset::decode(&serde_json::to_vec(&q).unwrap(), now()).unwrap();
    let r = run(&p, &d);
    settled(&r);
    let report = r.outcome.unwrap();
    assert_eq!(report.counts.total, "4096");
    assert_eq!(report.counts.accepted, "4096");
    assert_eq!(report.counts.known_usage_sum, "16384");
    assert_eq!(report.mean_observed_ms.to_bits(), 10.0_f64.to_bits());
}
#[test]
fn changed_request_digest_in_actual_child_response_is_refused() {
    let a = Area::new();
    let p=a.profile(Some("using HabitatAnalysis; raw=read(stdin); r=String(analyze(raw,UInt64(floor(time()*1000)))); write(stdout, replace(r, r\"sha256:[0-9a-f]{64}\" => \"sha256:ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff\"))"));
    let r = run(&p, &dataset());
    settled(&r);
    assert!(matches!(r.outcome, Err(Failure::Response(_))));
}
#[test]
fn short_expired_future_and_overlong_original_windows_refuse_before_launch() {
    let a = Area::new();
    let p = a.profile(None);
    let d = dataset();
    let now = Instant::now();
    for (start, end) in [
        (now, now),
        (now, now + Duration::from_secs(10)),
        (now, now + Duration::from_secs(61)),
        (now + Duration::from_secs(1), now + Duration::from_secs(50)),
    ] {
        let r = exchange(&p, &d, start, end, &AtomicBool::new(false));
        assert!(matches!(r.outcome, Err(Failure::Deadline)));
        assert!(r.process.is_none());
    }
}
#[test]
fn changed_file_after_dispatch_refuses_success_and_retains_raw_report() {
    let a = Area::new();
    let original = Path::new(env!("CARGO_MANIFEST_DIR")).join("julia");
    let script = fs::read_to_string(original.join("bin/analysis.jl")).unwrap();
    let p = a.profile(Some(&("sleep(2);\n".to_owned() + &script)));
    let start = Instant::now();
    let d = dataset();
    let r = std::thread::scope(|scope| {
        scope.spawn(|| {
            std::thread::sleep(Duration::from_secs(1));
            fs::write(
                p.project.join("src/Cohesion.jl"),
                "# changed in owned test profile",
            )
            .unwrap();
        });
        exchange(
            &p,
            &d,
            start,
            start + Duration::from_secs(60),
            &AtomicBool::new(false),
        )
    });
    settled(&r);
    assert!(matches!(r.outcome, Err(Failure::Profile)));
    assert_eq!(r.postflight, Some(Failure::Profile));
    assert!(!r.process.unwrap().stdout.bytes.is_empty());
}

#[test]
fn fixed_julia_entrypoint_returns_bounded_stable_error_and_available_binding() {
    use habitat_engine::worker::process::{self as owner, ProcessSpec};
    let area = Area::new();
    let profile = area.profile(None);
    let mut stale: serde_json::Value = serde_json::from_slice(dataset().raw()).unwrap();
    stale["expires_unix_ms"] = (now() - 1).to_string().into();
    let evidence = PathBuf::from(std::env::var_os("T21_PROCESS_EVIDENCE").unwrap());
    for (name, raw, expected_code) in [
        ("encoding", b"{".to_vec(), "encoding"),
        ("stale", serde_json::to_vec(&stale).unwrap(), "stale"),
    ] {
        let out = evidence.join(format!("direct-error-{name}"));
        fs::create_dir_all(&out).unwrap();
        fs::write(out.join("request.json"), &raw).unwrap();
        let spec = ProcessSpec {
            executable: profile.executable.clone(),
            arguments: vec![
                "--startup-file=no".into(),
                format!("--project={}", profile.project.display()).into(),
                "--check-bounds=yes".into(),
                "--depwarn=error".into(),
                "--threads=1".into(),
                "--compiled-modules=no".into(),
                profile.project.join("bin/analysis.jl").into_os_string(),
            ],
            directory: profile.scratch.clone(),
            environment: vec![
                (
                    "JULIA_DEPOT_PATH".into(),
                    format!(
                        "{}:{}",
                        profile.scratch.display(),
                        profile.dependency_depot.display()
                    )
                    .into(),
                ),
                ("JULIA_LOAD_PATH".into(), "@:@stdlib".into()),
                ("JULIA_PKG_OFFLINE".into(), "true".into()),
                ("JULIA_PKG_PRECOMPILE_AUTO".into(), "0".into()),
                ("OPENBLAS_NUM_THREADS".into(), "1".into()),
            ],
            input: raw,
            stream_limit: 65536,
        };
        let result = owner::run(
            &spec,
            Instant::now() + Duration::from_secs(40),
            &AtomicBool::new(false),
        )
        .unwrap();
        fs::write(out.join("stdout"), &result.stdout.bytes).unwrap();
        fs::write(out.join("stderr"), &result.stderr.bytes).unwrap();
        fs::write(
            out.join("observation.txt"),
            format!("spec={spec:?}\nreport={result:?}"),
        )
        .unwrap();
        assert_eq!(result.exit_code, Some(2));
        assert!(result.leader_reaped && result.process_group_settled && result.pending.is_none());
        assert!(result.stdout.eof && result.stderr.eof && result.stderr.bytes.is_empty());
        let error: serde_json::Value = serde_json::from_slice(&result.stdout.bytes).unwrap();
        assert_eq!(error["kind"], "error");
        assert_eq!(error["code"], expected_code);
        assert_eq!(error["request_sha256"], hash(&out.join("request.json")));
        if name == "stale" {
            assert_eq!(error["binding"]["subject"], stale["subject"]);
            assert_eq!(error["binding"]["request_id"], stale["request_id"]);
        } else {
            assert!(error["binding"].is_null());
        }
    }
}

#[test]
fn unpinned_julia_project_manifest_and_preference_overrides_refuse() {
    let area = Area::new();
    let script =
        fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("julia/bin/analysis.jl"))
            .unwrap();
    let profile = area.profile(Some(&script));
    for name in [
        "JuliaProject.toml",
        "JuliaManifest-v1.12.toml",
        "Manifest-v1.12.toml",
        "JuliaManifest.toml",
        "JuliaLocalPreferences.toml",
        "LocalPreferences.toml",
    ] {
        let path = profile.project.join(name);
        fs::write(&path, "# unpinned fixture override").unwrap();
        let result = run(&profile, &dataset());
        assert!(matches!(result.outcome, Err(Failure::Profile)));
        assert!(result.process.is_none());
        fs::remove_file(path).unwrap();
    }
    fs::write(
        profile.project.join("README.md"),
        "Nonexecuted documentation neighbor",
    )
    .unwrap();
    let result = run(&profile, &dataset());
    settled(&result);
    assert!(result.outcome.is_ok());
}

// NUM-01: the fixed entrypoint's own error path, reached through process::analyze.
// Rust refuses stale and malformed input before launch, so each case edits exactly
// one expression of the real bin/analysis.jl and asserts that edit happened.
type Checked = Result<(), Box<dyn std::error::Error>>;
const RECEIVED: &str = "UInt64(floor(time() * 1000))";
const FUTURE: &str = "UInt64(floor(time() * 1000)) + UInt64(86_400_000)";
fn entrypoint(edits: &[(&str, &str)]) -> Result<String, Box<dyn std::error::Error>> {
    let mut script =
        fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("julia/bin/analysis.jl"))?;
    for (from, to) in edits {
        assert_eq!(script.matches(from).count(), 1, "edit anchor {from:?}");
        script = script.replace(from, to);
    }
    Ok(script)
}
fn refused(
    edits: &[(&str, &str)],
) -> Result<(process::Exchange, Dataset), Box<dyn std::error::Error>> {
    let a = Area::new();
    let p = a.profile(Some(&entrypoint(edits)?));
    let d = dataset();
    let r = run(&p, &d);
    settled(&r);
    Ok((r, d))
}
#[test]
fn real_julia_stale_refusal_is_typed_with_its_bound_digest() -> Checked {
    let (r, d) = refused(&[(RECEIVED, FUTURE)])?;
    assert_eq!(
        r.outcome.as_ref().err(),
        Some(&Failure::Refused(JuliaCode::Stale))
    );
    let observed = r.process.as_ref().ok_or("process")?;
    assert_eq!(observed.exit_code, Some(2));
    let error: serde_json::Value = serde_json::from_slice(&observed.stdout.bytes)?;
    assert_eq!(error["request_sha256"], d.digest());
    assert_eq!(error["binding"]["request_id"], d.request().request_id);
    Ok(())
}
#[test]
fn real_julia_encoding_refusal_is_typed() -> Checked {
    let (r, _) = refused(&[("analyze(raw,", "analyze(UInt8[0x7b],")])?;
    assert_eq!(
        r.outcome.as_ref().err(),
        Some(&Failure::Refused(JuliaCode::Encoding))
    );
    assert_eq!(r.process.as_ref().ok_or("process")?.exit_code, Some(2));
    Ok(())
}
#[test]
fn refusal_bound_to_another_request_digest_stays_nonzero() -> Checked {
    let (r, _) = refused(&[
        (RECEIVED, FUTURE),
        (
            "bytes2hex(sha256(raw))",
            "bytes2hex(sha256(vcat(raw, UInt8[0x20])))",
        ),
    ])?;
    assert_eq!(r.outcome.as_ref().err(), Some(&Failure::Nonzero));
    assert_eq!(r.process.as_ref().ok_or("process")?.exit_code, Some(2));
    Ok(())
}
#[test]
fn exit_two_with_plausible_report_stays_nonzero() -> Checked {
    let a = Area::new();
    let p = a.profile(Some(&format!(
        "using HabitatAnalysis; raw=read(stdin); write(stdout, analyze(raw, {RECEIVED})); exit(2)"
    )));
    let r = run(&p, &dataset());
    settled(&r);
    assert_eq!(r.outcome.as_ref().err(), Some(&Failure::Nonzero));
    assert_eq!(r.process.as_ref().ok_or("process")?.exit_code, Some(2));
    Ok(())
}
#[test]
fn refusal_under_another_exit_status_stays_nonzero() -> Checked {
    let (r, _) = refused(&[(RECEIVED, FUTURE), ("exit(2)", "exit(3)")])?;
    assert_eq!(r.outcome.as_ref().err(), Some(&Failure::Nonzero));
    assert_eq!(r.process.as_ref().ok_or("process")?.exit_code, Some(3));
    Ok(())
}
#[test]
fn refusal_with_stderr_diagnostic_stays_nonzero() -> Checked {
    let (r, _) = refused(&[
        (RECEIVED, FUTURE),
        ("exit(2)", "write(stderr, \"diagnostic\"); exit(2)"),
    ])?;
    assert_eq!(r.outcome.as_ref().err(), Some(&Failure::Nonzero));
    let observed = r.process.as_ref().ok_or("process")?;
    assert_eq!(observed.exit_code, Some(2));
    assert!(!observed.stderr.bytes.is_empty());
    Ok(())
}
