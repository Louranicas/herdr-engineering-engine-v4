//! Independent RC04/schema structural oracles. Baseline fixture authorship is shared.
//! No reference resolution, producer authentication, custody or admission is asserted.

use habitat_engine::contracts::receipt::{
    Error, Id, LanguageFlagsV1, Record, decode_record, reference_for,
};
use serde_json::{Value, json};

const FIXTURES: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/receipts/inventory-examples.json"
));
const ID: &str = "123e4567-e89b-42d3-a456-000000000001";

fn schema(name: &str) -> String {
    format!("hee3.receipt/1:{name}")
}
fn fixture(name: &str) -> Value {
    serde_json::from_str::<Value>(FIXTURES).unwrap()["definitions"][name].clone()
}
fn bytes(value: &Value) -> Vec<u8> {
    serde_json::to_vec(value).unwrap()
}
fn check(name: &str, value: &Value) -> Result<Record, Error> {
    decode_record(&schema(name), &bytes(value))
}
fn good(name: &str, value: &Value) {
    assert!(
        check(name, value).is_ok(),
        "expected structural validity: {name}"
    );
}
fn bad(name: &str, value: &Value) {
    assert!(
        check(name, value).is_err(),
        "expected structural refusal: {name}"
    );
}
fn put(value: &mut Value, pointer: &str, replacement: Value) {
    *value.pointer_mut(pointer).unwrap() = replacement;
}
fn rejects(name: &str, pointer: &str, replacements: &[Value]) {
    let baseline = fixture(name);
    good(name, &baseline);
    for replacement in replacements {
        let mut changed = baseline.clone();
        put(&mut changed, pointer, replacement.clone());
        bad(name, &changed);
    }
}
fn available(value: Value) -> Value {
    Value::Object(serde_json::Map::from_iter([
        ("value".to_owned(), value),
        ("unavailable_reason".to_owned(), Value::Null),
    ]))
}
fn unavailable(reason: &str) -> Value {
    json!({"value":null,"unavailable_reason":reason})
}
fn reference(target: &str) -> Value {
    json!({"artifact_id":ID,"sha256":"sha256:1111111111111111111111111111111111111111111111111111111111111111",
        "byte_length":0,"media_type":"application/json","schema_id":target})
}
fn nonfinal_page() -> Value {
    let mut value = fixture("CasePageV1");
    value["page_count"] = json!(2);
    value["total_rows"] = json!(2);
    value["next"] = available(reference(&schema("CasePageV1")));
    value
}

#[test]
fn root_required_closed_identity() {
    let mut value = fixture("ReceiptV1");
    good("ReceiptV1", &value);
    value.as_object_mut().unwrap().remove("identity");
    bad("ReceiptV1", &value);
    value = fixture("ReceiptV1");
    value["accepted"] = json!(true);
    bad("ReceiptV1", &value);
}

#[test]
fn protocol_version_serialization_constants() {
    rejects(
        "ReceiptV1",
        "/protocol",
        &[json!("hee3.receipts"), json!(null)],
    );
    rejects("ReceiptV1", "/version", &[json!(2), json!("1")]);
    rejects(
        "ReceiptV1",
        "/serialization",
        &[json!("json"), json!("json-exact-v2")],
    );
}

#[test]
fn utf8_bom_and_invalid_sequences() {
    let original = bytes(&fixture("ReceiptV1"));
    let mut bom = vec![0xef, 0xbb, 0xbf];
    bom.extend_from_slice(&original);
    assert!(decode_record(&schema("ReceiptV1"), &bom).is_err());
    for raw in [
        b"{\"language\":\"\xff\",\"argv\":[]}".as_slice(),
        b"{\"language\":\"\xc0\xaf\",\"argv\":[]}".as_slice(),
    ] {
        assert!(decode_record(&schema("LanguageFlagsV1"), raw).is_err());
    }
    assert!(
        decode_record(
            &schema("LanguageFlagsV1"),
            br#"{"language":"rust","argv":["\u03bb"]}"#
        )
        .is_ok()
    );
    assert!(
        decode_record(
            &schema("LanguageFlagsV1"),
            br#"{"language":"rust","argv":["\ud800"]}"#
        )
        .is_err()
    );
}

