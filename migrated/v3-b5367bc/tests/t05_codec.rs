//! Independent input/output examples for the roster TOML and snapshot boundary.
use habitat_engine::contracts::roster::{
    Availability, Kind, Locality, Observation, ObservationInput, ObservationSource, ReceiptTime,
    Record, RosterDefinitionV1, RosterHeadV1, Snapshot,
};
use habitat_engine::roster::{Query, export_snapshot, parse_changes, query_snapshot};

fn id(n: u32) -> String {
    format!("00000000-0000-4000-8000-{n:012x}")
}
fn manifest() -> String {
    format!(
        r#"kind = "hee3-roster-changes"
version = 1
[[updates]]
idempotency_key = "{}"
audit_reason = "reviewed fixture"
[updates.definition]
kind = "agent"
display_name = "Fixture agent"
owner_id = "worker-owner"
version = "v1"
capabilities = ["edit", "edit"]
locality = "local"
limitations = ""
"#,
        id(1)
    )
}

fn record(n: u32, disabled: bool) -> Record {
    Record {
        head: RosterHeadV1 {
            record_id: id(n),
            record_version: "1".into(),
            definition: RosterDefinitionV1 {
                kind: Kind::Agent,
                display_name: format!("Agent {n}"),
                owner_id: "worker-owner".into(),
                version: "v1".into(),
                capabilities: vec!["edit".into()],
                locality: Locality::Local,
                endpoint_ref: None,
                limitations: String::new(),
            },
            disabled,
            observation_cutoff_unix_ms: None,
        },
        observation: None,
    }
}

fn snapshot() -> Snapshot {
    Snapshot {
        epoch: id(50),
        cutoff: 42,
        now: ReceiptTime {
            epoch: id(51),
            monotonic_ms: 100,
            unix_ms: 1_000,
        },
        records: vec![record(3, false), record(2, true), record(1, false)],
    }
}

fn query() -> Query {
    Query {
        kind: None,
        locality: None,
        version: None,
        capabilities: vec![],
        ttl_ms: 50,
        include_disabled: false,
        local_only: false,
    }
}

#[test]
fn explicit_changes_decode_to_reviewed_create_intent() {
    let updates = parse_changes(manifest().as_bytes()).unwrap();
    assert_eq!(updates.len(), 1);
    let value = &updates[0];
    assert_eq!(value.idempotency_key, id(1));
    assert_eq!(value.record_id, None);
    assert_eq!(value.expected_revision, None);
    assert_eq!(value.audit_reason, "reviewed fixture");
    assert_eq!(value.definition.capabilities, vec!["edit", "edit"]);
    assert_eq!(value.definition.endpoint_ref, None);
    assert_eq!(value.definition.limitations, "");
}

#[test]
fn explicit_empty_manifest_is_a_noop_and_not_an_implicit_default() {
    assert!(
        parse_changes(b"kind='hee3-roster-changes'\nversion=1\nupdates=[]\n")
            .unwrap()
            .is_empty()
    );
    assert!(parse_changes(b"").is_err());
    assert!(parse_changes(b"kind='hee3-roster-changes'\nversion=1\n").is_err());
}

#[test]
fn toml_controls_unicode_and_nullable_reference_preserve_exact_values() {
    let source = manifest()
        .replace("worker-owner", r"owner\u0000\t")
        .replace("Fixture agent", "Équipe λ")
        .replace(
            "limitations = \"\"",
            &format!("endpoint_ref = \"{}\"\nlimitations = \"\"", id(99)),
        );
    let updates = parse_changes(source.as_bytes()).unwrap();
    assert_eq!(updates[0].definition.owner_id, "owner\0\t");
    assert_eq!(updates[0].definition.display_name, "Équipe λ");
    assert_eq!(updates[0].definition.endpoint_ref, Some(id(99)));
}

#[test]
fn toml_rejects_duplicate_keys_unknown_fields_and_unsupported_document_kind() {
    let source = manifest();
    for bad in [
        source.replace("version = 1", "version = 1\nversion = 1"),
        source.replace("version = 1", "version = 2"),
        source.replace("hee3-roster-changes", "hee3-roster-snapshot"),
        source.replace("version = 1", "version = 1\nsecret = 'sentinel'"),
        source.replace(
            "limitations = \"\"",
            "limitations = \"\"\napi_key = 'sentinel'",
        ),
    ] {
        assert!(parse_changes(bad.as_bytes()).is_err());
    }
    assert!(parse_changes(source.as_bytes()).is_ok());
}

#[test]
fn toml_rejects_duplicate_operation_identity_before_returning_a_plan() {
    let source = manifest();
    let (_, row) = source.split_once("[[updates]]").unwrap();
    assert!(parse_changes(format!("{source}\n[[updates]]{row}").as_bytes()).is_err());
    let different = row.replace(&id(1), &id(2));
    assert_eq!(
        parse_changes(format!("{source}\n[[updates]]{different}").as_bytes())
            .unwrap()
            .len(),
        2
    );
}

#[test]
fn update_requires_exact_target_and_revision_without_implicit_create() {
    let source = manifest().replace(
        "audit_reason =",
        &format!(
            "record_id = \"{}\"\nexpected_revision = \"2\"\naudit_reason =",
            id(20)
        ),
    );
    let update = parse_changes(source.as_bytes()).unwrap().remove(0);
    assert_eq!(update.record_id, Some(id(20)));
    assert_eq!(update.expected_revision, Some("2".into()));
    assert!(parse_changes(source.replace("expected_revision = \"2\"\n", "").as_bytes()).is_err());
    assert!(
        parse_changes(
            source
                .replace("expected_revision = \"2\"", "expected_revision = \"02\"")
                .as_bytes()
        )
        .is_err()
    );
}

