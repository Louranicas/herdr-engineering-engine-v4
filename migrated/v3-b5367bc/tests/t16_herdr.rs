//! T16 herdr client-surface cases (`T16-HD-nn`). Every case builds its world as values.
//! There is no pane, socket, process or clock here.
//!
//! The contract's required proof is *"pane loss and reconnect; duplicate UI submission;
//! stale event; explicit cancellation; truthful failure/unknown presentation and proof
//! navigation"*.

use std::error::Error;

use habitat_engine::herdr::{
    Admitted, EngineReceipt, Epoch, Freshness, IntentKey, MAX_BUFFERED_EVENTS,
    MAX_EVENT_TEXT_BYTES, MAX_EVIDENCE_REFS, MAX_RECONNECTS, MAX_TASKS, Refusal, Snapshot, Status,
    View, render,
};

type Outcome = Result<(), Box<dyn Error>>;

fn id(index: usize) -> String {
    format!("{index:08x}-0000-4000-8000-000000000000")
}

const EPOCH_1: &str = "00000001-eeee-4eee-9eee-eeeeeeeeeeee";
const EPOCH_2: &str = "00000002-eeee-4eee-9eee-eeeeeeeeeeee";
const EPOCH_3: &str = "00000003-eeee-4eee-9eee-eeeeeeeeeeee";
const EPOCH_9999: &str = "0000270f-eeee-4eee-9eee-eeeeeeeeeeee";

/// The wire text of an engine epoch (a `UUIDv4`, distinct in form from task identities).
fn epoch_text(index: usize) -> String {
    format!("{index:08x}-eeee-4eee-9eee-eeeeeeeeeeee")
}

fn ep(index: usize) -> Result<Epoch, Refusal> {
    Epoch::parse(&epoch_text(index))
}

/// An intent key in the engine's `idempotency_key` form.
fn key(index: usize) -> Result<IntentKey, Refusal> {
    IntentKey::new(&format!("{index:08x}-1111-4111-a111-111111111111"))
}

fn view(epoch: usize) -> Result<View, Refusal> {
    Ok(View::new(ep(epoch)?))
}

/// A snapshot read in epoch 1.
fn snapshot(task: &str, status: Status, sequence: u64) -> Result<Snapshot, Refusal> {
    Ok(Snapshot {
        task: task.to_owned(),
        epoch: ep(1)?,
        status,
        route_explanation: None,
        evidence: Vec::new(),
        gaps: Vec::new(),
        sequence,
    })
}

// ------------------------------------------- acceptance cannot be minted by a client

/// T16-HD-01 · an acceptance exists only via an engine receipt. This case pins the one
/// legitimate path; the absence of any other is what the rest of the family rests on.
#[test]
fn acceptance_comes_only_from_an_engine_receipt() -> Outcome {
    let task = id(1);
    let receipt = EngineReceipt::issue(&task, EPOCH_1, "7")?;
    let admitted = Admitted::from_engine(receipt);
    let acceptance = admitted.acceptance().ok_or("expected an acceptance")?;
    assert_eq!(acceptance.task().as_str(), task);
    assert_eq!(acceptance.receipt().sequence(), 7);
    assert_eq!(acceptance.receipt().epoch().as_str(), epoch_text(1));
    assert!(admitted.is_durable());
    Ok(())
}

/// T16-HD-02 · a denial carries no acceptance at all, so a client rendering it has nothing
/// to mistake for one.
#[test]
fn a_denial_carries_no_acceptance() {
    let denied: Admitted<'_> = Admitted::Denied;
    assert!(denied.acceptance().is_none());
    assert!(!denied.is_durable());
}

/// T16-HD-03 · a malformed task identity is refused when the receipt is issued, before any
/// client sees it.
#[test]
fn a_malformed_task_identity_is_refused_at_issue() {
    use habitat_engine::contracts::ScalarError;
    assert_eq!(
        EngineReceipt::issue("not-a-uuid", EPOCH_1, "1").map(|_| ()),
        Err(Refusal::MalformedIdentity(ScalarError::InvalidUuid))
    );
}

/// T16-HD-04 · only the engine's own terminal verdicts are settled. `Unknown`,
/// `EffectUnknown`, `Blocked` and `CancellationPending` are explicitly unsettled: a client that
/// renders any of them as an outcome is inventing one.
#[test]
fn only_terminal_engine_verdicts_are_settled() {
    assert_eq!(Status::ALL.len(), 9);
    let settled: Vec<&str> = Status::ALL
        .into_iter()
        .filter(|status| status.is_settled())
        .map(Status::name)
        .collect();
    assert_eq!(settled, vec!["passed", "failed", "cancelled", "abandoned"]);
    for status in Status::ALL {
        assert_eq!(status.to_string(), status.name());
    }
}

/// T16-HD-05 · an unknown status presents as unknown and never as a verdict, in a view.
#[test]
fn an_unknown_status_presents_as_unknown() -> Outcome {
    let task = id(1);
    let mut view = view(1)?;
    view.present(snapshot(&task, Status::Unknown, 1)?)?;
    let presented = view.task(&task)?.snapshot();
    assert_eq!(presented.status, Status::Unknown);
    assert!(!presented.status.is_settled());
    Ok(())
}

// ------------------------------------------- duplicate submission

/// T16-HD-06 · a repeated submission under the same intent key reports `Duplicate`, carrying
/// the same acceptance — one durable task, not two.
#[test]
fn a_repeated_submission_is_a_duplicate_not_a_second_task() -> Outcome {
    let task = id(1);
    let mut view = view(1)?;
    let key = key(7)?;
    let first = view.submit(
        &key,
        Admitted::from_engine(EngineReceipt::issue(&task, EPOCH_1, "1")?),
    )?;
    assert!(matches!(first, Admitted::Accepted(_)));
    let second = view.submit(
        &key,
        Admitted::from_engine(EngineReceipt::issue(&task, EPOCH_1, "2")?),
    )?;
    assert!(matches!(second, Admitted::Duplicate(_)));
    assert!(
        second.is_durable(),
        "the task exists, it was just not created twice"
    );
    assert_eq!(view.submitted(&key), Some(task.as_str()));
    Ok(())
}

/// T16-HD-07 · two different intent keys are two submissions, even for the same task text.
#[test]
fn different_intent_keys_are_different_submissions() -> Outcome {
    let mut view = view(1)?;
    let (first, second) = (id(1), id(2));
    let a = view.submit(
        &key(1)?,
        Admitted::from_engine(EngineReceipt::issue(&first, EPOCH_1, "1")?),
    )?;
    let b = view.submit(
        &key(2)?,
        Admitted::from_engine(EngineReceipt::issue(&second, EPOCH_1, "2")?),
    )?;
    assert!(matches!(a, Admitted::Accepted(_)));
    assert!(matches!(b, Admitted::Accepted(_)));
    Ok(())
}

/// T16-HD-08 · a denial is not recorded as a submission, so a retry after a denial is a
/// fresh attempt rather than a duplicate.
#[test]
fn a_denial_does_not_register_the_key() -> Outcome {
    let mut view = view(1)?;
    let key = key(1)?;
    let denied = view.submit(&key, Admitted::Denied)?;
    assert!(matches!(denied, Admitted::Denied));
    assert_eq!(view.submitted(&key), None);
    let task = id(1);
    let retried = view.submit(
        &key,
        Admitted::from_engine(EngineReceipt::issue(&task, EPOCH_1, "1")?),
    )?;
    assert!(
        matches!(retried, Admitted::Accepted(_)),
        "a retry after denial is fresh"
    );
    Ok(())
}