#[test]
fn compact_whitespace_outside_strings() {
    for raw in [
        b" {\"language\":\"rust\",\"argv\":[]}".as_slice(),
        b"{\"language\": \"rust\",\"argv\":[]}".as_slice(),
        b"{\"language\":\"rust\",\t\"argv\":[]}".as_slice(),
    ] {
        assert!(decode_record(&schema("LanguageFlagsV1"), raw).is_err());
    }
    assert!(
        decode_record(
            &schema("LanguageFlagsV1"),
            br#"{"language":"rust","argv":["a b\t c"]}"#
        )
        .is_ok()
    );
}

#[test]
fn trailing_values_and_lf_are_not_part_of_manifest() {
    let original = br#"{"language":"rust","argv":[]}"#;
    for suffix in [b"\n".as_slice(), b"\r", b"{}", b"x", b" "] {
        let mut raw = original.to_vec();
        raw.extend_from_slice(suffix);
        assert!(decode_record(&schema("LanguageFlagsV1"), &raw).is_err());
    }
    assert!(decode_record(&schema("LanguageFlagsV1"), original).is_ok());
}

#[test]
fn duplicate_root_and_escaped_equivalent_keys() {
    for raw in [
        br#"{"language":"rust","language":"rust","argv":[]}"#.as_slice(),
        br#"{"language":"rust","langu\u0061ge":"rust","argv":[]}"#.as_slice(),
    ] {
        assert!(decode_record(&schema("LanguageFlagsV1"), raw).is_err());
    }
    assert!(
        decode_record(
            &schema("LanguageFlagsV1"),
            br#"{"argv":[],"language":"rust"}"#
        )
        .is_ok()
    );
}

#[test]
fn duplicate_nested_fields_cannot_be_hidden() {
    let value = fixture("ResourceV1");
    good("ResourceV1", &value);
    let original = String::from_utf8(bytes(&value)).unwrap();
    for needle in [
        "\"value\":\"1\"".to_owned(),
        format!("\"artifact_id\":\"{ID}\""),
    ] {
        assert_eq!(original.matches(&needle).count(), 1);
        let raw = original.replacen(&needle, &format!("{needle},{needle}"), 1);
        assert!(decode_record(&schema("ResourceV1"), raw.as_bytes()).is_err());
    }
}

#[test]
fn one_mib_exact_byte_boundary() {
    let mut value = fixture("InvocationV1");
    value["argv"] = json!(vec![String::new(); 256]);
    let mut remaining = 1_048_576 - bytes(&value).len();
    let values = value["argv"].as_array_mut().unwrap();
    for item in values.iter_mut() {
        let count = remaining.min(4096);
        *item = json!("x".repeat(count));
        remaining -= count;
    }
    assert_eq!(remaining, 0);
    assert_eq!(bytes(&value).len(), 1_048_576);
    good("InvocationV1", &value);
    let item = value["argv"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|item| item.as_str().unwrap().len() < 4096)
        .unwrap();
    *item = json!(format!("{}x", item.as_str().unwrap()));
    assert_eq!(bytes(&value).len(), 1_048_577);
    assert!(matches!(check("InvocationV1", &value), Err(Error::Bound)));
}

#[test]
fn deep_invalid_nesting_is_bounded_refusal() {
    let raw = format!(
        "{{\"language\":\"rust\",\"argv\":[],\"extra\":{}0{}}}",
        "[".repeat(1000),
        "]".repeat(1000)
    );
    assert!(decode_record(&schema("LanguageFlagsV1"), raw.as_bytes()).is_err());
    good("LanguageFlagsV1", &fixture("LanguageFlagsV1"));
}

#[test]
fn uuid_version_variant_case_and_length() {
    rejects(
        "IdentityV1",
        "/run_id",
        &[
            json!("123E4567-e89b-42d3-a456-000000000001"),
            json!("123e4567-e89b-12d3-a456-000000000001"),
            json!("123e4567-e89b-42d3-7456-000000000001"),
            json!("123e4567-e89b-42d3-a456-00000000001"),
        ],
    );
    for variant in ['8', '9', 'a', 'b'] {
        let mut value = fixture("IdentityV1");
        value["run_id"] = json!(format!("123e4567-e89b-42d3-{variant}456-000000000001"));
        good("IdentityV1", &value);
    }
}

#[test]
fn u64_decimal_extremes_and_noncanonical_forms() {
    for value in [json!("0"), json!("18446744073709551615")] {
        let mut record = fixture("ResourceV1");
        record["value"] = available(value);
        good("ResourceV1", &record);
    }
    for value in [
        json!("18446744073709551616"),
        json!("00"),
        json!("+1"),
        json!("-1"),
        json!("1e0"),
        json!("１"),
        json!(0),
    ] {
        let mut record = fixture("ResourceV1");
        record["value"] = available(value);
        bad("ResourceV1", &record);
    }
}

