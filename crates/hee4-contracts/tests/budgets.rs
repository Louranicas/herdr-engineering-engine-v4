//! Budgets: defaults, partial override, every field refused by name at zero and over its
//! ceiling, floors, order rules, the render line and the `Duration` accessors.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::BTreeMap;
use std::time::Duration;

use hee4_contracts::{BudgetParseError, BudgetRefusal, Budgets};
use serde_json::{Value, json};

/// `section.field` -> value, flattened from serde's view of the DEFAULT.
fn flat_default() -> BTreeMap<String, u64> {
    let value = serde_json::to_value(Budgets::DEFAULT).unwrap();
    let mut out = BTreeMap::new();
    for (section, fields) in value.as_object().unwrap() {
        for (name, v) in fields.as_object().unwrap() {
            out.insert(format!("{section}.{name}"), v.as_u64().unwrap());
        }
    }
    out
}

/// The DEFAULT as JSON with one dotted field set to `v`.
fn with_field(key: &str, v: Value) -> String {
    let (section, name) = key.split_once('.').unwrap();
    let mut value = serde_json::to_value(Budgets::DEFAULT).unwrap();
    value[section][name] = v;
    value.to_string()
}

fn refused(text: &str) -> BudgetRefusal {
    match Budgets::parse(text) {
        Err(BudgetParseError::Refused(r)) => r,
        other => panic!("expected a typed refusal for {text}, got {other:?}"),
    }
}

#[test]
fn default_validates_and_empty_object_is_default() {
    assert_eq!(Budgets::DEFAULT.validate(), Ok(()));
    assert_eq!(Budgets::parse("{}").unwrap(), Budgets::DEFAULT);
    assert_eq!(Budgets::default(), Budgets::DEFAULT);
}

