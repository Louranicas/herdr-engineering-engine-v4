//! The deep-diff-forge adapter against the real binary (skipped, UNMEASURED, when absent), the
//! stubs under `fixtures/`, and the task shape `for_task` the dispatcher calls.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use hee4_contracts::{Outcome, Sha256Hex};
use hee4_contracts::{Reason, Verdict};
use hee4_evidence::ddf::{self, AdapterError, Diff, Skip, TaskObservation};
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
    assert_eq!(o.evidence.len(), 2);
    assert_eq!(labels(&o), ["rank.v0", "diff"]);
    assert_eq!(o.evidence[1].sha256, Sha256Hex::digest(FIXTURE));
    println!("MEASURED tool={} {}", o.tool.name, o.tool.version);
}

fn labels(o: &hee4_contracts::Observation) -> Vec<&str> {
    o.evidence.iter().map(|e| e.label.as_str()).collect()
}

fn all_wired() -> Identities {
    let pin = |n: &str| {
        Identity::Wired(Source {
            name: n.parse().unwrap(),
            digest: Sha256Hex::digest(n.as_bytes()),
        })
    };
    Identities {
        collector: pin("c"),
        locks: pin("l"),
        standards: pin("s"),
    }
}

#[test]
fn skip_names_are_the_wire_words() {
    assert_eq!(Skip::NoWorktree.name(), "no_worktree");
    assert_eq!(Skip::NoDiff.name(), "no_diff");
    assert_eq!(Skip::ToolAbsent.name(), "tool_absent");
    assert_eq!(format!("{}", Skip::ToolAbsent), "tool_absent");
    assert_eq!(format!("{}", Skip::NoWorktree), "no_worktree");
    assert_eq!(format!("{}", Skip::NoDiff), "no_diff");
}

#[test]
fn no_worktree_is_a_named_skip() {
    let budget = Duration::from_millis(300);
    let t0 = Instant::now();
    let got = ddf::for_task_with(
        &stub("ddf-sleep5.sh"),
        Diff::NoWorktree,
        &subject(),
        &TestClock::new(0),
        budget,
    );
    let elapsed = t0.elapsed();
    assert!(elapsed < Duration::from_millis(100), "spawned? {elapsed:?}");
    assert!(
        matches!(got, Ok(TaskObservation::Skipped(Skip::NoWorktree))),
        "{got:?}"
    );
}

#[test]
fn empty_diff_is_a_named_skip() {
    let budget = Duration::from_millis(300);
    let t0 = Instant::now();
    let got = ddf::for_task_with(
        &stub("ddf-sleep5.sh"),
        Diff::Bytes(b""),
        &subject(),
        &TestClock::new(0),
        budget,
    );
    let elapsed = t0.elapsed();
    assert!(elapsed < Duration::from_millis(100), "spawned? {elapsed:?}");
    assert!(
        matches!(got, Ok(TaskObservation::Skipped(Skip::NoDiff))),
        "{got:?}"
    );
}

#[test]
fn absent_tool_is_a_named_skip() {
    let got = ddf::for_task_with(
        Path::new("/nonexistent/deep-diff-forge"),
        Diff::Bytes(FIXTURE),
        &subject(),
        &TestClock::new(0),
        BUDGET,
    );
    assert!(
        matches!(got, Ok(TaskObservation::Skipped(Skip::ToolAbsent))),
        "{got:?}"
    );
}

/// A VERIFY-digest subject: the diff digest and the subject's input differ, as in the
/// dispatcher (`Subject.input_sha256 = digest(brief VERIFY)`, V4-94).
fn verify_subject() -> Subject {
    let mut s = subject();
    s.input_sha256 = Sha256Hex::digest(b"VERIFY: /usr/bin/test -d /usr");
    s
}

fn assert_task_pass(got: Result<TaskObservation, AdapterError>) {
    let subject = verify_subject();
    let Ok(TaskObservation::Observed(o)) = got else {
        panic!("expected Observed, got {got:?}");
    };
    // The verdict first: an observation bound to the diff digest is `Refused(Unreconciled)`.
    let verdict = decide(&all_wired(), std::slice::from_ref(&o), &subject).verdict;
    assert_eq!(verdict, Verdict::Pass);
    assert_eq!(o.input_sha256, subject.input_sha256);
    assert_ne!(o.input_sha256, Sha256Hex::digest(FIXTURE));
    assert_eq!(o.outcome, Outcome::Pass);
    assert!(!o.advisory);
    assert_eq!(labels(&o), ["rank.v0", "diff"]);
    assert_eq!(o.evidence[1].sha256, Sha256Hex::digest(FIXTURE));
}

