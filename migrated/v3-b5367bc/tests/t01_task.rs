//! Primary owner: task. Pure RC01 loop policy; clocks are injected, never slept.
//! A reconciled-attempt call is a trusted input assumption, not authenticated proof.
//! These tests do not qualify storage, dispatch, cleanup, or parent admission.
//! Table rows do not multiply behavioral case credit or establish the 50-case floor.

use std::time::Duration;

use habitat_engine::{
    contracts::Generation,
    task::{LoopGuard, LoopRefusal},
};

fn generation(text: &str) -> Generation {
    text.parse().unwrap()
}

fn guard(criteria: u8) -> LoopGuard {
    LoopGuard::new(criteria, generation("1")).unwrap()
}

fn millis(value: u64) -> Duration {
    Duration::from_millis(value)
}

#[test]
fn criterion_count_must_fit_the_nonempty_declared_set() {
    for count in [0, 65, u8::MAX] {
        assert_eq!(
            LoopGuard::new(count, generation("1")).unwrap_err(),
            LoopRefusal::InvalidCriteria
        );
    }
}

#[test]
fn all_64_criteria_including_the_high_bit_are_representable() {
    let mut policy = guard(64);
    let attempt = policy.begin_attempt(millis(0)).unwrap();
    assert_eq!(
        policy.record_reconciled_attempt(attempt, u64::MAX, millis(1)),
        Ok(())
    );
}

#[test]
fn first_attempt_preserves_initial_generation_and_retry_advances_it() {
    let mut policy = LoopGuard::new(1, generation("41")).unwrap();
    let first = policy.begin_attempt(millis(0)).unwrap();
    assert_eq!(first.value(), 41);
    policy
        .record_reconciled_attempt(first, 1, millis(1))
        .unwrap();
    assert_eq!(policy.begin_attempt(millis(2)).unwrap().value(), 42);
}

#[test]
fn outstanding_attempt_refuses_duplicate_dispatch() {
    let mut policy = guard(1);
    let first = policy.begin_attempt(millis(0)).unwrap();
    assert_eq!(
        policy.begin_attempt(millis(2)),
        Err(LoopRefusal::ActiveAttempt)
    );
    assert_eq!(
        policy.record_reconciled_attempt(first, 1, millis(1)),
        Err(LoopRefusal::ClockRegression)
    );
    policy
        .record_reconciled_attempt(first, 1, millis(2))
        .unwrap();
    assert_eq!(policy.begin_attempt(millis(2)).unwrap().value(), 2);
}

#[test]
fn wrong_generation_cannot_reconcile_the_active_attempt() {
    let mut policy = guard(1);
    let first = policy.begin_attempt(millis(1)).unwrap();
    assert_eq!(
        policy.record_reconciled_attempt(generation("2"), 1, millis(10)),
        Err(LoopRefusal::StaleAttempt)
    );
    assert_eq!(
        policy.record_reconciled_attempt(first, 1, millis(2)),
        Err(LoopRefusal::ClockRegression)
    );
    assert_eq!(
        policy.record_reconciled_attempt(first, 1, millis(11)),
        Ok(())
    );
}

#[test]
fn duplicate_reconciliation_is_not_another_completed_attempt() {
    let mut policy = guard(1);
    let first = policy.begin_attempt(millis(0)).unwrap();
    policy
        .record_reconciled_attempt(first, 0, millis(1))
        .unwrap();
    assert_eq!(
        policy.record_reconciled_attempt(first, 0, millis(2)),
        Err(LoopRefusal::StaleAttempt)
    );
    assert!(policy.begin_attempt(millis(2)).is_ok());
}

#[test]
fn previous_generation_cannot_settle_a_retry() {
    let mut policy = guard(1);
    let first = policy.begin_attempt(millis(0)).unwrap();
    policy
        .record_reconciled_attempt(first, 1, millis(1))
        .unwrap();
    let second = policy.begin_attempt(millis(2)).unwrap();
    assert_eq!(
        policy.record_reconciled_attempt(first, 1, millis(3)),
        Err(LoopRefusal::StaleAttempt)
    );
    assert_eq!(
        policy.begin_attempt(millis(3)),
        Err(LoopRefusal::ActiveAttempt)
    );
    assert_eq!(
        policy.record_reconciled_attempt(second, 1, millis(3)),
        Ok(())
    );
}