#[test]
fn generation_excludes_zero_but_not_maximum() {
    rejects("IdentityV1", "/generation", &[json!("0")]);
    let mut value = fixture("IdentityV1");
    value["generation"] = json!("18446744073709551615");
    good("IdentityV1", &value);
}

#[test]
fn digest_prefix_lower_hex_and_exact_size() {
    let mut value = fixture("SubjectV1");
    good("SubjectV1", &value);
    for digest in [
        format!("SHA256:{}", "a".repeat(64)),
        format!("sha256:{}", "A".repeat(64)),
        format!("sha256:{}", "a".repeat(63)),
        format!("sha256:{}", "g".repeat(64)),
    ] {
        value["tree_sha256"] = json!(digest);
        value["files"]["sha256"] = value["tree_sha256"].clone();
        bad("SubjectV1", &value);
    }
    value["tree_sha256"] = json!(format!("sha256:{}", "0".repeat(64)));
    value["files"]["sha256"] = value["tree_sha256"].clone();
    good("SubjectV1", &value);
}

#[test]
fn ascii_names_preserve_controls_with_byte_limits() {
    let mut value = fixture("IdentityV1");
    value["module_id"] = json!("\0\t\n\u{7f}");
    good("IdentityV1", &value);
    value["module_id"] = json!("x".repeat(128));
    good("IdentityV1", &value);
    rejects(
        "IdentityV1",
        "/module_id",
        &[json!(""), json!("x".repeat(129)), json!("é")],
    );
}

#[test]
fn text_is_utf8_bytes_without_normalization() {
    let mut value = fixture("ReviewV1");
    for text in [String::new(), "é".repeat(2048), "e\u{301}".repeat(1365)] {
        value["reviewer"] = json!(text);
        good("ReviewV1", &value);
    }
    rejects(
        "ReviewV1",
        "/reviewer",
        &[
            json!(format!("{}x", "é".repeat(2048))),
            json!("x".repeat(4097)),
        ],
    );
}

#[test]
fn count_requires_u32_integer_domain() {
    let mut value = fixture("CasesV1");
    for count in [0_u64, 4_294_967_295] {
        value["discovered"] = json!(count);
        good("CasesV1", &value);
    }
    rejects(
        "CasesV1",
        "/discovered",
        &[json!(-1), json!(4_294_967_296_u64), json!(1.5), json!("1")],
    );
}

#[test]
fn booleans_do_not_coerce_strings_or_numbers() {
    rejects(
        "BuildProfileV1",
        "/default_features",
        &[json!(0), json!(1), json!(null), json!("false")],
    );
    let mut value = fixture("BuildProfileV1");
    value["default_features"] = json!(false);
    good("BuildProfileV1", &value);
}

#[test]
fn maybe_exactly_one_nonnull_member() {
    rejects(
        "ResourceV1",
        "/value",
        &[
            json!({"value":null,"unavailable_reason":null}),
            json!({"value":"1","unavailable_reason":"unknown"}),
            json!({"value":null}),
            json!({"value":"1","unavailable_reason":null,"guess":true}),
        ],
    );
}

#[test]
fn maybe_empty_reason_and_known_zero_stay_distinct() {
    let mut value = fixture("ResourceV1");
    value["value"] = unavailable("");
    good("ResourceV1", &value);
    value["value"] = available(json!("0"));
    good("ResourceV1", &value);
    let mut review = fixture("ReviewV1");
    review["effective_model"] = available(json!(""));
    good("ReviewV1", &review);
}

#[test]
fn arrays_bound_collection_without_padding_cases() {
    let mut value = fixture("LanguageFlagsV1");
    value["argv"] = json!((0..256).map(|i| format!("arg{i}")).collect::<Vec<_>>());
    good("LanguageFlagsV1", &value);
    value["argv"].as_array_mut().unwrap().push(json!("excess"));
    bad("LanguageFlagsV1", &value);
    value["argv"] = json!([]);
    good("LanguageFlagsV1", &value);
}

