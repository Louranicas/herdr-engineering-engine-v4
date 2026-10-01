//! Independent pure roster contracts; persistence and transport require separate evidence.
//! Freshness/selection receive immutable records already validated by the Store owner.

use habitat_engine::contracts::roster::{
    ActiveAttemptPolicy, Availability, Disable, Freshness, Invalid, Kind, Locality, Observation,
    ObservationInput, ObservationSource, ReceiptTime, RosterDefinitionV1, RosterHeadV1, Selection,
    Update, freshness,
};
use serde_json::{Value, json};

const RECORD: &str = "10000000-0000-4000-8000-000000000001";
const OTHER_RECORD: &str = "10000000-0000-4000-8000-000000000002";
const ENDPOINT: &str = "20000000-0000-4000-8000-000000000001";
const OTHER_ENDPOINT: &str = "20000000-0000-4000-8000-000000000002";
const INSTANCE: &str = "30000000-0000-4000-8000-000000000001";
const EVIDENCE: &str = "40000000-0000-4000-8000-000000000001";
const EPOCH: &str = "50000000-0000-4000-8000-000000000001";
const OTHER_EPOCH: &str = "50000000-0000-4000-8000-000000000002";
const OPERATION: &str = "60000000-0000-4000-8000-000000000001";
const OBSERVATION: &str = "70000000-0000-4000-8000-000000000001";

fn definition() -> RosterDefinitionV1 {
    RosterDefinitionV1 {
        kind: Kind::Agent,
        display_name: "Review agent".to_owned(),
        owner_id: "worker-fixture".to_owned(),
        version: "review-v7".to_owned(),
        capabilities: vec!["text".to_owned(), "tool_proposals".to_owned()],
        locality: Locality::Local,
        endpoint_ref: Some(ENDPOINT.to_owned()),
        limitations: String::new(),
    }
}

fn definition_json() -> Value {
    json!({
        "kind": "agent",
        "display_name": "Review agent",
        "owner_id": "worker-fixture",
        "version": "review-v7",
        "capabilities": ["text", "tool_proposals"],
        "locality": "local",
        "endpoint_ref": null,
        "limitations": ""
    })
}

fn head() -> RosterHeadV1 {
    RosterHeadV1 {
        record_id: RECORD.to_owned(),
        record_version: "7".to_owned(),
        definition: definition(),
        disabled: false,
        observation_cutoff_unix_ms: Some("1000000".to_owned()),
    }
}

fn observation_input() -> ObservationInput {
    ObservationInput {
        record_id: RECORD.to_owned(),
        record_version: "7".to_owned(),
        owner_id: "worker-fixture".to_owned(),
        endpoint_ref: Some(ENDPOINT.to_owned()),
        instance_id: None,
        instance_generation: None,
        source: ObservationSource::Worker,
        observed_unix_ms: Some(999_995),
        availability: Availability::Available,
        actual_identity: Some("provider/model-exact".to_owned()),
        immutable_revision: None,
        capabilities: vec!["text".to_owned(), "tool_proposals".to_owned()],
        evidence_ref: EVIDENCE.to_owned(),
    }
}

fn observed() -> Observation {
    Observation {
        id: OBSERVATION.to_owned(),
        confirmed_source: Some(ObservationSource::Worker),
        input: observation_input(),
        received: ReceiptTime {
            epoch: EPOCH.to_owned(),
            monotonic_ms: 1_000,
            unix_ms: 1_000_000,
        },
        sequence: 41,
    }
}

fn now() -> ReceiptTime {
    ReceiptTime {
        epoch: EPOCH.to_owned(),
        monotonic_ms: 1_099,
        unix_ms: 1_000_099,
    }
}

fn selection() -> Selection {
    Selection {
        record_id: RECORD.to_owned(),
        expected_revision: "7".to_owned(),
        capabilities: vec!["text".to_owned()],
        local_only: true,
        version: Some("review-v7".to_owned()),
        ttl_ms: 100,
    }
}

