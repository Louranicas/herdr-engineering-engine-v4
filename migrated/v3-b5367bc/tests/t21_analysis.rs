use habitat_engine::numerical::{Dataset, Invalid, JuliaCode, MAX_REPORT, MAX_REQUEST};
use serde_json::{Value, json};
const NOW: u64 = 1_769_999_995_000;
const RAW: &[u8] = include_bytes!("fixtures/t21/J01.json");
fn fixture() -> Value {
    serde_json::from_slice(RAW).unwrap()
}
fn dataset() -> Dataset {
    Dataset::decode(RAW, NOW).unwrap()
}
fn encode(v: &Value) -> Vec<u8> {
    serde_json::to_vec(v).unwrap()
}
fn reject(mutator: impl FnOnce(&mut Value)) {
    let mut q = fixture();
    mutator(&mut q);
    assert!(Dataset::decode(&encode(&q), NOW).is_err());
}
macro_rules! bad {
    ($name:ident, $pointer:literal, $value:expr) => {
        #[test]
        fn $name() {
            reject(|q| *q.pointer_mut($pointer).unwrap() = $value);
        }
    };
}
bad!(wrong_protocol, "/protocol", json!("hee3.control"));
bad!(wrong_version, "/version", json!(2));
bad!(boolean_version, "/version", json!(true));
bad!(float_version, "/version", json!(1.0));
bad!(invalid_request_id, "/request_id", json!("1"));
bad!(invalid_task_id, "/subject/task_id", json!("1"));
bad!(invalid_subject_attempt, "/subject/attempt_id", json!("1"));
bad!(numeric_generation, "/subject/generation", json!(3));
bad!(leading_generation, "/subject/generation", json!("03"));
bad!(
    overflow_generation,
    "/subject/generation",
    json!("18446744073709551616")
);
bad!(bare_digest, "/subject/artifact_sha256", json!("22"));
bad!(future_cutoff, "/cutoff_unix_ms", json!("1770000000000"));
bad!(expired, "/expires_unix_ms", json!("1769999995000"));
bad!(
    numeric_cutoff,
    "/cutoff_unix_ms",
    json!(1_769_999_990_000_u64)
);
bad!(wrong_recipe, "/recipe/id", json!("regression"));
bad!(wrong_recipe_version, "/recipe/version", json!(2));
bad!(seconds_unit, "/units/elapsed", json!("s"));
bad!(wrong_usage_unit, "/units/usage", json!("tokens"));
bad!(zero_rows, "/shape/rows", json!(0));
bad!(oversized_shape, "/shape/rows", json!(4097));
bad!(mismatched_shape, "/shape/rows", json!(4));
bad!(wrong_fields, "/shape/fields", json!(4));
bad!(boolean_rows, "/shape/rows", json!(true));
bad!(empty_observations, "/observations", json!([]));
bad!(
    duplicate_attempt,
    "/observations/1/attempt_id",
    json!("123e4567-e89b-42d3-a456-000000000021")
);
bad!(unknown_outcome, "/observations/0/outcome", json!("success"));
bad!(negative_elapsed, "/observations/0/elapsed_ms", json!(-1.0));
bad!(
    overflow_elapsed,
    "/observations/0/elapsed_ms",
    json!(86_400_001.0)
);
bad!(boolean_elapsed, "/observations/0/elapsed_ms", json!(true));
bad!(string_elapsed, "/observations/0/elapsed_ms", json!("10"));
bad!(numeric_usage, "/observations/0/usage_tokens", json!(4));
bad!(negative_usage, "/observations/0/usage_tokens", json!("-1"));
bad!(
    overflow_usage,
    "/observations/0/usage_tokens",
    json!("4294967296")
);
bad!(leading_usage, "/observations/0/usage_tokens", json!("04"));
bad!(accepted_censored, "/observations/0/censored", json!(true));
bad!(running_uncensored, "/observations/4/censored", json!(false));
#[test]
fn missing_nullable_usage() {
    reject(|q| {
        q["observations"][0]
            .as_object_mut()
            .unwrap()
            .remove("usage_tokens");
    });
}
#[test]
fn unknown_top_field() {
    reject(|q| q["extra"] = json!(null));
}
#[test]
fn unknown_row_field() {
    reject(|q| q["observations"][0]["extra"] = json!(null));
}
#[test]
fn missing_subject() {
    reject(|q| {
        q.as_object_mut().unwrap().remove("subject");
    });
}
#[test]
fn row_limit_actual() {
    reject(|q| {
        q["observations"] = json!(vec![q["observations"][0].clone(); 4097]);
        q["shape"]["rows"] = json!(4097);
    });
}
fn duplicate(key: &str) {
    let raw = String::from_utf8(RAW.to_vec()).unwrap();
    let replaced = raw.replacen(
        &format!("\"{key}\":"),
        &format!("\"{key}\":null,\"{key}\":"),
        1,
    );
    assert!(Dataset::decode(replaced.as_bytes(), NOW).is_err());
}
#[test]
fn duplicate_top() {
    duplicate("protocol");
}
#[test]
fn duplicate_subject() {
    duplicate("generation");
}
#[test]
fn duplicate_recipe() {
    duplicate("id");
}
#[test]
fn duplicate_units() {
    duplicate("usage");
}
#[test]
fn duplicate_shape() {
    duplicate("rows");
}
#[test]
fn duplicate_row() {
    duplicate("elapsed_ms");
}
#[test]
fn escaped_duplicate() {
    let raw = String::from_utf8(RAW.to_vec()).unwrap().replacen(
        "\"protocol\":",
        "\"pr\\u006ftocol\":null,\"protocol\":",
        1,
    );
    assert!(Dataset::decode(raw.as_bytes(), NOW).is_err());
}
#[test]
fn nonfinite_json() {
    for literal in ["NaN", "Infinity", "1e999"] {
        let raw = String::from_utf8(RAW.to_vec())
            .unwrap()
            .replacen("10.0", literal, 1);
        assert!(Dataset::decode(raw.as_bytes(), NOW).is_err());
    }
}
#[test]
fn multiple_objects() {
    assert!(Dataset::decode(&[RAW, RAW].concat(), NOW).is_err());
}
#[test]
fn incomplete_object() {
    assert!(Dataset::decode(&RAW[..RAW.len() - 1], NOW).is_err());
}
#[test]
fn invalid_utf8() {
    assert!(Dataset::decode(&[b'{', 255, b'}'], NOW).is_err());
}
#[test]
fn excessive_request() {
    assert_eq!(
        Dataset::decode(&vec![b' '; MAX_REQUEST + 1], NOW).unwrap_err(),
        Invalid::Bound
    );
}
#[test]
fn excessive_nesting() {
    assert_eq!(
        Dataset::decode(&[b'['; 33], NOW).unwrap_err(),
        Invalid::Bound
    );
}
#[test]
fn j01_independent_reference() {
    let d = dataset();
    let r = d.reference().unwrap();
    assert_eq!(
        d.digest(),
        "sha256:3aa532ed778498457dd105e0acdf5d01ff4dd2673fa081755f10c7083a2620c7"
    );
    assert_eq!(d.raw(), RAW);
    assert_eq!(r.counts.total, "5");
    assert_eq!(
        [
            r.counts.accepted,
            r.counts.failed,
            r.counts.cancelled,
            r.counts.abandoned,
            r.counts.running
        ],
        ["1"; 5]
    );
    assert_eq!(r.counts.unknown_usage, "3");
    assert_eq!(r.counts.censored, "1");
    assert_eq!(r.counts.known_usage_sum, "4");
    assert!((r.acceptance_fraction - 0.2).abs() < f64::EPSILON);
    assert!((r.mean_observed_ms - 30.0).abs() < f64::EPSILON);
}
#[test]
fn zero_to_unknown() {
    let mut q = fixture();
    q["observations"][2]["usage_tokens"] = Value::Null;
    let r = Dataset::decode(&encode(&q), NOW)
        .unwrap()
        .reference()
        .unwrap();
    assert_eq!(r.counts.unknown_usage, "4");
    assert_eq!(r.counts.known_usage_sum, "4");
}
#[test]
fn unknown_to_zero() {
    let mut q = fixture();
    q["observations"][1]["usage_tokens"] = json!("0");
    let r = Dataset::decode(&encode(&q), NOW)
        .unwrap()
        .reference()
        .unwrap();
    assert_eq!(r.counts.unknown_usage, "2");
    assert_eq!(r.counts.known_usage_sum, "4");
}
#[test]
fn exact_bytes_not_reserialized() {
    let a = dataset();
    let raw = [b" \n".as_slice(), RAW, b"\t"].concat();
    let b = Dataset::decode(&raw, NOW).unwrap();
    assert_eq!(a.request(), b.request());
    assert_ne!(a.digest(), b.digest());
    assert!(
        b.report(&serde_json::to_vec(&a.reference().unwrap()).unwrap(), NOW)
            .is_err()
    );
}
fn report_value() -> Value {
    serde_json::to_value(dataset().reference().unwrap()).unwrap()
}
#[test]
fn report_bound_fields() {
    for pointer in [
        "/request_id",
        "/request_sha256",
        "/subject/task_id",
        "/subject/attempt_id",
        "/subject/generation",
        "/subject/artifact_sha256",
        "/recipe/id",
        "/cutoff_unix_ms",
        "/expires_unix_ms",
        "/units/usage",
    ] {
        let mut r = report_value();
        *r.pointer_mut(pointer).unwrap() = json!("wrong");
        assert!(dataset().report(&encode(&r), NOW).is_err(), "{pointer}");
    }
}
#[test]
fn all_count_fields_verified() {
    for key in [
        "total",
        "accepted",
        "failed",
        "cancelled",
        "abandoned",
        "running",
        "unknown_usage",
        "censored",
        "known_usage_sum",
    ] {
        let mut r = report_value();
        r["counts"][key] = json!("99");
        assert!(dataset().report(&encode(&r), NOW).is_err(), "{key}");
    }
}
#[test]
fn report_stale_at_return() {
    assert_eq!(
        dataset()
            .report(&encode(&report_value()), 1_770_000_000_000)
            .unwrap_err(),
        Invalid::Stale
    );
}
#[test]
fn report_unknown_field() {
    let mut r = report_value();
    r["extra"] = json!(0);
    assert!(dataset().report(&encode(&r), NOW).is_err());
}
#[test]
fn report_duplicate_count() {
    let r = String::from_utf8(encode(&report_value()))
        .unwrap()
        .replacen("\"total\":", "\"total\":\"0\",\"total\":", 1);
    assert!(dataset().report(r.as_bytes(), NOW).is_err());
}
#[test]
fn report_output_limit() {
    assert_eq!(
        dataset()
            .report(&vec![b' '; MAX_REPORT + 1], NOW)
            .unwrap_err(),
        Invalid::Bound
    );
}
#[test]
fn wrong_in_range_mean() {
    let mut r = report_value();
    r["mean_observed_ms"] = json!(20.0);
    assert_eq!(
        dataset().report(&encode(&r), NOW).unwrap_err(),
        Invalid::Statistics
    );
}
#[test]
fn mean_tolerance_adjacent_values() {
    let tolerance = 8.0 * f64::EPSILON * 30.0;
    for direction in [-1.0, 1.0] {
        let mut edge = 30.0 + direction * tolerance;
        while (edge - 30.0).abs() > tolerance {
            edge = if direction > 0.0 {
                edge.next_down()
            } else {
                edge.next_up()
            };
        }
        let mut r = report_value();
        r["mean_observed_ms"] = json!(edge);
        assert!(dataset().report(&encode(&r), NOW).is_ok());
        r["mean_observed_ms"] = json!(if direction > 0.0 {
            edge.next_up()
        } else {
            edge.next_down()
        });
        assert!(dataset().report(&encode(&r), NOW).is_err());
    }
}
#[test]
fn fraction_tolerance_adjacent_values() {
    for direction in [-1.0, 1.0] {
        let mut edge: f64 = 0.2 + direction * 1e-12;
        while (edge - 0.2).abs() > 1e-12 {
            edge = if direction > 0.0 {
                edge.next_down()
            } else {
                edge.next_up()
            };
        }
        let mut r = report_value();
        r["acceptance_fraction"] = json!(edge);
        assert!(dataset().report(&encode(&r), NOW).is_ok());
        r["acceptance_fraction"] = json!(if direction > 0.0 {
            edge.next_up()
        } else {
            edge.next_down()
        });
        assert!(dataset().report(&encode(&r), NOW).is_err());
    }
}
#[test]
fn all_rows_mean_survives_relabel() {
    let mut q = fixture();
    q["observations"][0]["outcome"] = json!("failed");
    let r = Dataset::decode(&encode(&q), NOW)
        .unwrap()
        .reference()
        .unwrap();
    assert!((r.mean_observed_ms - 30.0).abs() < f64::EPSILON);
    assert!(r.acceptance_fraction.abs() < f64::EPSILON);
}
#[test]
fn actual_julia_report() {
    let path =
        std::env::var("T21_JULIA_REPORT").expect("actual Julia report must be supplied; no skip");
    let raw = std::fs::read(path).unwrap();
    let report = dataset().report(&raw, NOW).unwrap();
    assert_eq!(report.counts.total, "5");
    assert!((report.mean_observed_ms - 30.0).abs() < f64::EPSILON);
}