#[test]
fn reference_schema_namespace_is_closed() {
    let mut value = fixture("ResourceV1");
    for target in [
        "hee3.raw/1",
        "hee3.receipt/1:ReceiptV1",
        "hee3.receipt/1:CasePageV1",
    ] {
        value["evidence"] = reference(target);
        good("ResourceV1", &value);
    }
    for target in [
        "hee3.raw/2",
        "hee3.receipt/1:MaybeRef",
        "hee3.receipt/1:Id",
        "hee3.receipt/1:Ref",
        "ReceiptV1",
        "hee3.receipt/1:UnknownV1",
    ] {
        value["evidence"] = reference(target);
        bad("ResourceV1", &value);
    }
}

#[test]
fn reference_media_length_and_required_identity() {
    let baseline = fixture("ResourceV1");
    for name in [
        "artifact_id",
        "sha256",
        "byte_length",
        "media_type",
        "schema_id",
    ] {
        let mut value = baseline.clone();
        value["evidence"].as_object_mut().unwrap().remove(name);
        bad("ResourceV1", &value);
    }
    rejects(
        "ResourceV1",
        "/evidence/media_type",
        &[json!(""), json!("é")],
    );
    rejects(
        "ResourceV1",
        "/evidence/byte_length",
        &[json!(4_294_967_296_u64)],
    );
}

#[test]
fn subjects_keep_typed_role_targets() {
    for pointer in [
        "/seed_subject/schema_id",
        "/locks/schema_id",
        "/toolchain/schema_id",
        "/target_features_build_profile/schema_id",
    ] {
        rejects("SubjectsV1", pointer, &[json!("hee3.raw/1")]);
    }
    rejects(
        "SubjectsV1",
        "/locks/schema_id",
        &[json!(schema("ToolPageV1"))],
    );
}

#[test]
fn raw_payloads_cannot_be_typed_metadata_refs() {
    rejects(
        "ProducerV1",
        "/stdout/value/schema_id",
        &[json!(schema("DiagnosticV1"))],
    );
    let mut value = fixture("SubjectV1");
    value["dirty_patch"] = available(reference("hee3.raw/1"));
    good("SubjectV1", &value);
    put(
        &mut value,
        "/dirty_patch/value/schema_id",
        json!(schema("SubjectV1")),
    );
    bad("SubjectV1", &value);
}

#[test]
fn invocation_references_preserve_each_contract_target() {
    for field in [
        "environment",
        "grants",
        "expected",
        "limits",
        "allowed_effects",
        "cleanup_contract",
    ] {
        rejects(
            "InvocationV1",
            &format!("/{field}/schema_id"),
            &[json!(schema("SubjectV1"))],
        );
    }
}

#[test]
fn exited_and_signalled_producer_fields_are_exclusive() {
    rejects("ProducerV1", "/signal", &[available(json!(9))]);
    rejects("ProducerV1", "/exit_code", &[unavailable("unknown")]);
    let mut value = fixture("ProducerV1");
    value["status"] = json!("signalled");
    value["exit_code"] = unavailable("signal");
    value["signal"] = available(json!(9));
    good("ProducerV1", &value);
    value["exit_code"] = available(json!(0));
    bad("ProducerV1", &value);
}

#[test]
fn unstarted_unknown_producers_do_not_invent_exit() {
    for state in ["not_started", "unknown"] {
        let mut value = fixture("ProducerV1");
        value["status"] = json!(state);
        value["exit_code"] = unavailable("not observed");
        value["signal"] = unavailable("not observed");
        good("ProducerV1", &value);
        value["exit_code"] = available(json!(0));
        bad("ProducerV1", &value);
    }
}

#[test]
fn expected_producer_has_no_unknown_execution_mode() {
    rejects(
        "ExpectedProducerV1",
        "/status",
        &[json!("unknown"), json!("not_started")],
    );
    let mut value = fixture("ExpectedProducerV1");
    value["status"] = json!("signalled");
    value["exit_code"] = unavailable("signal expectation");
    value["signal"] = available(json!(15));
    good("ExpectedProducerV1", &value);
}

#[test]
fn pass_requires_result_subject_and_patch() {
    for pointer in ["/subjects/result_subject", "/subjects/seed_to_result_patch"] {
        rejects("ReceiptV1", pointer, &[unavailable("not observed")]);
        let mut value = fixture("ReceiptV1");
        value["verdict"]["state"] = json!("FAIL");
        put(&mut value, pointer, unavailable("not observed"));
        good("ReceiptV1", &value);
    }
}