fn update() -> Update {
    Update {
        idempotency_key: OPERATION.to_owned(),
        record_id: None,
        expected_revision: None,
        definition: definition(),
        audit_reason: "Reviewed fixture change".to_owned(),
    }
}

#[test]
fn native_definition_requires_nullable_endpoint_and_rejects_unknown_fields_and_enums() {
    let valid = definition_json();
    let parsed: RosterDefinitionV1 = serde_json::from_value(valid.clone()).unwrap();
    assert_eq!(parsed.endpoint_ref, None);
    assert_eq!(parsed.validate(), Ok(()));

    let mut missing = valid.clone();
    missing.as_object_mut().unwrap().remove("endpoint_ref");
    assert!(serde_json::from_value::<RosterDefinitionV1>(missing).is_err());

    for (field, value) in [
        ("secret", json!("synthetic-not-a-reference")),
        ("kind", json!("running_agent")),
        ("locality", json!("automatic")),
    ] {
        let mut invalid = valid.clone();
        invalid[field] = value;
        assert!(serde_json::from_value::<RosterDefinitionV1>(invalid).is_err());
    }
}

#[test]
fn definition_text_counts_utf8_bytes_and_preserves_distinct_unicode_sequences() {
    let mut record = definition();
    record.display_name = "é".repeat(128);
    assert_eq!(record.display_name.len(), 256);
    assert_eq!(record.validate(), Ok(()));
    record.display_name.push('x');
    assert_eq!(record.validate(), Err(Invalid::Bound));

    for exact in ["é", "e\u{301}"] {
        let mut encoded = definition_json();
        encoded["display_name"] = json!(exact);
        let decoded: RosterDefinitionV1 = serde_json::from_value(encoded).unwrap();
        assert_eq!(decoded.validate(), Ok(()));
        assert_eq!(decoded.display_name.as_bytes(), exact.as_bytes());
    }
    record.display_name.clear();
    assert_eq!(record.validate(), Err(Invalid::Bound));
    record.display_name = "valid".to_owned();
    record.limitations = "é".repeat(1_024);
    assert_eq!(record.validate(), Ok(()));
    record.limitations.push('x');
    assert_eq!(record.validate(), Err(Invalid::Bound));
}

#[test]
fn ascii_labels_allow_controls_without_allowing_non_ascii_or_empty_labels() {
    let mut record = definition();
    record.owner_id = "owner\0\t\n\u{7f}".to_owned();
    record.version = "v\r\u{1b}".to_owned();
    record.capabilities = vec!["text\0".to_owned()];
    assert_eq!(record.validate(), Ok(()));
    assert_eq!(record.owner_id.as_bytes(), b"owner\0\t\n\x7f");

    record.owner_id = "ówner".to_owned();
    assert_eq!(record.validate(), Err(Invalid::Field));
    record.owner_id = "owner".to_owned();
    record.version.clear();
    assert_eq!(record.validate(), Err(Invalid::Bound));
    record.version = "v".repeat(128);
    assert_eq!(record.validate(), Ok(()));
    record.version.push('v');
    assert_eq!(record.validate(), Err(Invalid::Bound));
}

#[test]
fn capability_collection_is_bounded_but_empty_and_repeated_labels_remain_valid() {
    let mut record = definition();
    record.capabilities.clear();
    assert_eq!(record.validate(), Ok(()));
    record.capabilities = vec!["text".to_owned(); 128];
    assert_eq!(record.validate(), Ok(()));
    record.capabilities.push("text".to_owned());
    assert_eq!(record.validate(), Err(Invalid::Bound));

    record.capabilities = vec!["c".repeat(128)];
    assert_eq!(record.validate(), Ok(()));
    record.capabilities[0].push('c');
    assert_eq!(record.validate(), Err(Invalid::Bound));
    record.capabilities = vec![String::new()];
    assert_eq!(record.validate(), Err(Invalid::Bound));
    record.capabilities = vec!["têxt".to_owned()];
    assert_eq!(record.validate(), Err(Invalid::Field));
}