#[test]
fn undeclared_criterion_refusal_preserves_attempt_but_retains_observed_time() {
    let mut policy = guard(1);
    let first = policy.begin_attempt(millis(0)).unwrap();
    let before = policy.progress();
    assert_eq!(
        policy.record_reconciled_attempt(first, 2, millis(100)),
        Err(LoopRefusal::InvalidCriteria)
    );
    assert_eq!(policy.progress(), before);
    assert_eq!(
        policy.record_reconciled_attempt(first, 1, millis(1)),
        Err(LoopRefusal::ClockRegression)
    );
    assert_eq!(policy.progress(), before);
    assert_eq!(
        policy.record_reconciled_attempt(first, 1, millis(100)),
        Ok(())
    );
}

#[test]
fn one_attempt_without_progress_still_allows_retry() {
    let mut policy = guard(1);
    let first = policy.begin_attempt(millis(0)).unwrap();
    policy
        .record_reconciled_attempt(first, 0, millis(1))
        .unwrap();
    assert_eq!(policy.begin_attempt(millis(2)).unwrap().value(), 2);
}

#[test]
fn two_consecutive_attempts_without_progress_stop_before_third() {
    let mut policy = guard(1);
    for time in [0, 2] {
        let attempt = policy.begin_attempt(millis(time)).unwrap();
        policy
            .record_reconciled_attempt(attempt, 0, millis(time + 1))
            .unwrap();
    }
    assert_eq!(
        policy.begin_attempt(millis(4)),
        Err(LoopRefusal::NoProgress)
    );
}

#[test]
fn newly_satisfied_criterion_resets_consecutive_no_progress() {
    let mut policy = guard(1);
    let first = policy.begin_attempt(millis(0)).unwrap();
    policy
        .record_reconciled_attempt(first, 0, millis(1))
        .unwrap();
    let second = policy.begin_attempt(millis(2)).unwrap();
    policy
        .record_reconciled_attempt(second, 1, millis(3))
        .unwrap();
    assert_eq!(policy.progress().consecutive_no_progress, 0);
    assert!(policy.begin_attempt(millis(4)).is_ok());
}

#[test]
fn reappearing_old_criterion_does_not_count_as_new_progress() {
    let mut policy = guard(2);
    for (time, criteria) in [(0, 1), (2, 0), (4, 1)] {
        let attempt = policy.begin_attempt(millis(time)).unwrap();
        policy
            .record_reconciled_attempt(attempt, criteria, millis(time + 1))
            .unwrap();
    }
    let progress = policy.progress();
    assert_eq!(progress.consecutive_no_progress, 2);
    assert_eq!(progress.ever_satisfied_criteria, 1);
    assert_eq!(progress.attempts_started, 3);
    assert_eq!(progress.active_generation, None);
}

#[test]
fn mixed_repeated_and_new_criteria_still_make_progress() {
    let mut policy = guard(2);
    for (time, criteria) in [(0, 1), (2, 0), (4, 3)] {
        let attempt = policy.begin_attempt(millis(time)).unwrap();
        policy
            .record_reconciled_attempt(attempt, criteria, millis(time + 1))
            .unwrap();
    }
    assert_eq!(policy.progress().consecutive_no_progress, 0);
    assert_eq!(policy.progress().ever_satisfied_criteria, 3);
}

#[test]
fn three_attempts_are_allowed_but_progress_never_authorizes_a_fourth() {
    let mut policy = guard(3);
    for (time, criteria) in [(0, 1), (2, 2), (4, 4)] {
        let attempt = policy.begin_attempt(millis(time)).unwrap();
        policy
            .record_reconciled_attempt(attempt, criteria, millis(time + 1))
            .unwrap();
    }
    assert_eq!(
        policy.begin_attempt(millis(6)),
        Err(LoopRefusal::AttemptsExhausted)
    );
}

#[test]
fn dispatch_is_allowed_immediately_before_cleanup_reserve() {
    assert!(guard(1).begin_attempt(millis(899_999)).is_ok());
}

#[test]
fn cleanup_reserve_starts_at_exactly_fifteen_minutes() {
    for time in [900_000, 1_199_999] {
        let mut policy = guard(1);
        assert_eq!(
            policy.begin_attempt(millis(time)),
            Err(LoopRefusal::CleanupReserve)
        );
        assert_eq!(
            policy.begin_attempt(millis(time - 1)),
            Err(LoopRefusal::ClockRegression)
        );
        assert_eq!(
            policy.begin_attempt(millis(time)),
            Err(LoopRefusal::CleanupReserve)
        );
    }
}

#[test]
fn task_deadline_is_inclusive_and_cannot_overflow() {
    for elapsed in [millis(1_200_000), millis(1_200_001), Duration::MAX] {
        assert_eq!(guard(1).begin_attempt(elapsed), Err(LoopRefusal::Deadline));
    }
}