#[test]
fn pass_requires_exited_producer_but_not_zero_exit() {
    let mut value = fixture("ReceiptV1");
    put(
        &mut value,
        "/observations/producer/exit_code",
        available(json!(1)),
    );
    good("ReceiptV1", &value);
    put(
        &mut value,
        "/observations/producer/status",
        json!("signalled"),
    );
    put(
        &mut value,
        "/observations/producer/exit_code",
        unavailable("signal"),
    );
    put(
        &mut value,
        "/observations/producer/signal",
        available(json!(9)),
    );
    bad("ReceiptV1", &value);
}

#[test]
fn pass_rejects_timeout_cancel_or_unsettled_cleanup() {
    rejects(
        "ReceiptV1",
        "/observations/producer/timeout",
        &[json!(true)],
    );
    rejects(
        "ReceiptV1",
        "/observations/cancellation",
        &[json!("requested"), json!("unknown")],
    );
    rejects(
        "ReceiptV1",
        "/observations/cleanup",
        &[
            json!("not_started"),
            json!("pending"),
            json!("failed"),
            json!("unknown"),
        ],
    );
}

#[test]
fn baseline_pass_rejects_diagnostics_without_overrejecting_faults() {
    rejects("ReceiptV1", "/diagnostics/warning_count", &[json!(1)]);
    rejects("ReceiptV1", "/diagnostics/error_count", &[json!(1)]);
    let mut value = fixture("ReceiptV1");
    value["diagnostics"]["baseline"] = json!(false);
    value["diagnostics"]["warning_count"] = json!(1);
    good("ReceiptV1", &value);
}

#[test]
fn pass_requires_both_available_untruncated_streams() {
    for pointer in [
        "/observations/producer/stdout",
        "/observations/producer/stderr",
    ] {
        rejects("ReceiptV1", pointer, &[unavailable("lost log")]);
    }
    for pointer in [
        "/diagnostics/stdout_truncated",
        "/diagnostics/stderr_truncated",
    ] {
        rejects("ReceiptV1", pointer, &[json!(true)]);
    }
}

#[test]
fn pass_rejects_explicit_diagnostic_mismatch() {
    rejects(
        "ReceiptV1",
        "/diagnostics/mismatch",
        &[available(json!("mismatch")), available(json!(""))],
    );
}

#[test]
fn pass_needs_selected_executed_but_does_not_zero_exhaustive_totals() {
    rejects("ReceiptV1", "/cases/selected", &[json!(0)]);
    rejects("ReceiptV1", "/cases/executed", &[json!(0)]);
    let mut value = fixture("ReceiptV1");
    value["cases"]["skipped"] = json!(1);
    value["cases"]["unmeasured"] = json!(1);
    good("ReceiptV1", &value);
}

#[test]
fn pass_requires_finalized_complete_artifacts() {
    rejects("ReceiptV1", "/artifacts/finalized", &[json!(false)]);
    rejects(
        "ReceiptV1",
        "/availability/state",
        &[json!("missing"), json!("incomplete")],
    );
}

#[test]
fn initial_receipt_cannot_inline_later_review() {
    let mut value = fixture("ReceiptV1");
    value["review"] = available(fixture("ReviewV1"));
    bad("ReceiptV1", &value);
    value["review"] = unavailable("");
    good("ReceiptV1", &value);
}

#[test]
fn append_review_targets_prior_receipt() {
    good("ReviewReceiptV1", &fixture("ReviewReceiptV1"));
    rejects(
        "ReviewReceiptV1",
        "/receipt/schema_id",
        &[json!(schema("ReviewReceiptV1")), json!("hee3.raw/1")],
    );
    rejects("ReviewReceiptV1", "/protocol", &[json!("hee3.receipt")]);
}

#[test]
fn missing_observation_descriptor_is_not_a_resolving_reference() {
    let mut value = fixture("MissingObjectV1");
    value["expected_schema_id"] = json!("retired-schema-name");
    good("MissingObjectV1", &value);
    value["reason"] = json!("");
    bad("MissingObjectV1", &value);
    rejects(
        "AvailabilityReceiptV1",
        "/receipt/schema_id",
        &[json!(schema("AvailabilityReceiptV1"))],
    );
}

#[test]
fn mandatory_case_cannot_be_unselected_or_excluded() {
    rejects("CaseV1", "/selected", &[json!(false)]);
    rejects("CaseV1", "/excluded", &[json!(true)]);
    let mut value = fixture("CaseV1");
    value["mandatory"] = json!(false);
    value["selected"] = json!(false);
    value["executed"] = json!(false);
    value["outcome"] = json!("unmeasured");
    value["reason"] = json!("not selected");
    good("CaseV1", &value);
    value["mandatory"] = json!(true);
    bad("CaseV1", &value);
}