#[test]
fn endpoint_definition_accepts_only_an_opaque_uuid_reference_or_null() {
    let mut record = definition();
    for reference in [None, Some(ENDPOINT.to_owned())] {
        record.endpoint_ref = reference;
        assert_eq!(record.validate(), Ok(()));
    }
    for invalid in [
        "https://example.invalid/model",
        "/run/private/model.sock",
        "00000000-0000-0000-0000-000000000000",
        "20000000-0000-1000-8000-000000000001",
    ] {
        record.endpoint_ref = Some(invalid.to_owned());
        assert_eq!(record.validate(), Err(Invalid::Identity));
    }
}

#[test]
fn native_head_preserves_decimal_strings_and_requires_its_nullable_cutoff() {
    let valid = json!({
        "record_id": RECORD,
        "record_version": "9007199254740993",
        "definition": definition_json(),
        "disabled": true,
        "observation_cutoff_unix_ms": null
    });
    let decoded: RosterHeadV1 = serde_json::from_value(valid.clone()).unwrap();
    assert_eq!(decoded.record_version, "9007199254740993");
    assert_eq!(decoded.observation_cutoff_unix_ms, None);
    assert!(decoded.disabled);

    let mut missing = valid.clone();
    missing
        .as_object_mut()
        .unwrap()
        .remove("observation_cutoff_unix_ms");
    assert!(serde_json::from_value::<RosterHeadV1>(missing).is_err());
    let mut numeric = valid.clone();
    numeric["record_version"] = json!(9_007_199_254_740_993_u64);
    assert!(serde_json::from_value::<RosterHeadV1>(numeric).is_err());
    let mut expanded = valid;
    expanded["instance_id"] = json!(INSTANCE);
    assert!(serde_json::from_value::<RosterHeadV1>(expanded).is_err());
}

#[test]
fn update_distinguishes_absent_create_from_exact_nonzero_revision_update() {
    let mut request = update();
    assert_eq!(request.validate(), Ok(()));
    request.record_id = Some(RECORD.to_owned());
    assert_eq!(request.validate(), Err(Invalid::Precondition));
    request.expected_revision = Some("7".to_owned());
    assert_eq!(request.validate(), Ok(()));
    request.record_id = None;
    assert_eq!(request.validate(), Err(Invalid::Precondition));

    request.record_id = Some(RECORD.to_owned());
    for invalid in ["0", "07", "+7", "7 ", "18446744073709551616"] {
        request.expected_revision = Some(invalid.to_owned());
        assert_eq!(request.validate(), Err(Invalid::Precondition));
    }
    request.expected_revision = Some("18446744073709551615".to_owned());
    assert_eq!(request.validate(), Ok(()));
}

#[test]
fn update_requires_bounded_exact_audit_text_and_a_valid_operation_identity() {
    let mut request = update();
    request.audit_reason = "é".repeat(512);
    assert_eq!(request.validate(), Ok(()));
    request.audit_reason.push('x');
    assert_eq!(request.validate(), Err(Invalid::Bound));
    request.audit_reason.clear();
    assert_eq!(request.validate(), Err(Invalid::Bound));
    request.audit_reason = "Review\nline two".to_owned();
    assert_eq!(request.validate(), Ok(()));
    assert_eq!(request.audit_reason.as_bytes(), b"Review\nline two");
    request.idempotency_key = "not-an-operation-id".to_owned();
    assert_eq!(request.validate(), Err(Invalid::Identity));
}

#[test]
fn disable_requires_identity_revision_and_reason_without_inventing_an_enable_policy() {
    let mut request = Disable {
        idempotency_key: OPERATION.to_owned(),
        record_id: RECORD.to_owned(),
        expected_revision: "7".to_owned(),
        active_attempt_policy: ActiveAttemptPolicy::LeaveRunning,
        audit_reason: "Disable future selection".to_owned(),
    };
    assert_eq!(request.validate(), Ok(()));
    request.active_attempt_policy = ActiveAttemptPolicy::RequestCancel;
    assert_eq!(request.validate(), Ok(()));
    request.expected_revision = "0".to_owned();
    assert_eq!(request.validate(), Err(Invalid::Precondition));
    request.expected_revision = "7".to_owned();
    request.record_id = "not-a-record".to_owned();
    assert_eq!(request.validate(), Err(Invalid::Identity));
    request.record_id = RECORD.to_owned();
    request.audit_reason.clear();
    assert_eq!(request.validate(), Err(Invalid::Bound));
    assert!(serde_json::from_str::<ActiveAttemptPolicy>("\"enable\"").is_err());
}