/// T16-HD-09 · a receipt from another epoch is refused at submission: a view cannot record
/// an acceptance that belongs to a world it is not attached to.
#[test]
fn a_receipt_from_another_epoch_is_refused() -> Outcome {
    let mut view = view(1)?;
    assert_eq!(
        view.submit(
            &key(1)?,
            Admitted::from_engine(EngineReceipt::issue(&id(1), EPOCH_2, "1")?)
        )
        .map(|_| ()),
        Err(Refusal::EpochMismatch)
    );
    Ok(())
}

/// T16-HD-10 · an intent key round-trips its text, and equality is by that text.
#[test]
fn an_intent_key_round_trips() -> Outcome {
    let text = "0badcafe-1234-4abc-b123-0123456789ab";
    assert_eq!(IntentKey::new(text)?.as_str(), text);
    assert_eq!(key(1)?, key(1)?);
    assert_ne!(key(1)?, key(2)?);
    Ok(())
}

// ------------------------------------------- stale events

/// T16-HD-11 · an event at or before the cursor is refused as stale, so a replayed event is
/// not rendered as new.
#[test]
fn an_event_at_or_before_the_cursor_is_stale() -> Outcome {
    let mut view = view(1)?;
    view.observe(&ep(1)?, 5, 0, "five")?;
    assert_eq!(view.cursor(), 5);
    assert_eq!(
        view.observe(&ep(1)?, 5, 5, "five again"),
        Err(Refusal::StaleEvent)
    );
    assert_eq!(
        view.observe(&ep(1)?, 4, 5, "four"),
        Err(Refusal::StaleEvent)
    );
    assert_eq!(view.buffered(), 1, "neither stale event was buffered");
    Ok(())
}

/// T16-HD-12 · the staleness boundary is asserted from both sides: cursor + 1 is accepted.
#[test]
fn the_staleness_boundary_admits_the_next_sequence() -> Outcome {
    let mut view = view(1)?;
    view.observe(&ep(1)?, 5, 0, "five")?;
    view.observe(&ep(1)?, 6, 5, "six")?;
    assert_eq!(view.cursor(), 6);
    assert_eq!(view.buffered(), 2);
    Ok(())
}

/// T16-HD-13 · an event from another epoch is refused; the client must reconnect.
#[test]
fn an_event_from_another_epoch_is_refused() -> Outcome {
    let mut view = view(1)?;
    assert_eq!(
        view.observe(&ep(2)?, 1, 0, "x"),
        Err(Refusal::EpochMismatch)
    );
    assert_eq!(view.buffered(), 0);
    Ok(())
}

/// T16-HD-14 · draining takes the buffer and leaves it empty, without moving the cursor —
/// rendering is not acknowledgement.
#[test]
fn draining_empties_the_buffer_without_moving_the_cursor() -> Outcome {
    let mut view = view(1)?;
    view.observe(&ep(1)?, 1, 0, "a")?;
    view.observe(&ep(1)?, 2, 1, "b")?;
    let drained = view.drain();
    assert_eq!(drained, vec![(1, "a".to_owned()), (2, "b".to_owned())]);
    assert_eq!(view.buffered(), 0);
    assert_eq!(view.cursor(), 2, "the cursor is where the client read to");
    Ok(())
}

/// T16-HD-15 · the buffer bound refuses before the event is stored, so a slow renderer
/// cannot make the client grow without bound.
#[test]
fn the_buffer_bound_refuses_before_storing() -> Outcome {
    let mut view = view(1)?;
    for index in 1..=MAX_BUFFERED_EVENTS {
        view.observe(
            &ep(1)?,
            u64::try_from(index)?,
            u64::try_from(index - 1)?,
            "x",
        )?;
    }
    assert_eq!(view.buffered(), MAX_BUFFERED_EVENTS);
    assert_eq!(
        view.observe(
            &ep(1)?,
            u64::try_from(MAX_BUFFERED_EVENTS + 1)?,
            u64::try_from(MAX_BUFFERED_EVENTS)?,
            "one too many",
        ),
        Err(Refusal::BufferFull)
    );
    assert_eq!(view.buffered(), MAX_BUFFERED_EVENTS);
    assert_eq!(
        view.cursor(),
        u64::try_from(MAX_BUFFERED_EVENTS)?,
        "and the refused event did not advance the cursor"
    );
    Ok(())
}

// ------------------------------------------- pane loss and reconnect

/// T16-HD-16 · reconnecting in the same epoch takes the engine's cursor and discards the
/// buffer: anything the client retained across the loss is what it cannot vouch for.
#[test]
fn reconnecting_takes_the_engine_cursor_and_drops_the_buffer() -> Outcome {
    let mut view = view(1)?;
    view.observe(&ep(1)?, 1, 0, "a")?;
    view.observe(&ep(1)?, 2, 1, "b")?;
    assert_eq!(view.buffered(), 2);
    view.reconnect(ep(1)?, 7)?;
    assert_eq!(view.cursor(), 7, "the engine decides where we are");
    assert_eq!(view.buffered(), 0);
    assert_eq!(view.reconnects(), 1);
    Ok(())
}

/// T16-HD-17 · reconnecting in the same epoch keeps presented snapshots — the tasks did not
/// stop existing because the pane did — but marks them stale: what the client held across the
/// loss is the last thing it heard, not the engine's current state (HERDR-G06).
#[test]
fn reconnecting_in_epoch_keeps_presented_tasks_as_stale() -> Outcome {
    let task = id(1);
    let mut view = view(1)?;
    view.present(snapshot(&task, Status::Running, 1)?)?;
    assert_eq!(view.task(&task)?.freshness(), Freshness::Current);
    view.reconnect(ep(1)?, 5)?;
    assert_eq!(view.tasks(), 1);
    assert_eq!(view.task(&task)?.snapshot().status, Status::Running);
    assert_eq!(view.task(&task)?.freshness(), Freshness::Stale);
    Ok(())
}

/// T16-HD-18 · reconnecting into a NEW epoch clears the view entirely: a snapshot from a
/// previous epoch describes a world that no longer exists.
#[test]
fn reconnecting_into_a_new_epoch_clears_the_view() -> Outcome {
    let task = id(1);
    let mut view = view(1)?;
    view.present(snapshot(&task, Status::Passed, 1)?)?;
    view.submit(
        &key(1)?,
        Admitted::from_engine(EngineReceipt::issue(&task, EPOCH_1, "1")?),
    )?;
    view.reconnect(ep(2)?, 0)?;
    assert_eq!(view.epoch().as_str(), epoch_text(2));
    assert_eq!(view.tasks(), 0);
    assert_eq!(view.submitted(&key(1)?), None);
    assert_eq!(view.task(&task).map(|_| ()), Err(Refusal::UnknownTask));
    Ok(())
}

/// T16-HD-19 · a reconnect that reports a cursor behind the client's is refused: the client
/// would have seen something the engine never emitted.
#[test]
fn a_backwards_reconnect_is_refused() -> Outcome {
    let mut view = view(1)?;
    view.observe(&ep(1)?, 9, 0, "nine")?;
    assert_eq!(view.reconnect(ep(1)?, 8), Err(Refusal::CursorAhead));
    assert_eq!(view.cursor(), 9, "and the view did not move");
    assert_eq!(view.reconnects(), 0);
    Ok(())
}

/// T16-HD-20 · a reconnect at exactly the client's cursor is admitted; the boundary is
/// asserted from both sides.
#[test]
fn reconnecting_at_the_same_cursor_is_admitted() -> Outcome {
    let mut view = view(1)?;
    view.observe(&ep(1)?, 9, 0, "nine")?;
    view.reconnect(ep(1)?, 9)?;
    assert_eq!(view.cursor(), 9);
    assert_eq!(view.reconnects(), 1);
    Ok(())
}