#[test]
fn executed_case_must_have_been_selected() {
    let mut value = fixture("CaseV1");
    value["mandatory"] = json!(false);
    value["selected"] = json!(false);
    value["outcome"] = json!("unmeasured");
    value["reason"] = json!("not selected");
    bad("CaseV1", &value);
    value["executed"] = json!(false);
    good("CaseV1", &value);
}

#[test]
fn exclusion_requires_unmeasured_unexecuted_reason() {
    let mut value = fixture("CaseV1");
    value["mandatory"] = json!(false);
    value["excluded"] = json!(true);
    value["selected"] = json!(false);
    value["executed"] = json!(false);
    value["outcome"] = json!("unmeasured");
    value["reason"] = json!("explicit exclusion");
    good("CaseV1", &value);
    for (field, replacement) in [
        ("selected", json!(true)),
        ("executed", json!(true)),
        ("outcome", json!("passed")),
        ("reason", json!("")),
    ] {
        let mut changed = value.clone();
        changed[field] = replacement;
        bad("CaseV1", &changed);
    }
}

#[test]
fn nonpassed_case_requires_meaningful_reason() {
    for outcome in [
        "failed",
        "skipped",
        "ignored",
        "broken",
        "timeout",
        "invalid",
        "unmeasured",
    ] {
        let mut value = fixture("CaseV1");
        value["outcome"] = json!(outcome);
        value["reason"] = json!("fixed expected adverse observation");
        good("CaseV1", &value);
        value["reason"] = json!("");
        bad("CaseV1", &value);
    }
}

#[test]
fn empty_inventory_has_one_canonical_page() {
    let mut value = fixture("CasePageV1");
    value["row_count"] = json!(0);
    value["total_rows"] = json!(0);
    value["rows"] = json!([]);
    good("CasePageV1", &value);
    for field in ["page_index", "row_count", "total_rows"] {
        let mut changed = value.clone();
        changed[field] = json!(1);
        bad("CasePageV1", &changed);
    }
}

#[test]
fn nonempty_inventory_cannot_contain_empty_page() {
    rejects("CasePageV1", "/rows", &[json!([])]);
    rejects("CasePageV1", "/row_count", &[json!(0)]);
}

#[test]
fn page_counts_and_row_cap_are_bounded() {
    rejects("CasePageV1", "/page_count", &[json!(0)]);
    let mut value = fixture("CasePageV1");
    value["row_count"] = json!(256);
    value["total_rows"] = json!(256);
    value["rows"] = Value::Array(
        (0..256)
            .map(|i| {
                let mut row = fixture("CaseV1");
                row["case_id"] = json!(format!("case-{i}"));
                row
            })
            .collect(),
    );
    good("CasePageV1", &value);
    value["rows"]
        .as_array_mut()
        .unwrap()
        .push(fixture("CaseV1"));
    value["row_count"] = json!(257);
    value["total_rows"] = json!(257);
    bad("CasePageV1", &value);
}

#[test]
fn terminal_page_reason_and_single_page_index_are_fixed() {
    rejects(
        "CasePageV1",
        "/next/unavailable_reason",
        &[json!("missing"), json!("")],
    );
    rejects("CasePageV1", "/page_index", &[json!(1)]);
    let mut value = fixture("CasePageV1");
    value["next"] = available(reference(&schema("CasePageV1")));
    bad("CasePageV1", &value);
    good("CasePageV1", &nonfinal_page());
}

#[test]
fn page_next_reference_is_same_page_type() {
    let mut value = nonfinal_page();
    good("CasePageV1", &value);
    for target in [
        schema("ArtifactPageV1"),
        schema("CaseV1"),
        "hee3.raw/1".to_owned(),
    ] {
        value["next"] = available(reference(&target));
        bad("CasePageV1", &value);
    }
}

#[test]
fn subject_paths_are_descriptive_relative_components() {
    rejects(
        "SubjectFileV1",
        "/path",
        &[
            json!("/root"),
            json!("."),
            json!(".."),
            json!("a/../b"),
            json!("a/./b"),
            json!("a//b"),
            json!("a/"),
            json!("a\0b"),
            json!(""),
        ],
    );
    let mut value = fixture("SubjectFileV1");
    value["path"] = json!("src/λ.rs");
    good("SubjectFileV1", &value);
}