#[test]
fn observation_instance_identity_and_generation_are_both_present_or_both_absent() {
    let mut input = observation_input();
    assert_eq!(input.validate(), Ok(()));
    input.instance_id = Some(INSTANCE.to_owned());
    assert_eq!(input.validate(), Err(Invalid::Identity));
    input.instance_generation = Some("3".to_owned());
    assert_eq!(input.validate(), Ok(()));
    input.instance_id = None;
    assert_eq!(input.validate(), Err(Invalid::Identity));
    input.instance_id = Some(INSTANCE.to_owned());
    input.instance_generation = Some("0".to_owned());
    assert_eq!(input.validate(), Err(Invalid::Precondition));
}

#[test]
fn immutable_model_revision_requires_actual_identity_while_absence_stays_unknown() {
    let mut input = observation_input();
    input.actual_identity = None;
    assert_eq!(input.validate(), Ok(()));
    input.immutable_revision = Some("provider-revision-7".to_owned());
    assert_eq!(input.validate(), Err(Invalid::Identity));
    input.actual_identity = Some("provider/model-exact".to_owned());
    assert_eq!(input.validate(), Ok(()));
    assert_eq!(
        input.immutable_revision.as_deref(),
        Some("provider-revision-7")
    );
    input.actual_identity = Some(String::new());
    assert_eq!(input.validate(), Err(Invalid::Bound));
    input.actual_identity = Some("m".repeat(256));
    assert_eq!(input.validate(), Ok(()));
    input.immutable_revision = Some("r".repeat(257));
    assert_eq!(input.validate(), Err(Invalid::Bound));
}

#[test]
fn observation_validation_rejects_unattributable_evidence_or_record_references() {
    let mut input = observation_input();
    input.evidence_ref = "https://example.invalid/proof".to_owned();
    assert_eq!(input.validate(), Err(Invalid::Identity));
    input.evidence_ref = EVIDENCE.to_owned();
    input.record_id = "not-a-record".to_owned();
    assert_eq!(input.validate(), Err(Invalid::Identity));
    input.record_id = RECORD.to_owned();
    input.record_version = "00".to_owned();
    assert_eq!(input.validate(), Err(Invalid::Precondition));
    input.record_version = "7".to_owned();
    input.owner_id = "ówner".to_owned();
    assert_eq!(input.validate(), Err(Invalid::Field));
    input.owner_id = "worker-fixture".to_owned();
    assert_eq!(input.validate(), Ok(()));
}

#[test]
fn production_observation_input_rejects_caller_supplied_receiver_clock_fields() {
    let valid = serde_json::to_value(observation_input()).unwrap();
    assert!(serde_json::from_value::<ObservationInput>(valid.clone()).is_ok());
    for (field, value) in [
        (
            "received",
            json!({"epoch": EPOCH, "monotonic_ms": 1099, "unix_ms": 1_000_099}),
        ),
        ("epoch", json!(EPOCH)),
        ("monotonic_ms", json!(1_099)),
        ("unix_ms", json!(1_000_099)),
    ] {
        let mut forged = valid.clone();
        forged[field] = value;
        assert!(serde_json::from_value::<ObservationInput>(forged).is_err());
    }
}

#[test]
fn a_declared_record_without_observation_has_missing_proof_and_cannot_be_selected() {
    let record = head();
    assert_eq!(freshness(&record, None, &now(), 100), Freshness::Missing);
    assert!(!selection().permits(&record, None, &now()));
    assert_eq!(record.definition.capabilities, ["text", "tool_proposals"]);
    assert!(selection().permits(&record, Some(&observed()), &now()));
}