/// T16-HD-21 · a backwards cursor in a DIFFERENT epoch is fine: sequences restart, so the
/// comparison only applies within one epoch.
#[test]
fn a_backwards_cursor_in_a_new_epoch_is_admitted() -> Outcome {
    let mut view = view(1)?;
    view.observe(&ep(1)?, 100, 0, "hundred")?;
    view.reconnect(ep(2)?, 0)?;
    assert_eq!(view.cursor(), 0);
    assert_eq!(view.epoch().as_str(), epoch_text(2));
    Ok(())
}

/// T16-HD-22 · the reconnect bound refuses at its limit.
#[test]
fn the_reconnect_bound_refuses_at_its_limit() -> Outcome {
    let mut view = view(1)?;
    for _ in 0..MAX_RECONNECTS {
        view.reconnect(ep(1)?, 0)?;
    }
    assert_eq!(view.reconnects(), MAX_RECONNECTS);
    assert_eq!(view.reconnect(ep(1)?, 0), Err(Refusal::ReconnectLimit));
    Ok(())
}

/// T16-HD-23 · a full loss-and-recover cycle: submit, observe, lose, reconnect, and the
/// durable task is still presented from the engine's own state. This is the contract's
/// integrated proof scenario.
#[test]
fn the_loss_and_recover_cycle_preserves_the_durable_task() -> Outcome {
    let task = id(1);
    let key = key(3)?;
    let mut view = view(1)?;
    let admitted = view.submit(
        &key,
        Admitted::from_engine(EngineReceipt::issue(&task, EPOCH_1, "1")?),
    )?;
    assert!(admitted.is_durable());
    view.observe(&ep(1)?, 1, 0, "dispatched")?;
    view.present(snapshot(&task, Status::Running, 1)?)?;

    // the pane is lost; the client keeps nothing it cannot vouch for
    view.reconnect(ep(1)?, 4)?;
    assert_eq!(view.buffered(), 0);

    // the engine re-presents, and the view carries the outcome it was told
    view.present(Snapshot {
        route_explanation: Some("chose the local adapter on the cost ceiling".to_owned()),
        evidence: vec!["evidence/release/T16-01.json".to_owned()],
        ..snapshot(&task, Status::Passed, 5)?
    })?;
    let presented = view.task(&task)?.snapshot();
    assert_eq!(presented.status, Status::Passed);
    assert!(presented.status.is_settled());
    assert_eq!(presented.evidence.len(), 1, "proof stays navigable");
    assert_eq!(view.submitted(&key), Some(task.as_str()));
    Ok(())
}

// ------------------------------------------- presentation is truthful

/// T16-HD-24 · an out-of-order snapshot does not move a task's presentation backwards.
#[test]
fn an_out_of_order_snapshot_is_ignored() -> Outcome {
    let task = id(1);
    let mut view = view(1)?;
    assert!(view.present(snapshot(&task, Status::Passed, 5)?)?);
    assert!(
        !view.present(snapshot(&task, Status::Running, 3)?)?,
        "an older snapshot is not applied"
    );
    assert_eq!(view.task(&task)?.snapshot().status, Status::Passed);
    Ok(())
}

/// T16-HD-25 · a snapshot at the same sequence is also ignored, so a re-delivered snapshot
/// cannot flip a presentation.
#[test]
fn a_same_sequence_snapshot_is_ignored() -> Outcome {
    let task = id(1);
    let mut view = view(1)?;
    view.present(snapshot(&task, Status::Passed, 5)?)?;
    assert!(!view.present(snapshot(&task, Status::Failed, 5)?)?);
    assert_eq!(view.task(&task)?.snapshot().status, Status::Passed);
    Ok(())
}

/// T16-HD-26 · a newer snapshot is applied and reported as applied.
#[test]
fn a_newer_snapshot_is_applied() -> Outcome {
    let task = id(1);
    let mut view = view(1)?;
    view.present(snapshot(&task, Status::Running, 1)?)?;
    assert!(view.present(snapshot(&task, Status::Failed, 2)?)?);
    assert_eq!(view.task(&task)?.snapshot().status, Status::Failed);
    Ok(())
}

/// T16-HD-27 · gaps the engine reported are preserved, not omitted: a client that drops them
/// presents a more complete picture than it has.
#[test]
fn reported_gaps_are_preserved() -> Outcome {
    let task = id(1);
    let mut view = view(1)?;
    view.present(Snapshot {
        gaps: vec![
            "worker cleanup unread".to_owned(),
            "cost unmeasured".to_owned(),
        ],
        ..snapshot(&task, Status::Unknown, 1)?
    })?;
    assert_eq!(view.task(&task)?.snapshot().gaps.len(), 2);
    Ok(())
}

/// T16-HD-28 · a route explanation is presented verbatim when supplied, and its absence is
/// `None` rather than an invented sentence.
#[test]
fn a_route_explanation_is_verbatim_or_absent() -> Outcome {
    let (a, b) = (id(1), id(2));
    let mut view = view(1)?;
    view.present(Snapshot {
        route_explanation: Some("excluded remote: privacy local_only".to_owned()),
        ..snapshot(&a, Status::Running, 1)?
    })?;
    view.present(snapshot(&b, Status::Running, 1)?)?;
    assert_eq!(
        view.task(&a)?.snapshot().route_explanation.as_deref(),
        Some("excluded remote: privacy local_only")
    );
    assert_eq!(view.task(&b)?.snapshot().route_explanation, None);
    Ok(())
}

/// T16-HD-29 · cancellation presents as pending rather than as a settled outcome, because an
/// obligation that is recorded is not an obligation that is discharged.
#[test]
fn cancellation_presents_as_pending_not_settled() -> Outcome {
    let task = id(1);
    let mut view = view(1)?;
    view.present(snapshot(&task, Status::CancellationPending, 1)?)?;
    let presented = view.task(&task)?.snapshot();
    assert_eq!(presented.status, Status::CancellationPending);
    assert!(!presented.status.is_settled());
    assert_eq!(presented.status.name(), "cancellation-pending");
    Ok(())
}

/// T16-HD-30 · a malformed task in a snapshot is refused, so a view cannot present a task
/// whose identity it could not parse.
#[test]
fn a_malformed_snapshot_task_is_refused() -> Outcome {
    use habitat_engine::contracts::ScalarError;
    let mut view = view(1)?;
    assert_eq!(
        view.present(snapshot("not-a-uuid", Status::Running, 1)?),
        Err(Refusal::MalformedIdentity(ScalarError::InvalidUuid))
    );
    assert_eq!(view.tasks(), 0);
    Ok(())
}

/// T16-HD-31 · an unknown task is refused by name rather than returning an empty snapshot.
#[test]
fn an_unknown_task_is_refused() -> Outcome {
    let view = view(1)?;
    assert_eq!(view.task(&id(9)).map(|_| ()), Err(Refusal::UnknownTask));
    Ok(())
}

/// T16-HD-32 · several tasks are presented independently.
#[test]
fn several_tasks_are_presented_independently() -> Outcome {
    let mut view = view(1)?;
    for index in 1..=4 {
        view.present(snapshot(&id(index), Status::Running, 1)?)?;
    }
    assert_eq!(view.tasks(), 4);
    view.present(snapshot(&id(2), Status::Failed, 2)?)?;
    assert_eq!(view.task(&id(2))?.snapshot().status, Status::Failed);
    assert_eq!(view.task(&id(3))?.snapshot().status, Status::Running);
    Ok(())
}

/// T16-HD-33 · a fresh view holds nothing and sits at genesis.
#[test]
fn a_fresh_view_holds_nothing() -> Outcome {
    let view = view(3)?;
    assert_eq!(view.epoch().as_str(), epoch_text(3));
    assert_eq!(view.cursor(), 0);
    assert_eq!(view.buffered(), 0);
    assert_eq!(view.tasks(), 0);
    assert_eq!(view.reconnects(), 0);
    Ok(())
}