#[test]
fn regular_source_file_requires_content_and_no_link() {
    rejects("SubjectFileV1", "/content", &[unavailable("lost")]);
    rejects(
        "SubjectFileV1",
        "/link_target",
        &[available(json!("another.rs"))],
    );
}

#[test]
fn directory_descriptor_carries_neither_content_nor_link() {
    let mut value = fixture("SubjectFileV1");
    value["kind"] = json!("directory");
    value["content"] = unavailable("directory");
    value["link_target"] = unavailable("not a link");
    value["executable"] = json!(false);
    good("SubjectFileV1", &value);
    value["content"] = available(reference("hee3.raw/1"));
    bad("SubjectFileV1", &value);
}

#[test]
fn symlink_target_and_executable_fact_remain_separate() {
    let mut value = fixture("SubjectFileV1");
    value["kind"] = json!("symlink");
    value["link_target"] = available(json!("neighbor.rs"));
    value["executable"] = json!(false);
    good("SubjectFileV1", &value);
    value["link_target"] = unavailable("missing");
    bad("SubjectFileV1", &value);
    for kind in ["directory", "symlink", "submodule", "other"] {
        let mut changed = fixture("SubjectFileV1");
        changed["kind"] = json!(kind);
        changed["executable"] = json!(false);
        if kind == "directory" {
            changed["content"] = unavailable("directory");
            changed["link_target"] = unavailable("not a link");
        } else if kind == "symlink" {
            changed["link_target"] = available(json!("neighbor.rs"));
        }
        good("SubjectFileV1", &changed);
        changed["executable"] = json!(true);
        bad("SubjectFileV1", &changed);
    }
    value = fixture("SubjectFileV1");
    value["executable"] = json!(true);
    good("SubjectFileV1", &value);
}

#[test]
fn excluded_source_origin_requires_actual_reason() {
    let mut value = fixture("SubjectFileV1");
    value["origin"] = json!("excluded");
    value["exclusion_reason"] = available(json!("outside selected source"));
    good("SubjectFileV1", &value);
    for reason in [unavailable("missing"), available(json!(""))] {
        value["exclusion_reason"] = reason;
        bad("SubjectFileV1", &value);
    }
}

#[test]
fn environment_literal_or_secret_handle_is_exclusive() {
    let mut value = fixture("EnvironmentV1");
    value["value"] = available(json!(""));
    value["secret_handle"] = unavailable("literal");
    good("EnvironmentV1", &value);
    value["secret_handle"] = available(json!(ID));
    bad("EnvironmentV1", &value);
    value["value"] = unavailable("handle");
    good("EnvironmentV1", &value);
    value["secret_handle"] = unavailable("missing");
    bad("EnvironmentV1", &value);
    rejects("EnvironmentV1", "/name", &[json!("A=B"), json!("A\0B")]);
}

#[test]
fn caught_mutant_requires_execution_detector_and_nonempty_diff() {
    rejects("MutantV1", "/executed", &[json!(false)]);
    rejects(
        "MutantV1",
        "/observed_detector",
        &[unavailable("not observed")],
    );
    rejects("MutantV1", "/diff/byte_length", &[json!(0)]);
}

#[test]
fn reviewed_equivalent_mutant_requires_disposition_ref() {
    let mut value = fixture("MutantV1");
    value["outcome"] = json!("reviewed-equivalent");
    value["review_ref"] = available(reference(&schema("ReviewV1")));
    good("MutantV1", &value);
    value["review_ref"] = unavailable("not reviewed");
    bad("MutantV1", &value);
    value["review_ref"] = available(reference(&schema("ReviewReceiptV1")));
    bad("MutantV1", &value);
}

#[test]
fn unviable_unexecuted_mutation_preserves_invalid_diff_inventory() {
    for outcome in ["unviable", "excluded", "unmeasured"] {
        let mut value = fixture("MutantV1");
        value["outcome"] = json!(outcome);
        value["executed"] = json!(false);
        value["diff"]["byte_length"] = json!(0);
        value["reason"] = json!("retained invalid or unexecuted diff");
        good("MutantV1", &value);
        value["reason"] = json!("");
        bad("MutantV1", &value);
    }
}

