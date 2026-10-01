//! Independent T06 task-driver design, frozen before test bodies.
//!
//! Authority: original T06 acceptance; RC01 revision4; existing published T01
//! `LoopGuard` public contract and parent-supplied driver API. New task/driver.rs
//! implementation is not read. The recording fake owns opaque attempts/evidence,
//! elapsed clock snapshots, cancellation answers and method results. It proves
//! control flow only, not actual worker custody, cost measurement or persistence.
//!
//! Frozen cases: invalid criterion cardinality before all runtime calls; ordered
//! begin/execute/verify/accept with exact evidence; worker failure and uncertainty;
//! failed-check repair; two zero-progress attempts; new progress allowing attempt3;
//! three-attempt ceiling despite progress; historic-union and partial/extra-pass
//! refusal; undeclared failed bits; invalid/error/timeout/unsettled checker taxonomy;
//! cancellation at all three declared boundaries;900s dispatch and1200s total
//! boundaries; reserved verification after900s; late pass; immutable origin across
//! repair; regressed clock; each runtime error port including cancellation and stop;
//! truthful `NeedsSettlement` when stop cannot settle; all64 declared criteria.
//!
//! Literal policy: <=3 attempts;2 consecutive attempts without newly satisfied
//! criteria; no new begin at elapsed>=900000ms; no acceptance at>=1200000ms.
//! Criteria count0/65 gives `Error::Policy(InvalidCriteria)` without runtime calls.
//! Passed requires exactly all current declared bits, otherwise stop `InvalidCheck`.
//! Failed undeclared bits gives `Error::Policy(InvalidCriteria)`. Guard refusals call
//! stop(Policy(reason)); false settlement gives `NeedsSettlement`, never terminality.
//! Runtime errors propagate without fabricated accept/stop. The fake verify call
//! models the documented persistence port contract; no row or cost is claimed here.
//!
//! Amendment B14a-1b (2026-09-26, recorded in B14-store-runtime DESIGN.md B14a-R1.3): `accept`
//! answers `Acceptance`; the store names a cancellation that won the race `CancelledFirst`, which
//! the driver stops as `Cancelled` (settled → `Stopped`, unsettled → `NeedsSettlement`) with no
//! later begin. The Fake's accept answer is scripted and its default is `Accepted`.
//!
//! Parent owns compilation/execution. This file adds no production implementation,
//! process/provider calls, collector qualification or module-admission claim.