/// T16-HD-34 · every refusal has a distinct name and none is a substring of another.
#[test]
fn refusal_names_are_distinct_and_non_overlapping() {
    use habitat_engine::contracts::ScalarError;
    let all = [
        Refusal::MalformedIdentity(ScalarError::InvalidUuid),
        Refusal::EpochMismatch,
        Refusal::StaleEvent,
        Refusal::BufferFull,
        Refusal::ReconnectLimit,
        Refusal::UnknownTask,
        Refusal::CursorAhead,
        Refusal::IntentConflict,
        Refusal::MalformedSequence(ScalarError::LeadingZero),
        Refusal::ContinuityBroken,
        Refusal::UnknownTaskState,
        Refusal::TaskLimit { limit: 1 },
        Refusal::EvidenceLimit { found: 2, limit: 1 },
        Refusal::EventTooLarge { bytes: 2, limit: 1 },
    ];
    for (i, a) in all.iter().enumerate() {
        assert!(!a.name().is_empty());
        for (j, b) in all.iter().enumerate() {
            if i != j {
                assert!(
                    !a.name().contains(b.name()),
                    "{} contains {}",
                    a.name(),
                    b.name()
                );
            }
        }
    }
}

/// T16-HD-35 · a refusal carrying a scalar error shows both parts.
#[test]
fn refusal_display_shows_the_carried_error() {
    use habitat_engine::contracts::ScalarError;
    assert_eq!(
        Refusal::MalformedIdentity(ScalarError::InvalidUuid).to_string(),
        "malformed client identity: expected a lowercase hyphenated UUIDv4"
    );
    assert_eq!(
        Refusal::StaleEvent.to_string(),
        "event precedes the client cursor"
    );
}

/// T16-HD-36 · the declared bounds are the values enforced.
#[test]
fn declared_bounds_are_the_enforced_bounds() {
    assert_eq!(MAX_BUFFERED_EVENTS, 1024);
    assert_eq!(MAX_RECONNECTS, 1024);
}

// ------------------------------------------- the client owns no task state

/// T16-HD-37 · the view has no method that transitions a task. Presenting a `Failed`
/// snapshot and then a `Passed` one at a higher sequence changes the presentation, and that
/// is the only way a status ever changes here: the engine said so.
#[test]
fn only_the_engine_changes_a_status() -> Outcome {
    let task = id(1);
    let mut view = view(1)?;
    view.present(snapshot(&task, Status::Failed, 1)?)?;
    assert_eq!(view.task(&task)?.snapshot().status, Status::Failed);
    view.present(snapshot(&task, Status::Passed, 2)?)?;
    assert_eq!(view.task(&task)?.snapshot().status, Status::Passed);
    Ok(())
}

/// T16-HD-38 · losing the client does not change any task: reconnecting in-epoch leaves
/// every presented status exactly as the engine last reported it.
#[test]
fn client_loss_leaves_engine_state_untouched() -> Outcome {
    let mut view = view(1)?;
    for index in 1..=3 {
        view.present(snapshot(&id(index), Status::Running, 1)?)?;
    }
    view.reconnect(ep(1)?, 99)?;
    for index in 1..=3 {
        assert_eq!(view.task(&id(index))?.snapshot().status, Status::Running);
    }
    Ok(())
}

/// T16-HD-39 · a view that never observed anything still presents whatever the engine gave
/// it, because presentation and the event stream are independent paths.
#[test]
fn presentation_does_not_depend_on_the_event_stream() -> Outcome {
    let task = id(1);
    let mut view = view(1)?;
    view.present(snapshot(&task, Status::Passed, 9)?)?;
    assert_eq!(view.cursor(), 0, "no events were observed");
    assert_eq!(view.task(&task)?.snapshot().status, Status::Passed);
    Ok(())
}

/// T16-HD-40 · evidence references are carried verbatim for navigation and are never
/// interpreted; an entry that looks like an instruction is just a string.
#[test]
fn evidence_references_are_carried_verbatim() -> Outcome {
    let task = id(1);
    let mut view = view(1)?;
    let refs = vec![
        "evidence/release/a.json".to_owned(),
        "; rm -rf / # not a command, just text".to_owned(),
    ];
    view.present(Snapshot {
        evidence: refs.clone(),
        ..snapshot(&task, Status::Passed, 1)?
    })?;
    assert_eq!(view.task(&task)?.snapshot().evidence, refs);
    Ok(())
}

/// T16-HD-41 · a snapshot with no evidence presents an empty list rather than a placeholder,
/// so "no proof retained" is visibly different from "proof not looked for".
#[test]
fn absent_evidence_is_an_empty_list() -> Outcome {
    let task = id(1);
    let mut view = view(1)?;
    view.present(snapshot(&task, Status::Passed, 1)?)?;
    assert!(view.task(&task)?.snapshot().evidence.is_empty());
    assert!(view.task(&task)?.snapshot().gaps.is_empty());
    Ok(())
}

/// T16-HD-42 · the submission record survives an in-epoch reconnect, so a reconnecting
/// client does not resubmit an intent the engine already admitted.
#[test]
fn the_submission_record_survives_an_in_epoch_reconnect() -> Outcome {
    let task = id(1);
    let key = key(1)?;
    let mut view = view(1)?;
    view.submit(
        &key,
        Admitted::from_engine(EngineReceipt::issue(&task, EPOCH_1, "1")?),
    )?;
    view.reconnect(ep(1)?, 3)?;
    assert_eq!(view.submitted(&key), Some(task.as_str()));
    let again = view.submit(
        &key,
        Admitted::from_engine(EngineReceipt::issue(&task, EPOCH_1, "4")?),
    )?;
    assert!(matches!(again, Admitted::Duplicate(_)));
    Ok(())
}

/// T16-HD-43 · after an epoch change the submission record is gone, so the same intent may
/// legitimately be submitted again into the new world.
#[test]
fn an_epoch_change_releases_the_submission_record() -> Outcome {
    let task = id(1);
    let key = key(1)?;
    let mut view = view(1)?;
    view.submit(
        &key,
        Admitted::from_engine(EngineReceipt::issue(&task, EPOCH_1, "1")?),
    )?;
    view.reconnect(ep(2)?, 0)?;
    let fresh = view.submit(
        &key,
        Admitted::from_engine(EngineReceipt::issue(&task, EPOCH_2, "1")?),
    )?;
    assert!(matches!(fresh, Admitted::Accepted(_)));
    Ok(())
}

/// T16-HD-44 · observing, draining and observing again keeps every event exactly once, in
/// order, across the drain boundary.
#[test]
fn events_survive_a_drain_boundary_exactly_once() -> Outcome {
    let mut view = view(1)?;
    let mut seen = Vec::new();
    for index in 1..=6_u64 {
        view.observe(&ep(1)?, index, index - 1, &format!("e{index}"))?;
        if index % 3 == 0 {
            seen.extend(view.drain());
        }
    }
    let sequences: Vec<u64> = seen.iter().map(|(s, _)| *s).collect();
    assert_eq!(sequences, vec![1, 2, 3, 4, 5, 6]);
    assert_eq!(view.buffered(), 0);
    Ok(())
}

/// T16-HD-45 · a NUMERICAL gap is accepted when the event continues the cursor: `sequence` is
/// the global store sequence and filtering permits gaps, so continuity is `previous_sequence ==
/// cursor`, never `+1` (contract-decisions §4). A gap that breaks continuity is refused by
/// T16-HD-66 (HERDR-G05).
#[test]
fn a_numerical_gap_that_continues_the_cursor_is_accepted() -> Outcome {
    let mut view = view(1)?;
    view.observe(&ep(1)?, 1, 0, "a")?;
    view.observe(&ep(1)?, 50, 1, "later")?;
    assert_eq!(view.cursor(), 50);
    assert_eq!(view.buffered(), 2);
    Ok(())
}

