//! The whitelist, from an independent literal table (State map §2c plus the brief's Observe and
//! Recover rows), checked against `transition` over every (state, event) pair.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use hee4_contracts::{
    Event, Phase, Reason, RecoveryRule, Refusal, Resolution, Settlement, TaskState, Verdict,
    transition,
};

use Event::{Accept, Admit, Cancel, Dispatch, Observe, Stop};
use Phase::{
    Abandoned, Accepted, Admitted, Blocked, CancellationRequested as Cr, Cancelled, EffectUnknown,
    Failed, RepairPending as Rp, Running, Verifying,
};

const B0: Phase = Blocked { cancel: false };
const B1: Phase = Blocked { cancel: true };
const E0: Phase = EffectUnknown { cancel: false };
const E1: Phase = EffectUnknown { cancel: true };
const READY: Event = Event::Settle(Settlement::Ready);
const NOT_READY: Event = Event::Settle(Settlement::NotReady);
const UNSETTLED: Event = Event::Settle(Settlement::Unsettled);
const PASS: Event = Event::Decide(Verdict::Pass);
const FAIL: Event = Event::Decide(Verdict::Fail);
const fn refused(r: Reason) -> Event {
    Event::Decide(Verdict::Refused(r))
}
const INVALID: Event = refused(Reason::Invalid);
const ERROR: Event = refused(Reason::Error);
const TIMEOUT: Event = refused(Reason::Timeout);
const CANCELLED_V: Event = refused(Reason::Cancelled);
const UNRECONCILED: Event = refused(Reason::Unreconciled);
const RQ: Event = Event::Resolve(Resolution::Quarantine);
const RA: Event = Event::Resolve(Resolution::Abandon);
const R07: Event = Event::Recover(RecoveryRule::R07ProcessNotOurs);
const R08: Event = Event::Recover(RecoveryRule::R08WorkerAbsent);

/// The 64 legal pairs from an existing task (the map's 50, with `blocked` and `effect_unknown`
/// split by their cancel field, plus Observe and Recover R07/R08). `None + Admit` is separate.
const WHITELIST: &[(Phase, Event, Phase)] = &[
    (Admitted, Dispatch, Running),
    (Admitted, Cancel, Cr),
    (Admitted, RQ, B0),
    (Admitted, RA, Abandoned),
    (Admitted, Stop, Failed),
    (Running, READY, Verifying),
    (Running, NOT_READY, Rp),
    (Running, UNSETTLED, E0),
    (Running, Cancel, Cr),
    (Running, RQ, B0),
    (Running, RA, Abandoned),
    (Running, R07, E0),
    (Running, R08, E0),
    (Verifying, Observe, Verifying),
    (Verifying, PASS, Verifying),
    (Verifying, FAIL, Rp),
    (Verifying, INVALID, Failed),
    (Verifying, ERROR, Failed),
    (Verifying, TIMEOUT, Failed),
    (Verifying, UNRECONCILED, E0),
    (Verifying, Accept, Accepted),
    (Verifying, Cancel, Cr),
    (Verifying, RQ, B0),
    (Verifying, RA, Abandoned),
    (Verifying, Stop, Failed),
    (Rp, Dispatch, Running),
    (Rp, Cancel, Cr),
    (Rp, RQ, B0),
    (Rp, RA, Abandoned),
    (Rp, Stop, Failed),
    (Cr, READY, Cr),
    (Cr, NOT_READY, Cr),
    (Cr, UNSETTLED, E1),
    (Cr, PASS, Cr),
    (Cr, FAIL, Cr),
    (Cr, INVALID, Cr),
    (Cr, ERROR, Cr),
    (Cr, TIMEOUT, Cr),
    (Cr, CANCELLED_V, Cr),
    (Cr, UNRECONCILED, E1),
    (Cr, Cancel, Cr),
    (Cr, RQ, B1),
    (Cr, RA, Cancelled),
    (Cr, Stop, Cancelled),
    (Cr, R07, E1),
    (Cr, R08, E1),
    (B0, READY, B0),
    (B0, NOT_READY, B0),
    (B0, UNSETTLED, B0),
    (B0, Cancel, B1),
    (B0, RQ, B0),
    (B0, RA, Abandoned),
    (B1, READY, B1),
    (B1, NOT_READY, B1),
    (B1, UNSETTLED, B1),
    (B1, Cancel, B1),
    (B1, RQ, B1),
    (B1, RA, Cancelled),
    (E0, Cancel, E1),
    (E0, RQ, B0),
    (E0, RA, Abandoned),
    (E1, Cancel, E1),
    (E1, RQ, B1),
    (E1, RA, Cancelled),
];