#[test]
fn freshness_uses_strict_receiver_age_with_bounded_ttl_and_checked_u64_arithmetic() {
    let record = head();
    let observation = observed();
    let mut sample = now();
    for (age, ttl, expected) in [
        (0, 1, Freshness::Fresh),
        (1, 1, Freshness::Expired),
        (99, 100, Freshness::Fresh),
        (100, 100, Freshness::Expired),
        (59_999, 60_000, Freshness::Fresh),
        (60_000, 60_000, Freshness::Expired),
        (0, 0, Freshness::Expired),
        (0, 60_001, Freshness::Expired),
    ] {
        sample.monotonic_ms = 1_000 + age;
        sample.unix_ms = 1_000_000 + age;
        assert_eq!(
            freshness(&record, Some(&observation), &sample, ttl),
            expected
        );
    }

    let mut high = observation;
    high.received.monotonic_ms = u64::MAX - 1;
    sample.monotonic_ms = u64::MAX;
    assert_eq!(
        freshness(&record, Some(&high), &sample, 2),
        Freshness::Fresh
    );
    high.received.monotonic_ms = 0;
    assert_eq!(
        freshness(&record, Some(&high), &sample, 2),
        Freshness::Expired
    );
}

#[test]
fn a_new_receiver_boot_epoch_invalidates_freshness_without_erasing_stored_proof() {
    let observation = observed();
    let mut sample = now();
    sample.epoch = OTHER_EPOCH.to_owned();
    assert_eq!(
        freshness(&head(), Some(&observation), &sample, 100),
        Freshness::EpochMismatch
    );
    assert!(!selection().permits(&head(), Some(&observation), &sample));
    assert_eq!(observation.received.epoch, EPOCH);
    assert_eq!(observation.input.evidence_ref, EVIDENCE);
    sample.epoch = EPOCH.to_owned();
    assert_eq!(
        freshness(&head(), Some(&observation), &sample, 100),
        Freshness::Fresh
    );
}

#[test]
fn receiving_monotonic_regression_cannot_become_unsigned_age_or_renewal() {
    let observation = observed();
    let mut sample = now();
    sample.monotonic_ms = 999;
    assert_eq!(
        freshness(&head(), Some(&observation), &sample, 100),
        Freshness::ClockRegression
    );
    assert!(!selection().permits(&head(), Some(&observation), &sample));
    sample.monotonic_ms = 1_000;
    assert_eq!(
        freshness(&head(), Some(&observation), &sample, 100),
        Freshness::Fresh
    );
}

#[test]
fn receiving_unix_clock_regression_cannot_be_hidden_by_forward_monotonic_time() {
    let observation = observed();
    let mut sample = now();
    sample.unix_ms = 999_999;
    assert_eq!(
        freshness(&head(), Some(&observation), &sample, 100),
        Freshness::ClockRegression
    );
    sample.unix_ms = 1_000_000;
    assert_eq!(
        freshness(&head(), Some(&observation), &sample, 100),
        Freshness::Fresh
    );
}

#[test]
fn sender_time_cannot_be_after_receipt_or_replace_receiving_age() {
    let mut observation = observed();
    observation.input.observed_unix_ms = Some(1_000_001);
    assert_eq!(
        freshness(&head(), Some(&observation), &now(), 100),
        Freshness::FutureSource
    );
    for sender in [None, Some(0), Some(1_000_000)] {
        observation.input.observed_unix_ms = sender;
        assert_eq!(
            freshness(&head(), Some(&observation), &now(), 100),
            Freshness::Fresh
        );
    }
    let expired = ReceiptTime {
        epoch: EPOCH.to_owned(),
        monotonic_ms: 1_100,
        unix_ms: 1_000_100,
    };
    assert_eq!(
        freshness(&head(), Some(&observation), &expired, 100),
        Freshness::Expired
    );
}