/// T16-HD-46 · reconnecting twice counts twice, so an operator can see a flapping client.
#[test]
fn reconnects_are_counted() -> Outcome {
    let mut view = view(1)?;
    for expected in 1..=5 {
        view.reconnect(ep(1)?, 0)?;
        assert_eq!(view.reconnects(), expected);
    }
    Ok(())
}

/// T16-HD-47 · an admitted submission reports the durable identity the engine chose, not one
/// the client invented.
#[test]
fn the_durable_identity_comes_from_the_engine() -> Outcome {
    let engine_task = id(42);
    let mut view = view(1)?;
    let admitted = view.submit(
        &key(5)?,
        Admitted::from_engine(EngineReceipt::issue(&engine_task, EPOCH_1, "1")?),
    )?;
    let acceptance = admitted.acceptance().ok_or("expected acceptance")?;
    assert_eq!(acceptance.task().as_str(), engine_task);
    Ok(())
}

/// T16-HD-48 · a duplicate carries the acceptance from the current call, so the caller
/// always has a receipt to navigate from even on the duplicate path.
#[test]
fn a_duplicate_still_carries_an_acceptance() -> Outcome {
    let task = id(1);
    let key = key(1)?;
    let mut view = view(1)?;
    view.submit(
        &key,
        Admitted::from_engine(EngineReceipt::issue(&task, EPOCH_1, "1")?),
    )?;
    let duplicate = view.submit(
        &key,
        Admitted::from_engine(EngineReceipt::issue(&task, EPOCH_1, "2")?),
    )?;
    let acceptance = duplicate.acceptance().ok_or("expected acceptance")?;
    assert_eq!(acceptance.task().as_str(), task);
    Ok(())
}

/// T16-HD-49 · a denial submitted under a key that already succeeded returns the denial
/// itself, not a fabricated duplicate: the engine's answer stands.
#[test]
fn a_denial_after_a_success_reports_the_denial() -> Outcome {
    let task = id(1);
    let key = key(1)?;
    let mut view = view(1)?;
    view.submit(
        &key,
        Admitted::from_engine(EngineReceipt::issue(&task, EPOCH_1, "1")?),
    )?;
    let denied = view.submit(&key, Admitted::Denied)?;
    assert!(matches!(denied, Admitted::Denied));
    assert!(!denied.is_durable());
    Ok(())
}

/// T16-HD-50 · status names are stable wire strings, asserted individually so a rename is a
/// visible change rather than a silent one.
#[test]
fn status_names_are_stable() {
    assert_eq!(Status::Running.name(), "running");
    assert_eq!(Status::Passed.name(), "passed");
    assert_eq!(Status::Failed.name(), "failed");
    assert_eq!(Status::CancellationPending.name(), "cancellation-pending");
    assert_eq!(Status::Unknown.name(), "unknown");
}

/// T16-HD-51 · a view attached to a high epoch behaves identically to one at epoch 1, so no
/// behaviour is pinned to the identity element.
#[test]
fn behaviour_does_not_depend_on_the_epoch_value() -> Outcome {
    let task = id(1);
    let mut view = view(9_999)?;
    view.submit(
        &key(1)?,
        Admitted::from_engine(EngineReceipt::issue(&task, EPOCH_9999, "1")?),
    )?;
    view.observe(&ep(9_999)?, 1, 0, "a")?;
    view.present(Snapshot {
        epoch: ep(9_999)?,
        ..snapshot(&task, Status::Running, 1)?
    })?;
    assert_eq!(view.epoch().as_str(), epoch_text(9_999));
    assert_eq!(view.cursor(), 1);
    assert_eq!(view.tasks(), 1);
    assert_eq!(
        view.observe(&ep(1)?, 2, 1, "wrong epoch"),
        Err(Refusal::EpochMismatch)
    );
    Ok(())
}

/// T16-HD-52 · the whole client cycle over two epochs: work in one, restore, and the view
/// carries nothing across that it cannot vouch for while the engine's new state presents
/// normally.
#[test]
fn the_cycle_across_an_epoch_boundary_carries_nothing_stale() -> Outcome {
    let old_task = id(1);
    let new_task = id(2);
    let mut view = view(1)?;
    view.submit(
        &key(1)?,
        Admitted::from_engine(EngineReceipt::issue(&old_task, EPOCH_1, "1")?),
    )?;
    view.observe(&ep(1)?, 1, 0, "old")?;
    view.present(snapshot(&old_task, Status::Passed, 1)?)?;

    view.reconnect(ep(2)?, 0)?;
    assert_eq!(view.tasks(), 0);
    assert_eq!(view.buffered(), 0);
    assert_eq!(view.cursor(), 0);

    view.submit(
        &key(2)?,
        Admitted::from_engine(EngineReceipt::issue(&new_task, EPOCH_2, "1")?),
    )?;
    view.present(Snapshot {
        epoch: ep(2)?,
        ..snapshot(&new_task, Status::Running, 1)?
    })?;
    assert_eq!(view.tasks(), 1);
    assert_eq!(view.task(&new_task)?.snapshot().status, Status::Running);
    assert_eq!(view.task(&old_task).map(|_| ()), Err(Refusal::UnknownTask));
    Ok(())
}

/// T16-HD-53 · review D2: a second, different task under an intent key already submitted is
/// refused, and the earlier outcome stands. `submit` used to discard the earlier outcome and
/// return the CURRENT acceptance labelled `Duplicate` — so the view reported task B as the
/// same submission as task A.
#[test]
fn a_different_task_under_a_submitted_key_is_refused_and_the_first_stands() -> Outcome {
    let (first_task, second_task) = (id(1), id(2));
    let mut view = view(1)?;
    let key = key(7)?;
    view.submit(
        &key,
        Admitted::from_engine(EngineReceipt::issue(&first_task, EPOCH_1, "1")?),
    )?;
    let second = view.submit(
        &key,
        Admitted::from_engine(EngineReceipt::issue(&second_task, EPOCH_1, "2")?),
    );
    assert_eq!(second, Err(Refusal::IntentConflict));
    assert_eq!(view.submitted(&key), Some(first_task.as_str()));
    Ok(())
}

/// T16-HD-54 · review D3: `EngineReceipt` is a view's rendering of what the engine said, not
/// authority. A client can always fabricate a response, so the protection is that no engine
/// module takes a receipt as evidence: the type is named in `src/herdr.rs` and nowhere else
/// under `src/`. Enumerated from the directory, so a new module is in the denominator.
#[test]
fn no_engine_module_takes_an_engine_receipt_as_authority() -> Outcome {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut pending = vec![root.clone()];
    let mut scanned = 0;
    let mut naming = Vec::new();
    while let Some(dir) = pending.pop() {
        for entry in std::fs::read_dir(&dir)? {
            let path = entry?.path();
            if path.is_dir() {
                pending.push(path);
            } else if path.extension().is_some_and(|ext| ext == "rs") {
                scanned += 1;
                if std::fs::read_to_string(&path)?.contains("EngineReceipt") {
                    naming.push(path.strip_prefix(&root)?.display().to_string());
                }
            }
        }
    }
    assert!(scanned > 20, "only {scanned} source files found under src/");
    assert_eq!(naming, ["herdr.rs"], "EngineReceipt named outside the view");
    Ok(())
}

// ------------------------------------------- engine-shaped identities (HERDR-G01)

