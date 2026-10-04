//! The deep-diff-forge adapter against the real binary (skipped, UNMEASURED, when absent) and a
//! stub that seals the wrong bytes.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::PathBuf;
use std::time::{Duration, Instant};

use hee4_contracts::{Outcome, Sha256Hex};
use hee4_contracts::{Reason, Verdict};
use hee4_evidence::ddf::{self, AdapterError};
use hee4_evidence::{Identities, Identity, Source, Subject, Why, decide};
use hee4_host::clock::TestClock;

const BUDGET: Duration = Duration::from_secs(60);
const FIXTURE: &[u8] = include_bytes!("fixtures/verdict.diff");

fn subject() -> Subject {
    Subject {
        task_id: "T-1".parse().unwrap(),
        head_sha: "3c1db9a2ebaedd3a0c153285fc0fc3e75bab163a".parse().unwrap(),
        input_sha256: Sha256Hex::digest(FIXTURE),
    }
}

fn present() -> bool {
    let ok = std::process::Command::new(ddf::BIN)
        .arg("--version")
        .output()
        .is_ok();
    if !ok {
        println!(
            "UNMEASURED: {} not on PATH; real-run test skipped",
            ddf::BIN
        );
    }
    ok
}

#[test]
fn real_run_on_fixture_is_a_sealed_tier0_observation() {
    if !present() {
        return;
    }
    let o = ddf::observe(FIXTURE, &subject(), &TestClock::new(0), BUDGET).unwrap();
    assert_eq!(
        o.input_sha256.to_string(),
        "be01a7e94940427a1e9c4ee7e21cedd88aab5b475b14039e66ab0eaad364a2ae"
    );
    assert_eq!(o.tool.name.as_str(), "deep-diff-forge");
    assert_eq!(o.outcome, Outcome::Pass);
    assert!(!o.advisory);
    assert_eq!(o.head_sha, subject().head_sha);
    assert_eq!(o.evidence.len(), 1);
    println!("MEASURED tool={} {}", o.tool.name, o.tool.version);
}

#[test]
fn exit_7_is_refused_with_the_stderr_line() {
    if !present() {
        return;
    }
    let header_only = b"diff --git a/x b/x\n";
    match ddf::observe(header_only, &subject(), &TestClock::new(0), BUDGET) {
        Err(AdapterError::Refused { stderr_line }) => {
            assert!(stderr_line.starts_with("refused: 0"), "{stderr_line}");
        }
        other => panic!("expected Refused, got {other:?}"),
    }
    match ddf::observe(b"", &subject(), &TestClock::new(0), BUDGET) {
        Err(AdapterError::Refused { stderr_line }) => {
            assert_eq!(stderr_line, "refused: 0 files in input (--require-files)");
        }
        other => panic!("expected Refused, got {other:?}"),
    }
}

fn stub(name: &str) -> PathBuf {
    // Committed with mode 755: a script written at test time races parallel forks (ETXTBSY).
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

#[test]
fn a_tool_that_sealed_other_bytes_is_a_seal_mismatch() {
    let bin = stub("ddf-wrong-seal.sh");
    match ddf::observe_with(&bin, FIXTURE, &subject(), &TestClock::new(0), BUDGET) {
        Err(AdapterError::SealMismatch { sent, sealed }) => {
            assert_eq!(sent, Sha256Hex::digest(FIXTURE));
            assert_eq!(sealed, Sha256Hex::digest(b"not the fixture"));
        }
        other => panic!("expected SealMismatch, got {other:?}"),
    }
}

#[test]
fn an_empty_ranking_looked_at_nothing() {
    let got = ddf::observe_with(
        &stub("ddf-empty.sh"),
        FIXTURE,
        &subject(),
        &TestClock::new(0),
        BUDGET,
    );
    assert!(matches!(got, Err(AdapterError::LookedAtNothing)), "{got:?}");
}

#[test]
fn a_tool_that_never_exits_is_killed_at_the_budget_and_refused_timeout() {
    let budget = Duration::from_millis(300);
    let t0 = Instant::now();
    let got = ddf::observe_with(
        &stub("ddf-sleep5.sh"),
        FIXTURE,
        &subject(),
        &TestClock::new(0),
        budget,
    );
    let elapsed = t0.elapsed();
    println!("MEASURED timeout elapsed={elapsed:?}");
    assert!(elapsed < Duration::from_secs(1), "{elapsed:?}");
    let Err(AdapterError::Timeout { budget: b }) = got else {
        panic!("expected Timeout, got {got:?}");
    };
    assert_eq!(b, budget);
    let o = ddf::timeout_observation(b, FIXTURE, &subject()).unwrap();
    assert_eq!(o.outcome, Outcome::Error);
    assert!(!o.advisory);
    assert!(o.elapsed_ms > o.budget_ms);
    let pin = |n: &str| {
        Identity::Wired(Source {
            name: n.parse().unwrap(),
            digest: Sha256Hex::digest(n.as_bytes()),
        })
    };
    let ids = Identities {
        collector: pin("c"),
        locks: pin("l"),
        standards: pin("s"),
    };
    let _ = Why::NotWired;
    assert_eq!(
        decide(&ids, &[o], &subject()).verdict,
        Verdict::Refused(Reason::Timeout)
    );
}