#[test]
fn wire_preserves_float64_neighbors() {
    let value = 0.199_999_999_998_999_98_f64;
    let mut report = report_value();
    report["acceptance_fraction"] = json!(value);
    let raw = encode(&report);
    let decoded: habitat_engine::numerical::Report = serde_json::from_slice(&raw).unwrap();
    assert_eq!(
        decoded.acceptance_fraction.to_bits(),
        value.to_bits(),
        "wire={}",
        String::from_utf8(raw).unwrap()
    );
}

#[test]
fn compensated_full_row_limit() {
    let mut q = fixture();
    let prototype = q["observations"][0].clone();
    let rows: Vec<Value> = (0..4096)
        .map(|index| {
            let mut row = prototype.clone();
            row["attempt_id"] = json!(format!("123e4567-e89b-42d3-a456-{index:012x}"));
            row["elapsed_ms"] = json!(if index == 0 { 86_400_000.0 } else { 1e-8 });
            row
        })
        .collect();
    q["observations"] = json!(rows);
    q["shape"]["rows"] = json!(4096);
    let result = Dataset::decode(&encode(&q), NOW)
        .unwrap()
        .reference()
        .unwrap();
    // Separately grouped exact decimal sum rounded once, unlike sequential rows.
    let expected = (86_400_000.0 + 4095.0 * 1e-8) / 4096.0;
    assert!((result.mean_observed_ms - expected).abs() <= 8.0 * f64::EPSILON * expected);
    assert_eq!(result.counts.total, "4096");
    assert_eq!(result.counts.known_usage_sum, "16384");
}
#[test]
fn maximum_request_whitespace() {
    let mut raw = RAW.to_vec();
    raw.resize(MAX_REQUEST, b' ');
    assert!(Dataset::decode(&raw, NOW).is_ok());
}
#[test]
fn escaped_integer_key_cannot_coerce_float() {
    let raw = String::from_utf8(RAW.to_vec()).unwrap().replacen(
        "\"version\":1",
        "\"ver\\u0073ion\":1.0",
        1,
    );
    assert!(Dataset::decode(raw.as_bytes(), NOW).is_err());
}
#[test]
fn malformed_number_spellings() {
    for value in ["01", "+1", "1.", "1e", "--1"] {
        let raw = String::from_utf8(RAW.to_vec())
            .unwrap()
            .replacen("10.0", value, 1);
        assert!(Dataset::decode(raw.as_bytes(), NOW).is_err(), "{value}");
    }
}