use habitat_engine::contracts::Generation;
use habitat_engine::task::LoopRefusal;
use habitat_engine::task::driver::{
    Acceptance, Checked, Error, Outcome, Runtime, StopReason, Work, run,
};
use std::cell::Cell;
use std::collections::VecDeque;
use std::fmt;
use std::time::Duration;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Fault(&'static str);
impl fmt::Display for Fault {
    fn fmt(&self, output: &mut fmt::Formatter<'_>) -> fmt::Result {
        output.write_str(self.0)
    }
}
impl std::error::Error for Fault {}

struct Attempt {
    token: u64,
}
struct Evidence(Vec<u8>);

#[derive(Debug, Eq, PartialEq)]
enum Call {
    Cancellation,
    Begin(String),
    Execute(u64),
    Verify(u64),
    Accept(u64, Vec<u8>),
    Stop(String),
}

struct Fake {
    clock: Cell<Duration>,
    clock_reads: Cell<usize>,
    calls: Vec<Call>,
    cancellations: VecDeque<Result<bool, Fault>>,
    works: VecDeque<Result<(Work, Duration), Fault>>,
    checks: VecDeque<Result<(Checked<Evidence>, Duration), Fault>>,
    begin_error: Option<Fault>,
    accept_error: Option<Fault>,
    acceptance: Acceptance,
    stop_error: Option<Fault>,
    stop_settled: bool,
    next_attempt: u64,
}

impl Fake {
    fn new(checks: Vec<Checked<Evidence>>) -> Self {
        let works = (0..checks.len())
            .map(|_| Ok((Work::ReadyForCheck, Duration::ZERO)))
            .collect();
        Self {
            clock: Cell::new(Duration::ZERO),
            clock_reads: Cell::new(0),
            calls: Vec::new(),
            cancellations: VecDeque::new(),
            works,
            checks: checks
                .into_iter()
                .map(|check| Ok((check, Duration::ZERO)))
                .collect(),
            begin_error: None,
            accept_error: None,
            acceptance: Acceptance::Accepted,
            stop_error: None,
            stop_settled: true,
            next_attempt: 100,
        }
    }

    fn begins(&self) -> Vec<&str> {
        self.calls
            .iter()
            .filter_map(|call| match call {
                Call::Begin(ordinal) => Some(ordinal.as_str()),
                _ => None,
            })
            .collect()
    }

    fn no_accept(&self) {
        assert!(
            !self
                .calls
                .iter()
                .any(|call| matches!(call, Call::Accept(..)))
        );
    }

    fn no_stop(&self) {
        assert!(!self.calls.iter().any(|call| matches!(call, Call::Stop(_))));
    }

    fn verify_count(&self) -> usize {
        self.calls
            .iter()
            .filter(|call| matches!(call, Call::Verify(_)))
            .count()
    }
}

impl Runtime for Fake {
    type Error = Fault;
    type Attempt = Attempt;
    type Evidence = Evidence;

    fn elapsed(&self) -> Duration {
        self.clock_reads.set(self.clock_reads.get() + 1);
        self.clock.get()
    }

    fn cancellation_requested(&mut self) -> Result<bool, Self::Error> {
        self.calls.push(Call::Cancellation);
        self.cancellations.pop_front().unwrap_or(Ok(false))
    }

    fn begin(&mut self, ordinal: Generation) -> Result<Self::Attempt, Self::Error> {
        self.calls.push(Call::Begin(ordinal.to_string()));
        if let Some(error) = self.begin_error {
            return Err(error);
        }
        self.next_attempt += 1;
        Ok(Attempt {
            token: self.next_attempt,
        })
    }

    fn execute(&mut self, attempt: &Self::Attempt) -> Result<Work, Self::Error> {
        self.calls.push(Call::Execute(attempt.token));
        let (work, elapsed) = self.works.pop_front().expect("unexpected execute port")?;
        self.clock.set(elapsed);
        Ok(work)
    }

    fn verify(&mut self, attempt: &Self::Attempt) -> Result<Checked<Self::Evidence>, Self::Error> {
        self.calls.push(Call::Verify(attempt.token));
        let (check, elapsed) = self.checks.pop_front().expect("unexpected verify port")?;
        self.clock.set(elapsed);
        Ok(check)
    }

    fn accept(
        &mut self,
        attempt: &Self::Attempt,
        evidence: Self::Evidence,
    ) -> Result<Acceptance, Self::Error> {
        self.calls.push(Call::Accept(attempt.token, evidence.0));
        self.accept_error.map_or(Ok(self.acceptance), Err)
    }

    fn stop(&mut self, reason: StopReason) -> Result<bool, Self::Error> {
        self.calls.push(Call::Stop(format!("{reason:?}")));
        self.stop_error.map_or(Ok(self.stop_settled), Err)
    }
}

fn passed(criteria: u64, marker: u8) -> Checked<Evidence> {
    Checked::Passed {
        evidence: Evidence(vec![marker, 0, 255]),
        criteria,
    }
}

fn failed(criteria: u64) -> Checked<Evidence> {
    Checked::Failed { criteria }
}

fn ms(value: u64) -> Duration {
    Duration::from_millis(value)
}

#[test]
fn invalid_criterion_counts_refuse_before_any_runtime_port_or_clock_read() {
    for count in [0, 65, u8::MAX] {
        let mut fake = Fake::new(vec![]);
        assert!(matches!(
            run(&mut fake, count),
            Err(Error::Policy(LoopRefusal::InvalidCriteria))
        ));
        assert!(fake.calls.is_empty());
        assert_eq!(fake.clock_reads.get(), 0);
    }
}

#[test]
fn successful_path_preserves_owned_attempt_evidence_and_reservation_first_order() {
    let mut fake = Fake::new(vec![passed(3, 42)]);
    assert!(matches!(run(&mut fake, 2).unwrap(), Outcome::Accepted));
    assert_eq!(
        fake.calls,
        vec![
            Call::Cancellation,
            Call::Begin("1".to_owned()),
            Call::Execute(101),
            Call::Cancellation,
            Call::Verify(101),
            Call::Cancellation,
            Call::Accept(101, vec![42, 0, 255])
        ]
    );
}

#[test]
fn worker_failure_never_reaches_checker_and_terminal_status_depends_on_settlement() {
    for settled in [true, false] {
        let mut fake = Fake::new(vec![]);
        fake.works.push_back(Ok((Work::Failed, Duration::ZERO)));
        fake.stop_settled = settled;
        let outcome = run(&mut fake, 1).unwrap();
        if settled {
            assert!(matches!(
                outcome,
                Outcome::Stopped(StopReason::WorkerFailed)
            ));
        } else {
            assert!(matches!(
                outcome,
                Outcome::NeedsSettlement(StopReason::WorkerFailed)
            ));
        }
        assert_eq!(fake.verify_count(), 0);
        fake.no_accept();
    }
}

#[test]
fn unsettled_worker_cannot_be_verified_or_reported_cleaned_without_owner_observation() {
    let mut fake = Fake::new(vec![]);
    fake.works.push_back(Ok((Work::Unsettled, Duration::ZERO)));
    fake.stop_settled = false;
    assert!(matches!(
        run(&mut fake, 1).unwrap(),
        Outcome::NeedsSettlement(StopReason::Unsettled)
    ));
    assert_eq!(fake.verify_count(), 0);
    fake.no_accept();
    assert_eq!(fake.begins(), ["1"]);
}

#[test]
fn failed_check_requires_new_owned_attempt_and_its_own_successful_verification() {
    let mut fake = Fake::new(vec![failed(1), passed(3, 91)]);
    assert!(matches!(run(&mut fake, 2).unwrap(), Outcome::Accepted));
    assert_eq!(fake.begins(), ["1", "2"]);
    assert_eq!(fake.verify_count(), 2);
    assert!(fake.calls.contains(&Call::Verify(101)));
    assert!(fake.calls.contains(&Call::Verify(102)));
    assert_eq!(
        fake.calls.last(),
        Some(&Call::Accept(102, vec![91, 0, 255]))
    );
    fake.no_stop();
}

#[test]
fn two_attempts_without_any_new_criterion_stop_before_a_third_begin() {
    let mut fake = Fake::new(vec![failed(0), failed(0)]);
    assert!(matches!(
        run(&mut fake, 1).unwrap(),
        Outcome::Stopped(StopReason::Policy(LoopRefusal::NoProgress))
    ));
    assert_eq!(fake.begins(), ["1", "2"]);
    assert_eq!(fake.verify_count(), 2);
    fake.no_accept();
}

#[test]
fn newly_satisfied_criterion_resets_no_progress_and_permits_attempt_three() {
    let mut fake = Fake::new(vec![failed(0), failed(1), passed(3, 93)]);
    assert!(matches!(run(&mut fake, 2).unwrap(), Outcome::Accepted));
    assert_eq!(fake.begins(), ["1", "2", "3"]);
    assert_eq!(
        fake.calls.last(),
        Some(&Call::Accept(103, vec![93, 0, 255]))
    );
}

#[test]
fn three_attempt_limit_stops_even_when_every_attempt_reports_new_progress() {
    let mut fake = Fake::new(vec![failed(1), failed(2), failed(4)]);
    assert!(matches!(
        run(&mut fake, 3).unwrap(),
        Outcome::Stopped(StopReason::Policy(LoopRefusal::AttemptsExhausted))
    ));
    assert_eq!(fake.begins(), ["1", "2", "3"]);
    assert_eq!(fake.verify_count(), 3);
    fake.no_accept();
}

#[test]
fn historical_union_of_criteria_cannot_accept_a_partial_current_candidate() {
    let mut fake = Fake::new(vec![failed(1), passed(2, 77)]);
    assert!(matches!(
        run(&mut fake, 2).unwrap(),
        Outcome::Stopped(StopReason::InvalidCheck)
    ));
    assert_eq!(fake.begins(), ["1", "2"]);
    fake.no_accept();
}

#[test]
fn passed_summary_with_missing_or_undeclared_bits_is_invalid_not_a_repair_signal() {
    for mask in [0, 1, 2, 4, 7, u64::MAX] {
        let mut fake = Fake::new(vec![passed(mask, 80)]);
        assert!(matches!(
            run(&mut fake, 2).unwrap(),
            Outcome::Stopped(StopReason::InvalidCheck)
        ));
        assert_eq!(fake.begins(), ["1"]);
        fake.no_accept();
    }
}

#[test]
fn failed_check_with_undeclared_bits_returns_policy_error_without_invented_stop() {
    let mut fake = Fake::new(vec![failed(2)]);
    assert!(matches!(
        run(&mut fake, 1),
        Err(Error::Policy(LoopRefusal::InvalidCriteria))
    ));
    assert_eq!(fake.begins(), ["1"]);
    fake.no_accept();
    fake.no_stop();
}

#[test]
fn invalid_error_and_timeout_checker_results_preserve_distinct_stop_reasons() {
    for (check, expected) in [
        (Checked::Invalid, "InvalidCheck"),
        (Checked::Error, "VerifierError"),
        (Checked::Timeout, "VerifierTimeout"),
    ] {
        let mut fake = Fake::new(vec![check]);
        let outcome = run(&mut fake, 1).unwrap();
        assert!(matches!(outcome, Outcome::Stopped(_)));
        assert_eq!(fake.calls.last(), Some(&Call::Stop(expected.to_owned())));
        assert_eq!(fake.begins(), ["1"]);
        fake.no_accept();
    }
}

#[test]
fn unsettled_checker_result_cannot_become_terminal_when_stop_has_not_settled() {
    let mut fake = Fake::new(vec![Checked::Unsettled]);
    fake.stop_settled = false;
    assert!(matches!(
        run(&mut fake, 1).unwrap(),
        Outcome::NeedsSettlement(StopReason::Unsettled)
    ));
    assert_eq!(fake.verify_count(), 1);
    fake.no_accept();
}

#[test]
fn cancellation_before_begin_stops_without_reservation_or_dispatch() {
    let mut fake = Fake::new(vec![]);
    fake.cancellations.push_back(Ok(true));
    fake.stop_settled = false;
    assert!(matches!(
        run(&mut fake, 1).unwrap(),
        Outcome::NeedsSettlement(StopReason::Cancelled)
    ));
    assert!(fake.begins().is_empty());
    assert_eq!(
        fake.calls,
        vec![Call::Cancellation, Call::Stop("Cancelled".to_owned())]
    );
}

#[test]
fn cancellation_after_worker_return_prevents_checker_dispatch() {
    let mut fake = Fake::new(vec![]);
    fake.works
        .push_back(Ok((Work::ReadyForCheck, Duration::ZERO)));
    fake.cancellations = [Ok(false), Ok(true)].into();
    assert!(matches!(
        run(&mut fake, 1).unwrap(),
        Outcome::Stopped(StopReason::Cancelled)
    ));
    assert_eq!(fake.verify_count(), 0);
    fake.no_accept();
    assert_eq!(fake.begins(), ["1"]);
}

#[test]
fn cancellation_after_checker_port_return_prevents_acceptance() {
    let mut fake = Fake::new(vec![passed(1, 11)]);
    fake.cancellations = [Ok(false), Ok(false), Ok(true)].into();
    assert!(matches!(
        run(&mut fake, 1).unwrap(),
        Outcome::Stopped(StopReason::Cancelled)
    ));
    assert_eq!(fake.verify_count(), 1);
    fake.no_accept();
    assert_eq!(fake.calls.last(), Some(&Call::Stop("Cancelled".to_owned())));
}

#[test]
fn dispatch_boundary_is_900_seconds_and_total_deadline_remains_1200_seconds() {
    for (elapsed, expected) in [
        (899_999, None),
        (900_000, Some(LoopRefusal::CleanupReserve)),
        (1_199_999, Some(LoopRefusal::CleanupReserve)),
        (1_200_000, Some(LoopRefusal::Deadline)),
    ] {
        let mut fake = Fake::new(vec![passed(1, 11)]);
        fake.clock.set(ms(elapsed));
        fake.works = [Ok((Work::ReadyForCheck, ms(elapsed)))].into();
        fake.checks = [Ok((passed(1, 11), ms(elapsed)))].into();
        let outcome = run(&mut fake, 1).unwrap();
        if let Some(reason) = expected {
            assert!(
                matches!(outcome,Outcome::Stopped(StopReason::Policy(actual)) if actual==reason)
            );
            assert!(fake.begins().is_empty());
            fake.no_accept();
        } else {
            assert!(matches!(outcome, Outcome::Accepted));
            assert_eq!(fake.begins(), ["1"]);
        }
    }
}

#[test]
fn reserved_final_window_can_verify_existing_work_without_starting_new_work() {
    let mut fake = Fake::new(vec![]);
    fake.clock.set(ms(899_999));
    fake.works.push_back(Ok((Work::ReadyForCheck, ms(900_000))));
    fake.checks.push_back(Ok((passed(1, 12), ms(1_199_999))));
    assert!(matches!(run(&mut fake, 1).unwrap(), Outcome::Accepted));
    assert_eq!(fake.begins(), ["1"]);
    assert_eq!(fake.verify_count(), 1);
}

#[test]
fn checker_pass_returned_at_total_deadline_is_observed_but_not_accepted() {
    let mut fake = Fake::new(vec![]);
    fake.works.push_back(Ok((Work::ReadyForCheck, ms(1000))));
    fake.checks.push_back(Ok((passed(1, 12), ms(1_200_000))));
    assert!(matches!(
        run(&mut fake, 1).unwrap(),
        Outcome::Stopped(StopReason::Policy(LoopRefusal::Deadline))
    ));
    assert_eq!(fake.verify_count(), 1);
    fake.no_accept();
}

#[test]
fn repair_does_not_reset_elapsed_origin_or_renew_the_dispatch_window() {
    let mut fake = Fake::new(vec![]);
    fake.works.push_back(Ok((Work::ReadyForCheck, ms(100))));
    fake.checks.push_back(Ok((failed(1), ms(900_000))));
    assert!(matches!(
        run(&mut fake, 2).unwrap(),
        Outcome::Stopped(StopReason::Policy(LoopRefusal::CleanupReserve))
    ));
    assert_eq!(fake.begins(), ["1"]);
    assert_eq!(fake.verify_count(), 1);
    fake.no_accept();
}

#[test]
fn regressed_elapsed_clock_cannot_accept_even_a_complete_current_pass() {
    let mut fake = Fake::new(vec![]);
    fake.clock.set(ms(1000));
    fake.works.push_back(Ok((Work::ReadyForCheck, ms(1000))));
    fake.checks.push_back(Ok((passed(1, 22), ms(999))));
    assert!(matches!(
        run(&mut fake, 1),
        Err(Error::Policy(LoopRefusal::ClockRegression))
    ));
    fake.no_accept();
    fake.no_stop();
    assert_eq!(fake.begins(), ["1"]);
}

#[test]
fn failed_begin_propagates_before_execute_and_without_fabricated_settlement() {
    let mut fake = Fake::new(vec![]);
    fake.begin_error = Some(Fault("begin"));
    assert!(matches!(
        run(&mut fake, 1),
        Err(Error::Runtime(Fault("begin")))
    ));
    assert_eq!(
        fake.calls,
        vec![Call::Cancellation, Call::Begin("1".to_owned())]
    );
}

#[test]
fn execute_port_error_propagates_without_check_accept_or_invented_stop() {
    let mut fake = Fake::new(vec![]);
    fake.works.push_back(Err(Fault("execute")));
    assert!(matches!(
        run(&mut fake, 1),
        Err(Error::Runtime(Fault("execute")))
    ));
    assert_eq!(fake.verify_count(), 0);
    fake.no_accept();
    fake.no_stop();
}

#[test]
fn verify_port_error_cannot_be_relabelled_as_a_candidate_failure_or_success() {
    let mut fake = Fake::new(vec![]);
    fake.works
        .push_back(Ok((Work::ReadyForCheck, Duration::ZERO)));
    fake.checks.push_back(Err(Fault("verify")));
    assert!(matches!(
        run(&mut fake, 1),
        Err(Error::Runtime(Fault("verify")))
    ));
    assert_eq!(fake.verify_count(), 1);
    assert_eq!(fake.begins(), ["1"]);
    fake.no_accept();
    fake.no_stop();
}

#[test]
fn accept_port_error_propagates_without_reporting_success_or_retrying_side_effects() {
    let mut fake = Fake::new(vec![passed(1, 31)]);
    fake.accept_error = Some(Fault("accept"));
    assert!(matches!(
        run(&mut fake, 1),
        Err(Error::Runtime(Fault("accept")))
    ));
    assert_eq!(
        fake.calls.last(),
        Some(&Call::Accept(101, vec![31, 0, 255]))
    );
    assert_eq!(fake.begins(), ["1"]);
    fake.no_stop();
}

#[test]
fn cancellation_read_errors_at_each_boundary_propagate_without_accept_or_stop() {
    for successful_reads in 0..3 {
        let mut fake = Fake::new(vec![passed(1, 40)]);
        fake.cancellations = std::iter::repeat_n(Ok(false), successful_reads)
            .chain(std::iter::once(Err(Fault("cancel read"))))
            .collect();
        assert!(matches!(
            run(&mut fake, 1),
            Err(Error::Runtime(Fault("cancel read")))
        ));
        fake.no_accept();
        fake.no_stop();
        assert_eq!(fake.verify_count(), usize::from(successful_reads == 2));
    }
}

#[test]
fn stop_port_error_cannot_invent_a_settled_or_unsettled_successful_outcome() {
    let mut fake = Fake::new(vec![]);
    fake.cancellations.push_back(Ok(true));
    fake.stop_error = Some(Fault("stop"));
    assert!(matches!(
        run(&mut fake, 1),
        Err(Error::Runtime(Fault("stop")))
    ));
    assert_eq!(
        fake.calls,
        vec![Call::Cancellation, Call::Stop("Cancelled".to_owned())]
    );
}

#[test]
fn exhausted_policy_with_unsettled_cleanup_preserves_reason_and_needs_settlement() {
    let mut fake = Fake::new(vec![]);
    fake.clock.set(ms(900_000));
    fake.stop_settled = false;
    assert!(matches!(
        run(&mut fake, 1).unwrap(),
        Outcome::NeedsSettlement(StopReason::Policy(LoopRefusal::CleanupReserve))
    ));
    assert!(fake.begins().is_empty());
    fake.no_accept();
}

#[test]
fn all_64_declared_criteria_can_pass_without_shift_overflow_or_truncation() {
    let mut fake = Fake::new(vec![passed(u64::MAX, 64)]);
    assert!(matches!(run(&mut fake, 64).unwrap(), Outcome::Accepted));
    assert_eq!(
        fake.calls.last(),
        Some(&Call::Accept(101, vec![64, 0, 255]))
    );
}

#[test]
fn a_cancellation_first_at_acceptance_stops_cancelled_with_no_later_begin() {
    for settled in [true, false] {
        let mut fake = Fake::new(vec![passed(3, 7)]);
        fake.acceptance = Acceptance::CancelledFirst;
        fake.stop_settled = settled;
        let outcome = run(&mut fake, 2).unwrap();
        if settled {
            assert_eq!(outcome, Outcome::Stopped(StopReason::Cancelled));
        } else {
            assert_eq!(outcome, Outcome::NeedsSettlement(StopReason::Cancelled));
        }
        assert_eq!(
            fake.calls,
            vec![
                Call::Cancellation,
                Call::Begin("1".to_owned()),
                Call::Execute(101),
                Call::Cancellation,
                Call::Verify(101),
                Call::Cancellation,
                Call::Accept(101, vec![7, 0, 255]),
                Call::Stop("Cancelled".to_owned()),
            ],
            "settled={settled}: one stop after the refused acceptance, no begin"
        );
        assert_eq!(fake.begins(), ["1"]);
    }
}