const ALL_PHASES: [Phase; 13] = [
    Admitted, Running, Verifying, Rp, Cr, B0, B1, E0, E1, Accepted, Failed, Cancelled, Abandoned,
];

fn all_events() -> Vec<Event> {
    let mut v = vec![
        Admit, Dispatch, Observe, READY, NOT_READY, UNSETTLED, PASS, FAIL,
    ];
    v.extend(Reason::ALL.map(refused));
    v.extend([Accept, Cancel, RQ, RA, Stop]);
    v.extend(RecoveryRule::ALL.map(Event::Recover));
    v
}

/// Reach `target` through the public API only (no other way to hold a `TaskState`).
fn reach(target: Phase) -> TaskState {
    let path: &[Event] = match target {
        Admitted => &[Admit],
        Running => &[Admit, Dispatch],
        Verifying => &[Admit, Dispatch, READY],
        Rp => &[Admit, Dispatch, NOT_READY],
        Cr => &[Admit, Cancel],
        B0 => &[Admit, RQ],
        B1 => &[Admit, Cancel, RQ],
        E0 => &[Admit, Dispatch, UNSETTLED],
        E1 => &[Admit, Cancel, UNSETTLED],
        Accepted => &[Admit, Dispatch, READY, PASS, Accept],
        Failed => &[Admit, Stop],
        Cancelled => &[Admit, Cancel, Stop],
        Abandoned => &[Admit, RA],
    };
    let state = TaskState::replay(path.iter().copied()).expect("path is legal");
    assert_eq!(state.phase(), target);
    state
}

#[test]
fn every_pair_matches_the_whitelist_and_every_other_pair_is_refused() {
    let events = all_events();
    assert_eq!(events.len(), 32);
    let (mut legal, mut refused_n) = (0, 0);
    for event in &events {
        let got = transition(None, *event);
        if *event == Admit {
            assert_eq!(got.map(TaskState::phase), Ok(Admitted));
            legal += 1;
        } else {
            assert_eq!(got, Err(Refusal::NotAdmitted { event: *event }));
            refused_n += 1;
        }
    }
    for from in &ALL_PHASES {
        let state = reach(*from);
        for event in &events {
            let want = WHITELIST
                .iter()
                .find(|(f, e, _)| f == from && e == event)
                .map(|(_, _, to)| *to);
            match (transition(Some(state), *event), want) {
                (Ok(to), Some(w)) => {
                    assert_eq!(to.phase(), w, "{from:?} + {event:?}");
                    legal += 1;
                }
                (Err(_), None) => refused_n += 1,
                (got, want) => panic!("{from:?} + {event:?}: got {got:?}, want {want:?}"),
            }
        }
    }
    println!("legal={legal}/65 illegal={refused_n}/{}", 14 * 32 - 65);
    assert_eq!((legal, refused_n), (65, 14 * 32 - 65));
}

#[test]
fn refusals_carry_their_names() {
    let accepted = reach(Accepted);
    assert_eq!(
        transition(Some(accepted), Cancel),
        Err(Refusal::Terminal {
            from: Accepted,
            event: Cancel
        })
    );
    assert_eq!(
        transition(Some(reach(Running)), Admit),
        Err(Refusal::AlreadyAdmitted { from: Running })
    );
    assert_eq!(
        transition(Some(reach(Cr)), Accept),
        Err(Refusal::Illegal {
            from: Cr,
            event: Accept
        })
    );
    assert_eq!(
        transition(
            Some(reach(E0)),
            Event::Recover(RecoveryRule::R10EffectAmbiguity)
        ),
        Err(Refusal::NoTaskEdge {
            from: E0,
            rule: RecoveryRule::R10EffectAmbiguity
        })
    );
    assert_eq!(
        transition(Some(reach(Verifying)), CANCELLED_V),
        Err(Refusal::Illegal {
            from: Verifying,
            event: CANCELLED_V
        })
    );
}

#[test]
fn durable_spelling_has_no_queued() {
    let spellings: Vec<_> = ALL_PHASES.iter().map(|p| p.as_str()).collect();
    assert!(!spellings.contains(&"queued"));
    assert_eq!(
        serde_json::to_string(&reach(E1)).unwrap(),
        r#"{"state":"effect_unknown","cancel":true}"#
    );
}