/// T16-HD-55 · an intent key is the engine's `idempotency_key`, a `UUIDv4`
/// (contract-decisions §3 `idempotency_key: UuidV4`; the engine's readback parses it as one).
/// Free text, upper case and a nil-version UUID are refused before any submission exists.
#[test]
fn an_intent_key_must_be_a_uuidv4() {
    use habitat_engine::contracts::ScalarError;
    for text in [
        "operator-enter-1",
        "",
        "0BADCAFE-1234-4ABC-B123-0123456789AB",
        "0badcafe-1234-1abc-b123-0123456789ab",
    ] {
        assert_eq!(
            IntentKey::new(text),
            Err(Refusal::MalformedIdentity(ScalarError::InvalidUuid)),
            "{text:?} was accepted as an intent key"
        );
    }
}

/// T16-HD-56 · a receipt carries the wire's epoch identity and its decimal sequence, asserted
/// whole over two fixtures that differ in every field.
#[test]
fn a_receipt_carries_the_wire_epoch_and_decimal_sequence() -> Outcome {
    let (task_a, task_b) = (id(3), id(0xbeef));
    let a = EngineReceipt::issue(&task_a, EPOCH_2, "41")?;
    let b = EngineReceipt::issue(&task_b, EPOCH_9999, "18446744073709551615")?;
    assert_eq!(
        (a.task().as_str(), a.epoch().as_str(), a.sequence()),
        (task_a.as_str(), EPOCH_2, 41)
    );
    assert_eq!(
        (b.task().as_str(), b.epoch().as_str(), b.sequence()),
        (task_b.as_str(), EPOCH_9999, u64::MAX)
    );
    Ok(())
}

/// T16-HD-57 · a receipt whose epoch is not a `UUIDv4` or whose sequence is not the wire's
/// canonical decimal is refused, naming which part failed.
#[test]
fn a_malformed_receipt_epoch_or_sequence_is_refused() {
    use habitat_engine::contracts::ScalarError;
    let task = id(1);
    assert_eq!(
        EngineReceipt::issue(&task, "1", "1").map(|_| ()),
        Err(Refusal::MalformedIdentity(ScalarError::InvalidUuid))
    );
    for (sequence, error) in [
        ("", ScalarError::Empty),
        ("-1", ScalarError::InvalidCharacter),
        ("07", ScalarError::LeadingZero),
        ("18446744073709551616", ScalarError::Overflow),
    ] {
        assert_eq!(
            EngineReceipt::issue(&task, EPOCH_1, sequence).map(|_| ()),
            Err(Refusal::MalformedSequence(error)),
            "sequence {sequence:?}"
        );
    }
    assert_eq!(
        Refusal::MalformedSequence(ScalarError::LeadingZero).to_string(),
        "malformed engine sequence: noncanonical leading zero"
    );
}

/// T16-HD-58 · an epoch is the ledger's `UUIDv4` and nothing else; its text round-trips.
#[test]
fn an_epoch_is_a_uuidv4() -> Outcome {
    use habitat_engine::contracts::ScalarError;
    assert_eq!(Epoch::parse(EPOCH_9999)?.as_str(), EPOCH_9999);
    for text in ["1", "", "0000270F-EEEE-4EEE-9EEE-EEEEEEEEEEEE"] {
        assert_eq!(
            Epoch::parse(text),
            Err(Refusal::MalformedIdentity(ScalarError::InvalidUuid)),
            "{text:?}"
        );
    }
    assert_ne!(ep(1)?, ep(2)?);
    Ok(())
}

// ------------------------------------------- engine task state -> presented status (HERDR-G02)

/// The engine's `TaskStateV1` vocabulary as the published wire contract declares it — read from
/// `docs/contract-decisions.md`, not typed here from the same reading that wrote the mapping.
fn published_task_states() -> Result<Vec<String>, Box<dyn Error>> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("docs/contract-decisions.md");
    let text = std::fs::read_to_string(path)?;
    let marker = "`TaskStateV1` is exactly `";
    let start = text
        .find(marker)
        .ok_or("TaskStateV1 declaration not found")?
        + marker.len();
    let rest = text.get(start..).ok_or("declaration truncated")?;
    let end = rest.find('`').ok_or("declaration unterminated")?;
    let list = rest.get(..end).ok_or("declaration truncated")?;
    Ok(list.split(" | ").map(str::to_owned).collect())
}

/// T16-HD-59 · the mapping is total over the engine's vocabulary. The denominator is the
/// published `TaskStateV1` list, cross-checked against the ledger's compiled parser, and each
/// spelling's presentation is asserted against a table stated here.
#[test]
fn every_engine_task_state_has_a_truthful_status() -> Result<(), Box<dyn Error>> {
    use habitat_engine::recovery::TaskState;
    let expected = [
        ("admitted", Status::Running),
        ("queued", Status::Running),
        ("running", Status::Running),
        ("verifying", Status::Running),
        ("repair_pending", Status::Running),
        ("cancellation_requested", Status::CancellationPending),
        ("blocked", Status::Blocked),
        ("accepted", Status::Passed),
        ("failed", Status::Failed),
        ("cancelled", Status::Cancelled),
        ("abandoned", Status::Abandoned),
        ("effect_unknown", Status::EffectUnknown),
    ];
    let published = published_task_states()?;
    assert_eq!(published.len(), 12, "published vocabulary: {published:?}");
    let table: Vec<&str> = expected.iter().map(|(state, _)| *state).collect();
    assert_eq!(
        published, table,
        "the table must cover the published list, in order"
    );
    for (state, status) in expected {
        let ledger = TaskState::parse(state).ok_or("the ledger does not spell this state")?;
        assert_eq!(ledger.name(), state);
        assert_eq!(Status::from_engine(state), Ok(status), "{state}");
    }
    Ok(())
}

/// T16-HD-60 · a spelling outside the vocabulary is refused rather than defaulted — including
/// the client's own word `passed`, which the engine never sends.
#[test]
fn an_unknown_engine_state_is_refused() {
    for text in [
        "passed",
        "Accepted",
        "accepted ",
        "",
        "unknown",
        "cancellation-pending",
    ] {
        assert_eq!(
            Status::from_engine(text),
            Err(Refusal::UnknownTaskState),
            "{text:?}"
        );
    }
}

/// T16-HD-61 · each new status has its own stable name, and the engine's terminal
/// `cancelled` and `abandoned` present as settled while `effect_unknown` and `blocked` do not.
#[test]
fn terminal_states_settle_and_unknown_effects_do_not() -> Outcome {
    for (state, name, settled) in [
        ("cancelled", "cancelled", true),
        ("abandoned", "abandoned", true),
        ("blocked", "blocked", false),
        ("effect_unknown", "effect-unknown", false),
    ] {
        let status = Status::from_engine(state)?;
        assert_eq!(
            (status.name(), status.is_settled()),
            (name, settled),
            "{state}"
        );
    }
    Ok(())
}

// ------------------------------------------- stale state is first-class (HERDR-G06)

/// T16-HD-62 · a late snapshot from the old epoch after a reconnect into a new one is refused:
/// it describes a world that no longer exists and must not present as current.
#[test]
fn an_old_epoch_snapshot_after_reconnect_is_refused() -> Outcome {
    let task = id(1);
    let mut view = view(1)?;
    view.reconnect(ep(2)?, 0)?;
    assert_eq!(
        view.present(snapshot(&task, Status::Passed, 9)?),
        Err(Refusal::EpochMismatch)
    );
    assert_eq!(view.tasks(), 0);
    Ok(())
}

/// T16-HD-63 · after an in-epoch reconnect a held snapshot stays stale until the engine
/// presents one read at or after the resynchronisation cursor: a newer snapshot read BEFORE
/// that cursor is applied but is still stale.
#[test]
fn a_stale_snapshot_is_cleared_only_from_the_resync_cursor() -> Outcome {
    let task = id(4);
    let mut view = view(1)?;
    view.present(snapshot(&task, Status::Running, 3)?)?;
    view.reconnect(ep(1)?, 10)?;
    assert!(view.present(snapshot(&task, Status::Running, 5)?)?);
    let presented = view.task(&task)?;
    assert_eq!(
        (presented.snapshot().sequence, presented.freshness()),
        (5, Freshness::Stale)
    );
    assert!(view.present(snapshot(&task, Status::Failed, 10)?)?);
    let presented = view.task(&task)?;
    assert_eq!(
        (presented.snapshot().status, presented.freshness()),
        (Status::Failed, Freshness::Current)
    );
    Ok(())
}