#[test]
fn a_partial_file_keeps_every_other_default() {
    let b = Budgets::parse(r#"{"door":{"max_requests":3}}"#).unwrap();
    assert_eq!(b.door.max_requests, 3);
    let mut expected = Budgets::DEFAULT;
    expected.door.max_requests = 3;
    assert_eq!(b, expected);
    assert_eq!(b.socket, Budgets::DEFAULT.socket);
    assert_eq!(b.door.max_body_bytes, 1_048_576);
}

#[test]
fn every_field_at_zero_is_refused_by_name() {
    let keys = flat_default();
    // No pinned count: every serialized key below must be refused by its own name, which already
    // requires a validation entry per field (the count pin broke on every new field).
    assert!(!keys.is_empty(), "the census looked at no field");
    for key in keys.keys() {
        let got = refused(&with_field(key, json!(0)));
        assert_eq!(got, BudgetRefusal::Zero { field: leak(key) }, "{key}");
    }
}

#[test]
fn every_ceiling_plus_one_is_refused_by_name() {
    let keys = flat_default();
    let mut ceilings = BTreeMap::new();
    for key in keys.keys() {
        // Probe at u64::MAX to learn the ceiling, then pin max + 1 exactly.
        let max = match refused(&with_field(key, json!(u64::MAX))) {
            BudgetRefusal::OverCeiling { field, max, .. } if field == key.as_str() => max,
            other => panic!("{key}: expected OverCeiling, got {other:?}"),
        };
        let got = refused(&with_field(key, json!(max + 1)));
        assert_eq!(
            got,
            BudgetRefusal::OverCeiling {
                field: leak(key),
                value: max + 1,
                max,
            },
            "{key}"
        );
        assert!(keys[key] <= max, "{key}: default {} over {max}", keys[key]);
        ceilings.insert(key.clone(), max);
    }
    let distinct: std::collections::BTreeSet<u64> = ceilings.values().copied().collect();
    assert_eq!(distinct.len(), 8, "ceiling count: {distinct:?}");
}

#[test]
fn unknown_key_is_a_json_error_not_a_default() {
    assert!(matches!(
        Budgets::parse(r#"{"door":{"max_requst":3}}"#),
        Err(BudgetParseError::Json(_))
    ));
    assert!(matches!(
        Budgets::parse(r#"{"doors":{}}"#),
        Err(BudgetParseError::Json(_))
    ));
    assert!(matches!(
        Budgets::parse("not json"),
        Err(BudgetParseError::Json(_))
    ));
    // A positional array never names a key: refused, not read as the default.
    for text in [
        "[]",
        "[[7]]",
        r#"{"door":[3]}"#,
        "[[],[],[],[],[],[],[],[]]",
    ] {
        let err = Budgets::parse(text).unwrap_err();
        assert!(matches!(err, BudgetParseError::Json(_)), "{text}: {err}");
        assert!(
            err.to_string().contains("expected an object"),
            "{text}: {err}"
        );
    }
    assert!(Budgets::parse(r#"{"door":{}}"#).is_ok());
}

/// `serde_json::from_str::<Budgets>` is the same path as `parse`: an unvalidated `Budgets`
/// cannot come out of serde.
#[test]
fn deserialize_runs_the_same_checks_as_parse() {
    let err = serde_json::from_str::<Budgets>(r#"{"door":{"pool":0}}"#).unwrap_err();
    assert!(err.to_string().contains("door.pool is zero"), "{err}");
    let err = serde_json::from_str::<Budgets>("[]").unwrap_err();
    assert!(err.to_string().contains("expected an object"), "{err}");
    let err = serde_json::from_str::<Budgets>(r#"{"door":{"max_requst":3}}"#).unwrap_err();
    assert!(err.to_string().contains("max_requst"), "{err}");
    let ok: Budgets = serde_json::from_str(r#"{"door":{"max_requests":3}}"#).unwrap();
    assert_eq!(
        ok,
        Budgets::parse(r#"{"door":{"max_requests":3}}"#).unwrap()
    );
    let round: Budgets =
        serde_json::from_str(&serde_json::to_string(&Budgets::DEFAULT).unwrap()).unwrap();
    assert_eq!(round, Budgets::DEFAULT);
}

#[test]
fn order_rules_and_floor_are_refused_by_name() {
    assert_eq!(
        refused(r#"{"door":{"max_body_bytes":2,"max_total_bytes":1}}"#),
        BudgetRefusal::Order {
            lesser: "door.max_body_bytes",
            greater: "door.max_total_bytes",
        }
    );
    assert_eq!(
        refused(r#"{"socket":{"frame_bytes":1023}}"#),
        BudgetRefusal::BelowFloor {
            field: "socket.frame_bytes",
            value: 1023,
            min: 1024,
        }
    );
    assert_eq!(
        Budgets::parse(r#"{"socket":{"frame_bytes":1024}}"#)
            .unwrap()
            .socket
            .frame_bytes,
        1024
    );
    assert_eq!(
        refused(r#"{"stream":{"close_deadline_ms":20000}}"#),
        BudgetRefusal::Order {
            lesser: "stream.close_deadline_ms",
            greater: "socket.write_deadline_ms",
        }
    );
}

#[test]
fn refusal_text_names_the_field() {
    let text = refused(&with_field("door.max_body_bytes", json!(0))).to_string();
    assert_eq!(text, "budgets: door.max_body_bytes is zero");
    let text = Budgets::parse(r#"{"door":{"max_requst":3}}"#)
        .unwrap_err()
        .to_string();
    assert!(text.starts_with("budgets: "), "{text}");
    assert!(text.contains("max_requst"), "{text}");
}

#[test]
fn render_names_every_field_once() {
    let line = Budgets::DEFAULT.render();
    assert!(!line.contains('\n'), "{line}");
    let keys = flat_default();
    let mut seen = BTreeMap::<String, usize>::new();
    for pair in line.split(' ') {
        let (k, v) = pair.split_once('=').unwrap();
        assert_eq!(v.parse::<u64>().unwrap(), keys[k], "{k}");
        *seen.entry(k.to_owned()).or_default() += 1;
    }
    assert_eq!(
        seen.keys().collect::<Vec<_>>(),
        keys.keys().collect::<Vec<_>>()
    );
    assert!(seen.values().all(|&n| n == 1), "{seen:?}");
    assert!(line.starts_with("socket.max_connections=256 "), "{line}");
}

#[test]
fn every_ms_field_has_a_duration_accessor() {
    let b = Budgets::DEFAULT;
    assert_eq!(b.door.header_deadline(), Duration::from_millis(2_000));
    let accessors = [
        ("socket.read_deadline_ms", b.socket.read_deadline()),
        ("socket.write_deadline_ms", b.socket.write_deadline()),
        ("socket.refusal_write_ms", b.socket.refusal_write()),
        ("socket.client_read_ms", b.socket.client_read()),
        ("stream.poll_ms", b.stream.poll()),
        ("stream.close_deadline_ms", b.stream.close_deadline()),
        ("door.header_deadline_ms", b.door.header_deadline()),
        ("door.body_deadline_ms", b.door.body_deadline()),
        ("door.io_timeout_ms", b.door.io_timeout()),
        ("attempt.timebox_default_ms", b.attempt.timebox_default()),
        ("attempt.deadline_ms", b.attempt.deadline()),
        ("dispatcher.idle_ms", b.dispatcher.idle()),
        ("dispatcher.error_backoff_ms", b.dispatcher.error_backoff()),
        ("model.tags_timeout_ms", b.model.tags_timeout()),
        ("ledger.busy_timeout_ms", b.ledger.busy_timeout()),
        ("stream.stall_ms", b.stream.stall()),
    ];
    let keys = flat_default();
    let ms_keys: Vec<&String> = keys.keys().filter(|k| k.ends_with("_ms")).collect();
    let covered: Vec<&str> = accessors.iter().map(|(k, _)| *k).collect();
    for key in &ms_keys {
        assert!(
            covered.contains(&key.as_str()),
            "no accessor listed for {key}"
        );
    }
    assert_eq!(
        ms_keys.len(),
        accessors.len(),
        "accessor list covers every _ms field"
    );
    for (key, d) in accessors {
        assert_eq!(d, Duration::from_millis(keys[key]), "{key}");
    }
}

/// Field names in refusals are `&'static str`; the test's keys are owned, so compare through a
/// leaked copy (test-only).
fn leak(s: &str) -> &'static str {
    Box::leak(s.to_owned().into_boxed_str())
}

/// `stream.stall_ms` (2026-10-06): zero is refused by name, and a stall window shorter than one
/// poll is refused by the order rule (equal is allowed), so a subscriber always gets at least one
/// poll's chance to drain before it is judged slow.
#[test]
fn stall_ms_is_validated_and_lasts_at_least_a_poll() {
    let zero = refused(&with_field("stream.stall_ms", json!(0)));
    assert!(format!("{zero:?}").contains("stream.stall_ms"), "{zero:?}");
    let poll = Budgets::DEFAULT.stream.poll_ms;
    let short = refused(&with_field("stream.stall_ms", json!(poll - 1)));
    assert!(
        matches!(short, BudgetRefusal::Order { lesser, greater } if lesser == "stream.poll_ms" && greater == "stream.stall_ms"),
        "{short:?}"
    );
    assert!(Budgets::parse(&with_field("stream.stall_ms", json!(poll))).is_ok());
    assert_eq!(
        Budgets::DEFAULT.stream.stall(),
        Duration::from_millis(Budgets::DEFAULT.stream.stall_ms)
    );
}

/// The stall window must end within the socket's write deadline, so `slow_consumer` is decided
/// before a blocked write can end the stream with a bare EOF (the refuter's equal-timer race).
#[test]
fn stall_ms_ends_within_the_write_deadline() {
    let write = Budgets::DEFAULT.socket.write_deadline_ms;
    assert!(
        Budgets::DEFAULT.stream.stall_ms < write,
        "the defaults must not race"
    );
    let long = refused(&with_field("stream.stall_ms", json!(write + 1)));
    assert!(
        matches!(long, BudgetRefusal::Order { lesser, greater } if lesser == "stream.stall_ms" && greater == "socket.write_deadline_ms"),
        "{long:?}"
    );
}