// NUM-01: the fixed Julia entrypoint's bounded error object (julia/bin/analysis.jl).
type Checked = Result<(), Box<dyn std::error::Error>>;
fn julia_error(code: &str) -> Value {
    let q = fixture();
    json!({
        "protocol": "hee3.analysis",
        "version": 1,
        "kind": "error",
        "request_sha256": dataset().digest(),
        "binding": {
            "request_id": q["request_id"],
            "subject": q["subject"],
            "recipe": q["recipe"],
            "cutoff_unix_ms": q["cutoff_unix_ms"],
            "expires_unix_ms": q["expires_unix_ms"],
            "units": q["units"],
        },
        "code": code,
        "diagnostic": "analysis refusal",
    })
}
/// Every code the entrypoint can write, read from the Julia source that raises it:
/// `analyze` (Evaluate.jl) and the entrypoint's own bound. Cohesion.jl is not reachable.
fn entrypoint_codes() -> Result<std::collections::BTreeSet<String>, Box<dyn std::error::Error>> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("julia");
    let mut codes = std::collections::BTreeSet::new();
    for file in ["src/Evaluate.jl", "bin/analysis.jl"] {
        let text = std::fs::read_to_string(root.join(file))?;
        for opener in ["refuse(:", "AnalysisError(:"] {
            for (at, _) in text.match_indices(opener) {
                let rest = &text[at + opener.len()..];
                let end = rest.find(')').ok_or("unterminated refusal site")?;
                let symbol = &rest[..end];
                assert!(
                    !symbol.is_empty()
                        && symbol.bytes().all(|b| b.is_ascii_lowercase() || b == b'_'),
                    "unexpected refusal symbol in {file}: {symbol:?}"
                );
                codes.insert(symbol.to_owned());
            }
        }
    }
    Ok(codes)
}
#[test]
fn julia_refusal_codes_are_exactly_the_entrypoint_world() -> Checked {
    let codes = entrypoint_codes()?;
    let expected = [
        ("bound", JuliaCode::Bound),
        ("domain", JuliaCode::Domain),
        ("duplicate", JuliaCode::Duplicate),
        ("encoding", JuliaCode::Encoding),
        ("identity", JuliaCode::Identity),
        ("schema", JuliaCode::Schema),
        ("stale", JuliaCode::Stale),
    ];
    let named: std::collections::BTreeSet<String> = expected
        .iter()
        .map(|(name, _)| (*name).to_owned())
        .collect();
    assert_eq!(
        codes, named,
        "Julia entrypoint codes differ from the typed set"
    );
    for (name, code) in expected {
        assert_eq!(
            dataset().refusal(&encode(&julia_error(name))),
            Ok(code),
            "{name}"
        );
    }
    Ok(())
}
#[test]
fn refusal_code_outside_entrypoint_world_is_not_typed() {
    // Cohesion-only codes and a spelling variant are not written by bin/analysis.jl.
    for code in ["join", "conservation", "Stale", "", "unknown"] {
        assert_eq!(
            dataset().refusal(&encode(&julia_error(code))),
            Err(Invalid::Encoding),
            "{code}"
        );
    }
}
#[test]
fn refusal_without_available_binding_is_typed() {
    let mut e = julia_error("encoding");
    e["binding"] = Value::Null;
    assert_eq!(dataset().refusal(&encode(&e)), Ok(JuliaCode::Encoding));
}
#[test]
fn refusal_for_another_request_digest_is_refused() {
    for digest in [
        json!("sha256:ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff"),
        Value::Null,
    ] {
        let mut e = julia_error("stale");
        e["request_sha256"] = digest.clone();
        assert_eq!(
            dataset().refusal(&encode(&e)),
            Err(Invalid::Binding),
            "{digest}"
        );
    }
}
#[test]
fn refusal_echoing_another_binding_is_refused() -> Checked {
    for pointer in [
        "/binding/request_id",
        "/binding/subject/task_id",
        "/binding/subject/attempt_id",
        "/binding/subject/generation",
        "/binding/subject/artifact_sha256",
        "/binding/recipe/id",
        "/binding/cutoff_unix_ms",
        "/binding/expires_unix_ms",
        "/binding/units/elapsed",
        "/binding/units/usage",
    ] {
        let mut e = julia_error("stale");
        *e.pointer_mut(pointer).ok_or(pointer)? = json!("wrong");
        assert_eq!(
            dataset().refusal(&encode(&e)),
            Err(Invalid::Binding),
            "{pointer}"
        );
    }
    let mut e = julia_error("stale");
    e["binding"]["recipe"]["version"] = json!(2);
    assert_eq!(dataset().refusal(&encode(&e)), Err(Invalid::Binding));
    Ok(())
}
#[test]
fn refusal_envelope_is_fixed() {
    for (key, value) in [
        ("protocol", json!("hee3.control")),
        ("version", json!(2)),
        ("kind", json!("report")),
        ("diagnostic", json!("other")),
    ] {
        let mut e = julia_error("stale");
        e[key] = value;
        assert_eq!(
            dataset().refusal(&encode(&e)),
            Err(Invalid::Schema),
            "{key}"
        );
    }
}
#[test]
fn refusal_fields_are_closed_and_required() {
    let mut extra = julia_error("stale");
    extra["detail"] = json!("x");
    assert_eq!(dataset().refusal(&encode(&extra)), Err(Invalid::Encoding));
    let mut extra = julia_error("stale");
    extra["binding"]["shape"] = json!({"rows": 5, "fields": 5});
    assert_eq!(dataset().refusal(&encode(&extra)), Err(Invalid::Encoding));
    for key in ["request_sha256", "binding", "code", "diagnostic", "kind"] {
        let mut e = julia_error("stale");
        e.as_object_mut().map(|o| o.remove(key));
        assert_eq!(
            dataset().refusal(&encode(&e)),
            Err(Invalid::Encoding),
            "{key}"
        );
    }
}
#[test]
fn success_report_is_not_a_refusal() {
    assert_eq!(
        dataset().refusal(&encode(&report_value())),
        Err(Invalid::Encoding)
    );
}
#[test]
fn refusal_is_bounded_before_decoding() {
    let mut raw = encode(&julia_error("stale"));
    raw.resize(MAX_REPORT, b' ');
    assert_eq!(dataset().refusal(&raw), Ok(JuliaCode::Stale));
    raw.push(b' ');
    assert_eq!(dataset().refusal(&raw), Err(Invalid::Bound));
}