/// T16-HD-64 · a stale task re-presented at its held sequence from the resynchronisation cursor
/// is current again: nothing changed in the ledger, and the engine has now said so. A current
/// task at the same sequence is still ignored (T16-HD-25).
#[test]
fn a_stale_task_reconfirmed_at_its_sequence_is_current() -> Outcome {
    let task = id(6);
    let mut view = view(1)?;
    view.present(snapshot(&task, Status::Running, 8)?)?;
    view.reconnect(ep(1)?, 8)?;
    assert_eq!(view.task(&task)?.freshness(), Freshness::Stale);
    assert!(view.present(snapshot(&task, Status::Running, 8)?)?);
    assert_eq!(view.task(&task)?.freshness(), Freshness::Current);
    assert!(!view.present(snapshot(&task, Status::Running, 8)?)?);
    Ok(())
}

/// T16-HD-65 · a snapshot of a task the view first sees after a reconnect, read before the
/// resynchronisation cursor, is stale on arrival.
#[test]
fn a_new_snapshot_read_before_the_resync_cursor_is_stale() -> Outcome {
    let task = id(7);
    let mut view = view(1)?;
    view.reconnect(ep(1)?, 20)?;
    view.present(snapshot(&task, Status::Running, 19)?)?;
    assert_eq!(view.task(&task)?.freshness(), Freshness::Stale);
    Ok(())
}

// ------------------------------------------- event continuity (HERDR-G05)

/// T16-HD-66 · an event whose `previous_sequence` is not the client's cursor is refused: the
/// subscription delivered something the client never saw, so it must resynchronise rather
/// than present a history with a hole in it. Nothing is buffered and the cursor stays.
#[test]
fn an_event_that_does_not_continue_the_cursor_is_refused() -> Outcome {
    let mut view = view(1)?;
    view.observe(&ep(1)?, 3, 0, "a")?;
    assert_eq!(
        view.observe(&ep(1)?, 9, 5, "after a lost event"),
        Err(Refusal::ContinuityBroken)
    );
    assert_eq!(
        view.observe(&ep(1)?, 9, 2, "previous behind the cursor"),
        Err(Refusal::ContinuityBroken)
    );
    assert_eq!((view.cursor(), view.buffered()), (3, 1));
    Ok(())
}

/// T16-HD-67 · after a reconnect the first event continues from the engine's cursor (the
/// snapshot high-water), and continuity is enforced from there.
#[test]
fn continuity_resumes_from_the_reconnect_cursor() -> Outcome {
    let mut view = view(1)?;
    view.observe(&ep(1)?, 2, 0, "a")?;
    view.reconnect(ep(1)?, 40)?;
    assert_eq!(
        view.observe(&ep(1)?, 45, 2, "continues the pre-loss cursor"),
        Err(Refusal::ContinuityBroken)
    );
    view.observe(&ep(1)?, 45, 40, "first after resync")?;
    assert_eq!(view.cursor(), 45);
    Ok(())
}

// ------------------------------------------- display failure (HERDR-G09)

fn rendered(view: &View, task: &str) -> Result<String, Box<dyn Error>> {
    let mut out = String::new();
    render(view.task(task)?, &mut out)?;
    Ok(out)
}

/// T16-HD-68 · the renderer is asserted whole over two fixtures that differ in every field:
/// status and settledness, freshness, epoch, sequence, route, evidence and gaps. The two
/// fixtures live in two views attached to two different epochs, so a renderer that writes
/// a constant epoch (or sequence) cannot match both.
#[test]
fn render_is_whole_over_two_fixtures_differing_in_every_field() -> Result<(), Box<dyn Error>> {
    let (a, b) = (id(0xa), id(0xb));
    let mut other = view(3)?;
    let mut view = view(2)?;
    view.present(Snapshot {
        task: a.clone(),
        epoch: ep(2)?,
        status: Status::Failed,
        route_explanation: Some("excluded remote: privacy local_only".to_owned()),
        evidence: vec!["evidence/a.json".to_owned(), "evidence/b.json".to_owned()],
        gaps: vec!["cost unmeasured".to_owned()],
        sequence: 17,
    })?;
    other.present(Snapshot {
        task: b.clone(),
        epoch: ep(3)?,
        status: Status::EffectUnknown,
        route_explanation: None,
        evidence: Vec::new(),
        gaps: Vec::new(),
        sequence: 4,
    })?;
    other.reconnect(ep(3)?, 9)?;
    view.reconnect(ep(2)?, 17)?;
    view.present(Snapshot {
        task: a.clone(),
        epoch: ep(2)?,
        status: Status::Failed,
        route_explanation: Some("excluded remote: privacy local_only".to_owned()),
        evidence: vec!["evidence/a.json".to_owned(), "evidence/b.json".to_owned()],
        gaps: vec!["cost unmeasured".to_owned()],
        sequence: 17,
    })?;
    assert_eq!(
        rendered(&view, &a)?,
        format!(
            "task {a}\nstatus failed (settled)\nfreshness current\nas of epoch {EPOCH_2} sequence 17\n\
             route \"excluded remote: privacy local_only\"\nevidence \"evidence/a.json\"\n\
             evidence \"evidence/b.json\"\ngap \"cost unmeasured\"\n"
        )
    );
    assert_eq!(
        rendered(&other, &b)?,
        format!(
            "task {b}\nstatus effect-unknown (not settled)\n\
             freshness stale: held from before the last reconnect\n\
             as of epoch {EPOCH_3} sequence 4\nroute none supplied\nevidence none retained\n"
        )
    );
    Ok(())
}

/// A writer that accepts `budget` bytes and then fails, recording what it was given.
struct FailingWriter {
    budget: usize,
    written: String,
}

impl std::fmt::Write for FailingWriter {
    fn write_str(&mut self, text: &str) -> std::fmt::Result {
        if self.written.len() + text.len() > self.budget {
            return Err(std::fmt::Error);
        }
        self.written.push_str(text);
        Ok(())
    }
}

/// A writer whose `fail_at`-th write fails once; every other write succeeds and is recorded.
struct FlakyWriter {
    fail_at: usize,
    calls: usize,
    written: String,
}

impl std::fmt::Write for FlakyWriter {
    fn write_str(&mut self, text: &str) -> std::fmt::Result {
        self.calls += 1;
        if self.calls == self.fail_at {
            return Err(std::fmt::Error);
        }
        self.written.push_str(text);
        Ok(())
    }
}