#[test]
fn requested_source_does_not_prove_effective_identity_or_capability() {
    let mut observation = observed();
    observation.input.source = ObservationSource::Requested;
    observation.input.actual_identity = Some("requested-alias".to_owned());
    observation.input.immutable_revision = Some("claimed-revision".to_owned());
    assert_eq!(observation.input.validate(), Ok(()));
    assert_eq!(
        freshness(&head(), Some(&observation), &now(), 100),
        Freshness::UntrustedSource
    );
    assert!(!selection().permits(&head(), Some(&observation), &now()));
    observation.input.source = ObservationSource::Worker;
    observation.confirmed_source = None;
    assert_eq!(
        freshness(&head(), Some(&observation), &now(), 100),
        Freshness::UntrustedSource
    );
    observation.confirmed_source = Some(ObservationSource::ProviderResponse);
    assert_eq!(
        freshness(&head(), Some(&observation), &now(), 100),
        Freshness::UntrustedSource
    );
    observation.confirmed_source = Some(ObservationSource::Worker);
    assert_eq!(
        freshness(&head(), Some(&observation), &now(), 100),
        Freshness::Fresh
    );
}

#[test]
fn freshness_binds_exact_record_revision_owner_and_nullable_endpoint() {
    let record = head();
    let original = observed();
    let mut wrong = original.clone();
    wrong.input.record_id = OTHER_RECORD.to_owned();
    assert_eq!(
        freshness(&record, Some(&wrong), &now(), 100),
        Freshness::WrongBinding
    );
    wrong = original.clone();
    wrong.input.record_version = "8".to_owned();
    assert_eq!(
        freshness(&record, Some(&wrong), &now(), 100),
        Freshness::WrongBinding
    );
    wrong = original.clone();
    wrong.input.owner_id = "another-worker".to_owned();
    assert_eq!(
        freshness(&record, Some(&wrong), &now(), 100),
        Freshness::WrongBinding
    );
    for endpoint in [None, Some(OTHER_ENDPOINT.to_owned())] {
        wrong = original.clone();
        wrong.input.endpoint_ref = endpoint;
        assert_eq!(
            freshness(&record, Some(&wrong), &now(), 100),
            Freshness::WrongBinding
        );
    }
    assert_eq!(
        freshness(&record, Some(&original), &now(), 100),
        Freshness::Fresh
    );
    let mut no_endpoint = record;
    no_endpoint.definition.endpoint_ref = None;
    wrong = original;
    wrong.input.endpoint_ref = None;
    assert_eq!(
        freshness(&no_endpoint, Some(&wrong), &now(), 100),
        Freshness::Fresh
    );
}

#[test]
fn effective_required_capabilities_are_declared_and_fresh_observed_intersection() {
    let mut record = head();
    let mut observation = observed();
    let wanted = selection();
    assert!(wanted.permits(&record, Some(&observation), &now()));
    record.definition.capabilities = vec!["tool_proposals".to_owned()];
    assert!(!wanted.permits(&record, Some(&observation), &now()));
    record = head();
    observation.input.capabilities = vec!["tool_proposals".to_owned()];
    assert!(!wanted.permits(&record, Some(&observation), &now()));
    record.definition.capabilities.push("text".to_owned());
    observation.input.capabilities = vec!["text".to_owned(); 2];
    assert!(wanted.permits(&record, Some(&observation), &now()));
}

#[test]
fn recent_unavailable_or_unknown_observation_remains_ineligible() {
    let mut observation = observed();
    for unavailable in [Availability::Unavailable, Availability::Unknown] {
        observation.input.availability = unavailable;
        assert_eq!(
            freshness(&head(), Some(&observation), &now(), 100),
            Freshness::Fresh
        );
        assert!(!selection().permits(&head(), Some(&observation), &now()));
        assert_eq!(observation.input.availability, unavailable);
    }
    observation.input.availability = Availability::Available;
    assert!(selection().permits(&head(), Some(&observation), &now()));
}