#[test]
fn a_task_pass_reconciles_against_the_verify_digest() {
    // The deterministic stub always runs; the real binary too when present.
    assert_task_pass(ddf::for_task_with(
        &stub("ddf-pass.sh"),
        Diff::Bytes(FIXTURE),
        &verify_subject(),
        &TestClock::new(0),
        BUDGET,
    ));
    if !present() {
        return;
    }
    let got = ddf::for_task(
        Diff::Bytes(FIXTURE),
        &verify_subject(),
        &TestClock::new(0),
        BUDGET,
    );
    if let Ok(TaskObservation::Observed(o)) = &got {
        println!("MEASURED for_task tool={} {}", o.tool.name, o.tool.version);
    }
    assert_task_pass(got);
}

#[test]
fn a_timed_out_task_run_is_an_observed_timeout() {
    let budget = Duration::from_millis(300);
    let t0 = Instant::now();
    let got = ddf::for_task_with(
        &stub("ddf-sleep5.sh"),
        Diff::Bytes(FIXTURE),
        &subject(),
        &TestClock::new(0),
        budget,
    );
    let elapsed = t0.elapsed();
    println!("MEASURED for_task timeout elapsed={elapsed:?}");
    assert!(elapsed < Duration::from_secs(1), "{elapsed:?}");
    let Ok(TaskObservation::Observed(o)) = got else {
        panic!("expected Observed timeout, got {got:?}");
    };
    assert_eq!(o.outcome, Outcome::Error);
    assert!(!o.advisory);
    assert!(o.elapsed_ms > o.budget_ms);
    assert_eq!(
        decide(&all_wired(), &[o], &subject()).verdict,
        Verdict::Refused(Reason::Timeout)
    );
}

#[test]
fn a_wrong_seal_is_an_error_not_a_skip() {
    let got = ddf::for_task_with(
        &stub("ddf-wrong-seal.sh"),
        Diff::Bytes(FIXTURE),
        &subject(),
        &TestClock::new(0),
        BUDGET,
    );
    assert!(
        matches!(got, Err(AdapterError::SealMismatch { .. })),
        "{got:?}"
    );
}

#[test]
fn an_empty_ranking_is_an_error_not_a_skip() {
    let got = ddf::for_task_with(
        &stub("ddf-empty.sh"),
        Diff::Bytes(FIXTURE),
        &subject(),
        &TestClock::new(0),
        BUDGET,
    );
    assert!(matches!(got, Err(AdapterError::LookedAtNothing)), "{got:?}");
}

#[test]
fn exit_7_through_the_task_shape_is_observed_refused() {
    let got = ddf::for_task_with(
        &stub("ddf-exit7-silent.sh"),
        Diff::Bytes(FIXTURE),
        &subject(),
        &TestClock::new(0),
        BUDGET,
    );
    let Ok(TaskObservation::Observed(o)) = got else {
        panic!("expected Observed, got {got:?}");
    };
    let Outcome::Refused { reason } = &o.outcome else {
        panic!("expected Refused, got {:?}", o.outcome);
    };
    assert_eq!(
        reason.as_str(),
        "deep-diff-forge exited 7 without a message"
    );
    assert!(!o.advisory);
}

#[test]
fn exit_7_is_a_tier0_refused_observation_that_decides_invalid() {
    if !present() {
        return;
    }
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
    for (diff, reason) in [
        (b"diff --git a/x b/x\n".as_slice(), None),
        (
            b"".as_slice(),
            Some("refused: 0 files in input (--require-files)"),
        ),
    ] {
        let mut subject = subject();
        subject.input_sha256 = Sha256Hex::digest(diff);
        let o = ddf::observe(diff, &subject, &TestClock::new(0), BUDGET).unwrap();
        assert!(!o.advisory);
        let Outcome::Refused { reason: got } = &o.outcome else {
            panic!("expected Refused, got {:?}", o.outcome);
        };
        match reason {
            Some(want) => assert_eq!(got.as_str(), want),
            None => assert!(got.as_str().starts_with("refused: 0"), "{got}"),
        }
        assert_eq!(
            decide(&ids, &[o], &subject).verdict,
            Verdict::Refused(Reason::Invalid)
        );
    }
}

fn stub(name: &str) -> PathBuf {
    // Committed with mode 755: a script written at test time races parallel forks (ETXTBSY).
    // Read at run time: a binary reused across `git archive` exports must not walk a baked path.
    let manifest = std::env::var("CARGO_MANIFEST_DIR")
        .expect("CARGO_MANIFEST_DIR is not set (run under cargo test)");
    PathBuf::from(manifest).join("tests/fixtures").join(name)
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

#[test]
fn a_silent_exit_7_still_carries_a_reason() {
    let o = ddf::observe_with(
        &stub("ddf-exit7-silent.sh"),
        FIXTURE,
        &subject(),
        &TestClock::new(0),
        BUDGET,
    )
    .unwrap();
    let Outcome::Refused { reason } = &o.outcome else {
        panic!("expected Refused, got {:?}", o.outcome);
    };
    assert_eq!(
        reason.as_str(),
        "deep-diff-forge exited 7 without a message"
    );
    assert!(!o.advisory);
}