#[test]
fn campaign_language_and_baseline_targets_are_closed() {
    rejects("CampaignV1", "/language", &[json!("bash"), json!("Rust")]);
    rejects(
        "CampaignV1",
        "/baseline_receipt/schema_id",
        &[json!(schema("ReviewReceiptV1"))],
    );
    rejects(
        "CampaignV1",
        "/mutants/schema_id",
        &[json!(schema("CasePageV1"))],
    );
    let mut value = fixture("CampaignV1");
    value["language"] = json!("julia");
    good("CampaignV1", &value);
}

#[test]
fn finding_disposition_requires_reason_and_bounded_proof_lists() {
    rejects(
        "FindingV1",
        "/disposition",
        &[json!("accepted"), json!("closed")],
    );
    rejects("FindingV1", "/rationale", &[json!("")]);
    let mut value = fixture("FindingV1");
    value["post_fix_evidence"] = json!(vec![reference("hee3.raw/1"); 256]);
    good("FindingV1", &value);
    value["post_fix_evidence"]
        .as_array_mut()
        .unwrap()
        .push(reference("hee3.raw/1"));
    bad("FindingV1", &value);
}

#[test]
fn expectation_and_oracle_result_keep_independent_roles() {
    for class in [
        "contract",
        "reference",
        "round_trip_invariant",
        "metamorphic",
        "differential",
    ] {
        let mut value = fixture("ExpectationV1");
        value["oracle_class"] = json!(class);
        good("ExpectationV1", &value);
    }
    rejects("ExpectationV1", "/expected_oracle", &[json!("accepted")]);
    rejects("ExpectationV1", "/oracle_class", &[json!("self_report")]);
    for outcome in ["satisfied", "violated", "unavailable", "error"] {
        let mut value = fixture("OracleResultV1");
        value["result"] = json!(outcome);
        good("OracleResultV1", &value);
    }
    rejects(
        "OracleResultV1",
        "/expected/schema_id",
        &[json!(schema("OracleResultV1"))],
    );
}

#[test]
fn exact_byte_reference_hash_does_not_canonicalize_key_order() {
    let first = br#"{"language":"rust","argv":[]}"#;
    let second = br#"{"argv":[],"language":"rust"}"#;
    let a = reference_for::<LanguageFlagsV1>(Id::new(ID).unwrap(), first).unwrap();
    let b = reference_for::<LanguageFlagsV1>(Id::new(ID).unwrap(), second).unwrap();
    assert_eq!(
        a.as_ref().sha256.as_str(),
        "sha256:0a7806cb5c40f6a147901616b7738ce49b297ff1e92aa114be78d2bbea02b0bf"
    );
    assert_eq!(
        b.as_ref().sha256.as_str(),
        "sha256:e54e68d66775ea5e90b8c1009407882742c245f0ec366007a1bb24dfaae5a04a"
    );
    assert_eq!(a.as_ref().byte_length, 29);
    assert_eq!(b.as_ref().schema_id.as_str(), schema("LanguageFlagsV1"));
}

#[test]
fn decoder_errors_do_not_echo_rejected_source() {
    let message = |marker: &str| {
        let value = json!({"language":"rust","argv":[],"unknown":marker});
        match check("LanguageFlagsV1", &value) {
            Err(error) => error.to_string(),
            Ok(_) => panic!("unknown field accepted"),
        }
    };
    let a = message("PRIVATE_SYNTHETIC_MARKER_A");
    let b = message("PRIVATE_SYNTHETIC_MARKER_B");
    assert_eq!(a, b);
    assert!(!a.contains("PRIVATE_SYNTHETIC_MARKER"));
}

#[test]
fn declared_records_maybe_and_references_are_maps_not_positional_arrays() {
    assert!(decode_record(&schema("LanguageFlagsV1"), br#"["rust",[]]"#).is_err());
    rejects("ResourceV1", "/value", &[json!([null, "unknown"])]);
    let reference_array = json!([
        ID,
        "sha256:1111111111111111111111111111111111111111111111111111111111111111",
        0,
        "application/octet-stream",
        "hee3.raw/1"
    ]);
    rejects("ResourceV1", "/evidence", &[reference_array]);
    good("ResourceV1", &fixture("ResourceV1"));
}

#[test]
fn closed_enum_strings_do_not_accept_externally_tagged_objects() {
    rejects("CampaignV1", "/language", &[json!({"rust":null})]);
    rejects("ProducerV1", "/status", &[json!({"exited":null})]);
    rejects("CaseV1", "/outcome", &[json!({"passed":null})]);
}