#[test]
fn disabled_record_refuses_selection_even_with_fresh_matching_available_proof() {
    let mut record = head();
    record.disabled = true;
    assert_eq!(
        freshness(&record, Some(&observed()), &now(), 100),
        Freshness::Fresh
    );
    assert!(!selection().permits(&record, Some(&observed()), &now()));
    assert_eq!(record.definition.version, "review-v7");
    record.disabled = false;
    assert!(selection().permits(&record, Some(&observed()), &now()));
}

#[test]
fn selection_binds_its_expected_record_and_revision_independently_of_proof_freshness() {
    let mut wanted = selection();
    wanted.record_id = OTHER_RECORD.to_owned();
    assert!(!wanted.permits(&head(), Some(&observed()), &now()));
    wanted.record_id = RECORD.to_owned();
    wanted.expected_revision = "8".to_owned();
    assert!(!wanted.permits(&head(), Some(&observed()), &now()));
    wanted.expected_revision = "7".to_owned();
    assert!(wanted.permits(&head(), Some(&observed()), &now()));
}

#[test]
fn local_only_selection_refuses_remote_and_hybrid_without_fallback() {
    let mut wanted = selection();
    let mut record = head();
    for nonlocal in [Locality::Remote, Locality::Hybrid] {
        record.definition.locality = nonlocal;
        assert!(!wanted.permits(&record, Some(&observed()), &now()));
        wanted.local_only = false;
        assert!(wanted.permits(&record, Some(&observed()), &now()));
        wanted.local_only = true;
    }
    record.definition.locality = Locality::Local;
    assert!(wanted.permits(&record, Some(&observed()), &now()));
}

#[test]
fn selected_definition_version_is_exact_and_an_optional_filter_is_not_an_alias() {
    let mut wanted = selection();
    for different in ["review-v8", "REVIEW-V7", "review-v7 "] {
        wanted.version = Some(different.to_owned());
        assert!(!wanted.permits(&head(), Some(&observed()), &now()));
    }
    wanted.version = None;
    assert!(wanted.permits(&head(), Some(&observed()), &now()));
    wanted.version = Some("review-v7".to_owned());
    assert!(wanted.permits(&head(), Some(&observed()), &now()));
}

#[test]
fn empty_required_capabilities_still_require_enabled_fresh_available_proof() {
    let mut wanted = selection();
    wanted.capabilities.clear();
    let mut observation = observed();
    assert!(wanted.permits(&head(), Some(&observation), &now()));
    assert!(!wanted.permits(&head(), None, &now()));
    observation.input.availability = Availability::Unknown;
    assert!(!wanted.permits(&head(), Some(&observation), &now()));
    observation.input.availability = Availability::Available;
    let mut record = head();
    record.disabled = true;
    assert!(!wanted.permits(&record, Some(&observation), &now()));
    record.disabled = false;
    let expired = ReceiptTime {
        monotonic_ms: 1_100,
        ..now()
    };
    assert!(!wanted.permits(&record, Some(&observation), &expired));
}

#[test]
fn malformed_selection_constraints_cannot_become_a_permitted_query() {
    let mut wanted = selection();
    for ttl in [0, 60_001, u64::MAX] {
        wanted.ttl_ms = ttl;
        assert_eq!(wanted.validate(), Err(Invalid::Bound));
        assert!(!wanted.permits(&head(), Some(&observed()), &now()));
    }
    wanted.ttl_ms = 100;
    wanted.capabilities = vec![String::new()];
    assert_eq!(wanted.validate(), Err(Invalid::Bound));
    assert!(!wanted.permits(&head(), Some(&observed()), &now()));
    wanted.capabilities = vec!["têxt".to_owned()];
    assert_eq!(wanted.validate(), Err(Invalid::Field));
    wanted.capabilities = vec!["text".to_owned()];
    wanted.expected_revision = "07".to_owned();
    assert_eq!(wanted.validate(), Err(Invalid::Precondition));
    wanted.expected_revision = "7".to_owned();
    wanted.version = Some(String::new());
    assert_eq!(wanted.validate(), Err(Invalid::Bound));
    wanted.version = Some("review-v7".to_owned());
    assert!(wanted.permits(&head(), Some(&observed()), &now()));
}
