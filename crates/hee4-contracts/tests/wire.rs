//! The durable spelling of `Event` round-trips, reasons are data, and `Outcome::Refused` is strict.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use hee4_contracts::{
    AbandonReason, Event, Outcome, Phase, QuarantineReason, Reason, RecoveryRule, Resolution,
    Settlement, TaskState, Verdict, transition,
};

const ABANDONS: [AbandonReason; 8] = [
    AbandonReason::BriefUnreadable,
    AbandonReason::RouteRefused { floor_unmet: true },
    AbandonReason::RouteRefused { floor_unmet: false },
    AbandonReason::NamespaceRefused,
    AbandonReason::WorkDirUnavailable,
    AbandonReason::HeadUnknown,
    AbandonReason::NoPermit,
    AbandonReason::AttemptFailed,
];

fn quarantines() -> Vec<QuarantineReason> {
    RecoveryRule::ALL
        .map(|rule| QuarantineReason::EffectUnknownPermanent { rule })
        .to_vec()
}

fn thirty_two() -> Vec<Event> {
    let mut v = vec![Event::Admit, Event::Dispatch, Event::Observe];
    v.extend(
        [
            Settlement::Ready,
            Settlement::NotReady,
            Settlement::Unsettled,
        ]
        .map(Event::Settle),
    );
    v.extend([Verdict::Pass, Verdict::Fail].map(Event::Decide));
    v.extend(Reason::ALL.map(|r| Event::Decide(Verdict::Refused(r))));
    v.extend([Event::Accept, Event::Cancel, Event::Stop]);
    v.push(Event::Resolve(Resolution::Quarantine(quarantines()[9])));
    v.push(Event::Resolve(Resolution::Abandon(ABANDONS[5])));
    v.extend(RecoveryRule::ALL.map(Event::Recover));
    v
}

#[test]
fn all_32_events_round_trip_through_json() {
    let events = thirty_two();
    assert_eq!(events.len(), 32);
    let mut spellings = std::collections::HashSet::new();
    for e in &events {
        let text = serde_json::to_string(e).unwrap();
        assert_eq!(serde_json::from_str::<Event>(&text).unwrap(), *e, "{text}");
        assert!(spellings.insert(text), "two events share a spelling");
    }
}

#[test]
fn every_reason_round_trips_and_is_data_not_a_string() {
    for a in ABANDONS {
        let e = Event::Resolve(Resolution::Abandon(a));
        let v = serde_json::to_value(e).unwrap();
        assert_eq!(serde_json::from_value::<Event>(v).unwrap(), e);
    }
    for q in quarantines() {
        let e = Event::Resolve(Resolution::Quarantine(q));
        let v = serde_json::to_value(e).unwrap();
        assert_eq!(serde_json::from_value::<Event>(v).unwrap(), e);
    }
    assert_eq!(
        serde_json::to_string(&Event::Resolve(Resolution::Abandon(
            AbandonReason::RouteRefused { floor_unmet: true }
        )))
        .unwrap(),
        r#"{"resolve":{"abandon":{"route_refused":{"floor_unmet":true}}}}"#
    );
    assert_eq!(
        serde_json::to_string(&Event::Resolve(Resolution::Quarantine(
            QuarantineReason::EffectUnknownPermanent {
                rule: RecoveryRule::R10EffectAmbiguity
            }
        )))
        .unwrap(),
        r#"{"resolve":{"quarantine":{"effect_unknown_permanent":{"rule":"R10EffectAmbiguity"}}}}"#
    );
}

#[test]
fn unknown_spellings_do_not_decode() {
    for bad in [
        r#""resolve""#,
        r#"{"resolve":{"abandon":"no_such_reason"}}"#,
        r#"{"resolve":{"abandon":"head unknown"}}"#,
        r#"{"resolve":"abandon"}"#,
        r#"{"resolve":{"quarantine":{"effect_unknown_permanent":{"rule":"R99"}}}}"#,
    ] {
        assert!(serde_json::from_str::<Event>(bad).is_err(), "{bad}");
    }
}

#[test]
fn the_reason_never_changes_the_edge() {
    let start = TaskState::replay([Event::Admit, Event::Dispatch]).unwrap();
    let to = |e| transition(Some(start), e).map(TaskState::phase);
    for a in ABANDONS {
        assert_eq!(
            to(Event::Resolve(Resolution::Abandon(a))),
            Ok(Phase::Abandoned)
        );
    }
    for q in quarantines() {
        assert_eq!(
            to(Event::Resolve(Resolution::Quarantine(q))),
            Ok(Phase::Blocked { cancel: false })
        );
    }
}

#[test]
fn outcome_refused_round_trips_and_denies_unknown_fields() {
    let o: Outcome = serde_json::from_str(r#"{"refused":{"reason":"adapter exit 7"}}"#).unwrap();
    assert!(matches!(&o, Outcome::Refused { reason } if reason.as_str() == "adapter exit 7"));
    assert_eq!(
        serde_json::to_string(&o).unwrap(),
        r#"{"refused":{"reason":"adapter exit 7"}}"#
    );
    for bad in [
        r#"{"refused":{"reason":"x","extra":1}}"#,
        r#"{"refused":{}}"#,
        r#"{"refused":{"reason":""}}"#,
        r#"{"refused":{"reason":"a\nb"}}"#,
        r#""refused""#,
    ] {
        assert!(serde_json::from_str::<Outcome>(bad).is_err(), "{bad}");
    }
    for plain in ["pass", "fail", "error"] {
        assert!(serde_json::from_str::<Outcome>(&format!("\"{plain}\"")).is_ok());
    }
}