#[test]
fn retry_uses_the_original_time_origin() {
    let mut policy = guard(1);
    let first = policy.begin_attempt(millis(899_000)).unwrap();
    policy
        .record_reconciled_attempt(first, 1, millis(899_500))
        .unwrap();
    assert_eq!(
        policy.begin_attempt(millis(900_000)),
        Err(LoopRefusal::CleanupReserve)
    );
}

#[test]
fn observed_deadline_cannot_be_reopened_by_rewinding_time() {
    let mut policy = guard(1);
    let before = policy.progress();
    assert_eq!(
        policy.begin_attempt(millis(1_200_000)),
        Err(LoopRefusal::Deadline)
    );
    assert_eq!(policy.progress(), before);
    assert_eq!(
        policy.begin_attempt(millis(0)),
        Err(LoopRefusal::ClockRegression)
    );
    for time in [1_200_000, 1_200_001] {
        assert_eq!(
            policy.begin_attempt(millis(time)),
            Err(LoopRefusal::Deadline)
        );
    }
    assert_eq!(policy.progress(), before);
}

#[test]
fn reconciliation_clock_regression_refuses_without_settling() {
    let mut policy = guard(1);
    let first = policy.begin_attempt(millis(10)).unwrap();
    assert_eq!(
        policy.record_reconciled_attempt(first, 1, millis(9)),
        Err(LoopRefusal::ClockRegression)
    );
    assert_eq!(
        policy.record_reconciled_attempt(first, 1, millis(10)),
        Ok(())
    );
}

#[test]
fn dispatch_clock_cannot_move_backwards_after_reconciliation() {
    let mut policy = guard(1);
    let first = policy.begin_attempt(millis(0)).unwrap();
    policy
        .record_reconciled_attempt(first, 1, millis(10))
        .unwrap();
    assert_eq!(
        policy.begin_attempt(millis(9)),
        Err(LoopRefusal::ClockRegression)
    );
    assert!(policy.begin_attempt(millis(10)).is_ok());
}

#[test]
fn cancellation_before_dispatch_is_permanent_and_idempotent() {
    let mut policy = guard(1);
    policy.request_cancel();
    policy.request_cancel();
    assert_eq!(policy.begin_attempt(millis(0)), Err(LoopRefusal::Cancelled));
    assert_eq!(policy.begin_attempt(millis(1)), Err(LoopRefusal::Cancelled));
}

#[test]
fn cancellation_intent_keeps_active_attempt_available_for_reconciliation() {
    let mut policy = guard(1);
    let first = policy.begin_attempt(millis(0)).unwrap();
    policy.request_cancel();
    assert_eq!(policy.progress().active_generation, Some(first));
    assert_eq!(policy.begin_attempt(millis(1)), Err(LoopRefusal::Cancelled));
    assert_eq!(
        policy.record_reconciled_attempt(first, 1, millis(2)),
        Ok(())
    );
    assert_eq!(policy.begin_attempt(millis(3)), Err(LoopRefusal::Cancelled));
}

#[test]
fn unresolvable_authority_blocks_work_even_when_limits_allow_it() {
    let mut policy = guard(1);
    policy.block_authority();
    policy.block_authority();
    assert_eq!(
        policy.begin_attempt(millis(0)),
        Err(LoopRefusal::AuthorityBlocked)
    );
}

#[test]
fn authority_block_does_not_erase_outstanding_reconciliation() {
    let mut policy = guard(1);
    let first = policy.begin_attempt(millis(0)).unwrap();
    policy.block_authority();
    assert_eq!(
        policy.record_reconciled_attempt(first, 1, millis(1)),
        Ok(())
    );
    assert_eq!(
        policy.begin_attempt(millis(2)),
        Err(LoopRefusal::AuthorityBlocked)
    );
}

#[test]
fn late_reconciliation_preserves_observation_without_reopening_deadline() {
    let mut policy = guard(1);
    let first = policy.begin_attempt(millis(0)).unwrap();
    assert_eq!(
        policy.record_reconciled_attempt(first, 1, millis(1_200_001)),
        Ok(())
    );
    assert_eq!(
        policy.begin_attempt(millis(1_200_001)),
        Err(LoopRefusal::Deadline)
    );
}

#[test]
fn generation_exhaustion_refuses_retry_without_wrapping() {
    let mut policy = LoopGuard::new(1, generation("18446744073709551615")).unwrap();
    let first = policy.begin_attempt(millis(0)).unwrap();
    assert_eq!(first.value(), u64::MAX);
    policy
        .record_reconciled_attempt(first, 1, millis(1))
        .unwrap();
    assert_eq!(
        policy.begin_attempt(millis(2)),
        Err(LoopRefusal::GenerationExhausted)
    );
    assert_eq!(
        policy.begin_attempt(millis(3)),
        Err(LoopRefusal::GenerationExhausted)
    );
}