/// T16-HD-69 · a display failure invents nothing: the render reports the failure, the view's
/// presentation is unchanged, and the next render presents the same state in full. Every cut
/// point through the output is tried, so no line boundary is special; and a writer that fails
/// ONCE and then recovers still yields an error, never an `Ok` card with a line missing.
#[test]
fn a_failed_render_leaves_the_view_unchanged() -> Result<(), Box<dyn Error>> {
    let task = id(0x51);
    let mut view = view(1)?;
    view.present(Snapshot {
        route_explanation: Some("local adapter".to_owned()),
        evidence: vec!["evidence/release/T16.json".to_owned()],
        ..snapshot(&task, Status::Running, 3)?
    })?;
    let before = rendered(&view, &task)?;
    for budget in 0..before.len() {
        let mut writer = FailingWriter {
            budget,
            written: String::new(),
        };
        assert!(
            render(view.task(&task)?, &mut writer).is_err(),
            "budget {budget}"
        );
        assert!(before.starts_with(&writer.written), "budget {budget}");
        let presented = view.task(&task)?;
        assert_eq!(
            (presented.snapshot().status, presented.freshness()),
            (Status::Running, Freshness::Current)
        );
        assert_eq!(rendered(&view, &task)?, before, "budget {budget}");
    }
    let mut calls = 0;
    for fail_at in 1..=before.len() {
        let mut writer = FlakyWriter {
            fail_at,
            calls: 0,
            written: String::new(),
        };
        let result = render(view.task(&task)?, &mut writer);
        if writer.calls < fail_at {
            break;
        }
        calls = fail_at;
        assert!(
            result.is_err(),
            "write {fail_at} failed and render returned Ok"
        );
        assert!(before.starts_with(&writer.written), "write {fail_at}");
    }
    assert!(calls > 7, "only {calls} writes were exercised");
    assert_eq!(rendered(&view, &task)?, before);
    Ok(())
}

/// T16-HD-70 · engine-supplied text cannot forge a line: a gap, route or evidence reference
/// carrying a newline and a fake settled status renders escaped on its own line, so the only
/// `status` line is the one computed from the engine's status.
#[test]
fn engine_text_cannot_forge_a_rendered_status() -> Result<(), Box<dyn Error>> {
    let task = id(0x70);
    let forged = "x\nstatus passed (settled)\nfreshness current";
    let mut view = view(1)?;
    view.present(Snapshot {
        route_explanation: Some(forged.to_owned()),
        evidence: vec![forged.to_owned()],
        gaps: vec![forged.to_owned()],
        ..snapshot(&task, Status::Unknown, 2)?
    })?;
    let out = rendered(&view, &task)?;
    let status_lines: Vec<&str> = out
        .lines()
        .filter(|line| line.starts_with("status"))
        .collect();
    assert_eq!(status_lines, ["status unknown (not settled)"]);
    assert_eq!(out.lines().count(), 7, "{out}");
    assert!(out.contains("gap \"x\\nstatus passed (settled)\\nfreshness current\""));
    Ok(())
}

/// T16-HD-71 · a stale accepted task never renders as a current verdict: its freshness line
/// says stale, even though its status is settled.
#[test]
fn a_stale_passed_task_renders_as_stale() -> Result<(), Box<dyn Error>> {
    let task = id(0x71);
    let mut view = view(1)?;
    view.present(snapshot(&task, Status::from_engine("accepted")?, 5)?)?;
    view.reconnect(ep(1)?, 6)?;
    let out = rendered(&view, &task)?;
    let lines: Vec<&str> = out.lines().take(3).collect();
    assert_eq!(
        lines,
        [
            format!("task {task}").as_str(),
            "status passed (settled)",
            "freshness stale: held from before the last reconnect",
        ]
    );
    Ok(())
}

// ------------------------------------------- acquisition bounds (review §5 item 2)

/// T16-HD-72 · a view presents at most [`MAX_TASKS`] tasks: the last one is admitted, the next
/// distinct one is refused with the bound and nothing is stored; an already-presented task is
/// still updated at the bound.
#[test]
fn the_task_bound_refuses_a_new_task_before_storing() -> Outcome {
    let mut view = view(1)?;
    for index in 0..MAX_TASKS {
        assert!(view.present(snapshot(&id(index), Status::Running, 1)?)?);
    }
    assert_eq!(view.tasks(), MAX_TASKS);
    assert_eq!(
        view.present(snapshot(&id(MAX_TASKS), Status::Running, 1)?),
        Err(Refusal::TaskLimit { limit: MAX_TASKS })
    );
    assert_eq!(view.tasks(), MAX_TASKS);
    assert!(view.present(snapshot(&id(0), Status::Running, 2)?)?);
    Ok(())
}

/// T16-HD-73 · the submissions a view remembers are bounded the same way: one per task.
#[test]
fn the_submission_bound_refuses_a_new_intent_before_storing() -> Outcome {
    let mut view = view(1)?;
    for index in 0..MAX_TASKS {
        let task = id(index);
        view.submit(
            &key(index)?,
            Admitted::from_engine(EngineReceipt::issue(&task, EPOCH_1, "1")?),
        )?;
    }
    let task = id(MAX_TASKS);
    assert_eq!(
        view.submit(
            &key(MAX_TASKS)?,
            Admitted::from_engine(EngineReceipt::issue(&task, EPOCH_1, "1")?),
        )
        .map(|_| ()),
        Err(Refusal::TaskLimit { limit: MAX_TASKS })
    );
    assert_eq!(view.submitted(&key(MAX_TASKS)?), None);
    Ok(())
}

/// T16-HD-74 · a snapshot with more evidence references than the engine serves is refused whole,
/// with both numbers; at the bound it is presented.
#[test]
fn the_evidence_bound_refuses_the_whole_snapshot() -> Outcome {
    let mut view = view(1)?;
    let refs = |count: usize| {
        (0..count)
            .map(|index| format!("ref-{index}"))
            .collect::<Vec<_>>()
    };
    let mut full = snapshot(&id(1), Status::Running, 1)?;
    full.evidence = refs(MAX_EVIDENCE_REFS);
    assert!(view.present(full)?);
    let mut over = snapshot(&id(2), Status::Running, 1)?;
    over.evidence = refs(MAX_EVIDENCE_REFS + 1);
    assert_eq!(
        view.present(over),
        Err(Refusal::EvidenceLimit {
            found: MAX_EVIDENCE_REFS + 1,
            limit: MAX_EVIDENCE_REFS
        })
    );
    assert_eq!(view.task(&id(2)).map(|_| ()), Err(Refusal::UnknownTask));
    Ok(())
}

/// T16-HD-75 · an event text larger than any frame is refused before it is copied, and the cursor
/// does not move; at the bound it is stored.
#[test]
fn the_event_text_bound_refuses_before_copying() -> Outcome {
    let mut view = view(1)?;
    let epoch = ep(1)?;
    view.observe(&epoch, 1, 0, &"x".repeat(MAX_EVENT_TEXT_BYTES))?;
    assert_eq!(
        view.observe(&epoch, 2, 1, &"x".repeat(MAX_EVENT_TEXT_BYTES + 1)),
        Err(Refusal::EventTooLarge {
            bytes: MAX_EVENT_TEXT_BYTES + 1,
            limit: MAX_EVENT_TEXT_BYTES
        })
    );
    assert_eq!((view.cursor(), view.buffered()), (1, 1));
    Ok(())
}

/// T16-HD-76 · each list bound is the engine's published one, read from the schemas the engine serves
/// (an independent source: the generated contract, not this module's constants).
#[test]
fn the_view_bounds_are_the_published_wire_bounds() -> Outcome {
    let read = |name: &str| -> Result<serde_json::Value, Box<dyn Error>> {
        Ok(serde_json::from_str(&std::fs::read_to_string(format!(
            "schemas/actions/control-v1.result.{name}.schema.json"
        ))?)?)
    };
    let subscribe = read("events.subscribe")?;
    let get = read("task.get")?;
    assert_eq!(
        subscribe["$defs"]["BodyResult_events_subscribe"]["properties"]["snapshot"]["maxItems"],
        serde_json::json!(MAX_TASKS)
    );
    assert_eq!(
        get["$defs"]["BodyResult_task_get"]["properties"]["evidence"]["maxItems"],
        serde_json::json!(MAX_EVIDENCE_REFS)
    );
    // MAX_EVENT_TEXT_BYTES is the frame bound by construction; no schema declares a text bound, so
    // there is no independent value to pin it against (review of 2f44216, gap 3) — the decoder
    // (C08a) owns the tight bound.
    Ok(())
}