#[test]
fn whole_source_byte_ceiling_has_an_exact_usable_boundary() {
    let mut source = manifest();
    source.push('#');
    source.extend(std::iter::repeat_n(' ', 1_048_576 - source.len()));
    assert_eq!(source.len(), 1_048_576);
    assert_eq!(parse_changes(source.as_bytes()).unwrap().len(), 1);
    source.push(' ');
    assert!(parse_changes(source.as_bytes()).is_err());
}

#[test]
fn row_ceiling_refuses_the_first_excess_entry_without_truncation() {
    let source = manifest();
    let (header, row) = source.split_once("[[updates]]").unwrap();
    let mut batch = header.to_owned();
    for n in 1..=256 {
        batch.push_str("[[updates]]");
        batch.push_str(&row.replace(&id(1), &id(n)));
    }
    assert_eq!(parse_changes(batch.as_bytes()).unwrap().len(), 256);
    batch.push_str("[[updates]]");
    batch.push_str(&row.replace(&id(1), &id(257)));
    assert!(parse_changes(batch.as_bytes()).is_err());
}

#[test]
fn snapshot_export_sorts_identity_and_preserves_disabled_revision_and_escaped_text() {
    let mut captured = snapshot();
    captured.records[1].head.record_version = "9".into();
    captured.records[1].head.definition.owner_id = "owner\0\t".into();
    let exported = export_snapshot(&captured).unwrap();
    let value: toml::Value = toml::from_str(&exported).unwrap();
    assert_eq!(value["kind"].as_str(), Some("hee3-roster-snapshot"));
    let records = value["records"].as_array().unwrap();
    assert_eq!(records.len(), 3);
    assert_eq!(records[0]["record_id"].as_str(), Some(id(1).as_str()));
    assert_eq!(records[1]["record_version"].as_str(), Some("9"));
    assert_eq!(records[1]["disabled"].as_bool(), Some(true));
    assert_eq!(
        records[1]["definition"]["owner_id"].as_str(),
        Some("owner\0\t")
    );
    captured.records.reverse();
    assert_eq!(exported, export_snapshot(&captured).unwrap());
    assert!(parse_changes(exported.as_bytes()).is_err());
}

#[test]
fn inventory_paging_keeps_one_snapshot_and_explicit_disabled_visibility() {
    let captured = snapshot();
    let filter = query();
    let first = query_snapshot(&captured, &filter, 0, 1).unwrap();
    assert_eq!(first.total, 2);
    assert_eq!(first.records.len(), 1);
    assert_eq!(first.next_offset, Some(1));
    let second = query_snapshot(&captured, &filter, 1, 1).unwrap();
    assert_eq!(second.total, 2);
    assert_eq!(second.records.len(), 1);
    assert_eq!(second.next_offset, None);
    assert_ne!(
        first.records[0].record.head.record_id,
        second.records[0].record.head.record_id
    );
    assert_eq!(first.snapshot.cutoff, second.snapshot.cutoff);
    let mut include = filter;
    include.include_disabled = true;
    assert_eq!(
        query_snapshot(&captured, &include, 0, 256).unwrap().total,
        3
    );
}

#[test]
fn capability_queries_require_declared_and_confirmed_fresh_available_facts() {
    let mut captured = snapshot();
    captured.records.truncate(1);
    let h = &captured.records[0].head;
    let observed = Observation {
        id: id(80),
        confirmed_source: Some(ObservationSource::Worker),
        input: ObservationInput {
            record_id: h.record_id.clone(),
            record_version: h.record_version.clone(),
            owner_id: h.definition.owner_id.clone(),
            endpoint_ref: None,
            instance_id: None,
            instance_generation: None,
            source: ObservationSource::Worker,
            observed_unix_ms: Some(990),
            availability: Availability::Available,
            actual_identity: Some("worker-runtime".into()),
            immutable_revision: Some("exact-runtime-1".into()),
            capabilities: vec!["edit".into()],
            evidence_ref: id(81),
        },
        received: ReceiptTime {
            epoch: id(51),
            monotonic_ms: 90,
            unix_ms: 990,
        },
        sequence: 41,
    };
    let mut filter = query();
    filter.capabilities = vec!["edit".into()];
    filter.local_only = true;
    assert_eq!(query_snapshot(&captured, &filter, 0, 256).unwrap().total, 0);
    captured.records[0].observation = Some(observed.clone());
    assert_eq!(query_snapshot(&captured, &filter, 0, 256).unwrap().total, 1);
    captured.records[0].head.definition.capabilities.clear();
    assert_eq!(query_snapshot(&captured, &filter, 0, 256).unwrap().total, 0);
    captured.records[0].head.definition.capabilities = vec!["edit".into()];
    captured.now.monotonic_ms = 140;
    assert_eq!(query_snapshot(&captured, &filter, 0, 256).unwrap().total, 0);
}

#[test]
fn query_bounds_and_exact_filters_refuse_invalid_or_substituted_inputs() {
    let captured = snapshot();
    let mut filter = query();
    assert!(query_snapshot(&captured, &filter, 0, 0).is_err());
    assert!(query_snapshot(&captured, &filter, 0, 257).is_err());
    filter.ttl_ms = 0;
    assert!(query_snapshot(&captured, &filter, 0, 1).is_err());
    filter.ttl_ms = 50;
    filter.kind = Some(Kind::Model);
    assert_eq!(query_snapshot(&captured, &filter, 0, 256).unwrap().total, 0);
    filter.kind = None;
    filter.version = Some("v2".into());
    assert_eq!(query_snapshot(&captured, &filter, 0, 256).unwrap().total, 0);
}
