#!/usr/bin/env python3
"""RC04 structural-schema checks; no collector, artifact import or module credit.

Run: python3 -W error tests/receipt_schema.py
jsonschema is installed test tooling only. These tests and the schema share an
author; expected fixtures were hand-authored from RC04, not schema introspection.
Independent oracle authorship, runtime inventory closure and collector admission are not
claimed. A machine summary is printed to stdout; unittest diagnostics use stderr.
"""

import copy
import hashlib
import importlib.metadata
import json
import sys
import unittest
from pathlib import Path

from jsonschema import Draft202012Validator


ROOT = Path(__file__).resolve().parents[1]
SCHEMA_PATH = ROOT / "schemas/receipts/receipt-v1.schema.json"
GENERATOR_PATH = ROOT / "schemas/receipts/generate_receipt_schema.py"
FIXTURES = ROOT / "tests/fixtures/receipts"
SCHEMA = json.loads(SCHEMA_PATH.read_bytes())
EXAMPLES = json.loads((FIXTURES / "receipt-examples.json").read_bytes())
OVERRIDES = json.loads((FIXTURES / "invalid-overrides.json").read_bytes())
INVENTORIES = json.loads((FIXTURES / "inventory-examples.json").read_bytes())
BASE = EXAMPLES["examples"][0]["receipt"]
VALIDATOR = Draft202012Validator(SCHEMA)
ID = "123e4567-e89b-42d3-a456-000000000001"
RUNTIME_GAPS = (
    "Typed inventory reference resolution, complete chains and exact count arithmetic.",
    "Exact byte serialization, duplicate keys, UTF-8 byte limits and numeric tokens.",
    "Protected artifact resolution, digest/length/profile equality and source custody.",
    "Complete inventory arithmetic, mandatory selection and one primary owner.",
    "Independent oracle truth, expected exit/diagnostics and real case sufficiency.",
    "Cleanup/cancellation observations, unresolved effects, deadlines and resource truth.",
    "Separate review, retention availability and independent module admission.",
)


def at(value, path):
    for name in path:
        value = value[name]
    return value


def replaced(value, path, replacement):
    result = copy.deepcopy(value)
    at(result, path[:-1])[path[-1]] = copy.deepcopy(replacement)
    return result


def available(value):
    return {"value": copy.deepcopy(value), "unavailable_reason": None}


def unavailable(reason="Synthetic unavailable fixture value."):
    return {"value": None, "unavailable_reason": reason}


def object_paths(value, path=()):
    if isinstance(value, dict):
        yield path, value
        for name, child in value.items():
            yield from object_paths(child, (*path, name))
    elif isinstance(value, list):
        for index, child in enumerate(value):
            yield from object_paths(child, (*path, index))


class ReceiptSchemaTests(unittest.TestCase):
    def valid(self, value):
        errors = list(VALIDATOR.iter_errors(value))
        self.assertFalse(errors, "\n".join(str(error) for error in errors[:3]))

    def invalid(self, value):
        self.assertTrue(list(VALIDATOR.iter_errors(value)), "Malformed receipt accepted")

    def scalar(self, name, value, expected):
        validator = Draft202012Validator({
            "$ref": "#/$defs/" + name, "$defs": SCHEMA["$defs"],
        })
        self.assertEqual(validator.is_valid(value), expected, (name, repr(value)))

    def test_schema_meta_validation_and_explicit_structural_bundle(self):
        Draft202012Validator.check_schema(SCHEMA)
        self.assertTrue(SCHEMA["x-hee3-bundle-complete"])
        self.assertEqual(SCHEMA["x-hee3-open-decisions"], [])
        self.assertTrue(SCHEMA["x-hee3-runtime-obligations"])

    def test_full_hand_authored_receipt_fixtures(self):
        self.assertEqual(len(EXAMPLES["examples"]), 6)
        for fixture in EXAMPLES["examples"]:
            with self.subTest(fixture=fixture["id"]):
                self.assertEqual(VALIDATOR.is_valid(fixture["receipt"]), fixture["schema_valid"])
        self.assertIn("fictional", EXAMPLES["fixture_notice"])
        self.assertIn("Module case credits: 0", EXAMPLES["fixture_notice"])

    def test_explicit_contract_negative_overrides(self):
        for fixture in OVERRIDES["overrides"]:
            with self.subTest(fixture=fixture["id"]):
                self.assertFalse(fixture["schema_valid"])
                self.invalid(replaced(BASE, fixture["path"], fixture["value"]))

    def test_every_present_object_is_closed_and_every_field_required(self):
        for receipt in (BASE,):
            for path, value in object_paths(receipt):
                with self.subTest(path=path, change="unknown"):
                    changed = copy.deepcopy(receipt)
                    at(changed, path)["unexpected_field"] = None
                    self.invalid(changed)
                for key in value:
                    with self.subTest(path=path, missing=key):
                        changed = copy.deepcopy(receipt)
                        del at(changed, path)[key]
                        self.invalid(changed)

    def test_maybe_exactly_one_nonnull_and_inner_type(self):
        values = {"Id": ID, "Count": 0, "Ref": BASE["subjects"]["seed_subject"],
                  "Text": "", "ReviewV1": EXAMPLES["examples"][3]["receipt"]["review"]["value"]}
        for name, value in values.items():
            with self.subTest(inner=name):
                self.scalar("Maybe" + name, available(value), True)
                self.scalar("Maybe" + name, unavailable(), True)
                self.scalar("Maybe" + name, unavailable(""), True)
                self.scalar("Maybe" + name, {"value": None, "unavailable_reason": None}, False)
                self.scalar("Maybe" + name, {"value": value, "unavailable_reason": "both"}, False)
                self.scalar("Maybe" + name, {"value": False, "unavailable_reason": None}, False)
                self.scalar("Maybe" + name, None, False)

    def test_producer_status_exit_signal_correlations(self):
        receipt = copy.deepcopy(BASE)
        receipt["verdict"]["state"] = "FAIL"
        path = ["observations", "producer"]
        for status in ("exited", "signalled", "not_started", "unknown"):
            for exit_present in (True, False):
                for signal_present in (True, False):
                    producer = copy.deepcopy(at(receipt, path))
                    producer.update(
                        status=status,
                        exit_code=available(0) if exit_present else unavailable(),
                        signal=available(15) if signal_present else unavailable(),
                    )
                    expected = (exit_present, signal_present) == {
                        "exited": (True, False), "signalled": (False, True),
                        "not_started": (False, False), "unknown": (False, False),
                    }[status]
                    with self.subTest(status=status, exit=exit_present, signal=signal_present):
                        self.assertEqual(VALIDATOR.is_valid(replaced(receipt, path, producer)), expected)
        self.invalid(replaced(receipt, path + ["status"], "SIGTERM"))

    def test_all_nonpass_states_represent_incomplete_observations(self):
        for state in ("FAIL", "INVALID", "ERROR", "TIMEOUT", "CANCELLED", "UNMEASURED"):
            with self.subTest(state=state):
                changed = copy.deepcopy(BASE)
                changed["verdict"]["state"] = state
                changed["subjects"]["result_subject"] = unavailable()
                changed["subjects"]["seed_to_result_patch"] = unavailable()
                changed["cases"]["selected"] = 0
                changed["cases"]["executed"] = 0
                changed["diagnostics"]["warning_count"] = 1
                changed["diagnostics"]["stdout_truncated"] = True
                changed["artifacts"]["finalized"] = False
                changed["availability"]["state"] = "missing"
                changed["observations"]["producer"].update(
                    status="not_started", exit_code=unavailable(), signal=unavailable(),
                    stdout=unavailable(), stderr=unavailable(),
                )
                self.valid(changed)

    def test_negative_control_nonzero_exit_and_isolated_diagnostics(self):
        # Shape cannot decide whether the independently fixed expectation holds.
        control = copy.deepcopy(BASE)
        control["diagnostics"].update(baseline=False, warning_count=1, error_count=1)
        control["observations"]["producer"]["exit_code"] = available(17)
        self.valid(control)
        control["diagnostics"]["baseline"] = True
        self.invalid(control)

    def test_pass_refuses_nondecisive_producer_and_defers_row_selection(self):
        # Exhaustive totals can include unselected or nonmandatory rows. Runtime
        # must refuse any selected mandatory nonpass; root totals cannot decide it.
        for field in ("failed", "skipped", "ignored", "broken", "timed_out", "invalid", "unmeasured"):
            with self.subTest(field=field):
                self.valid(replaced(BASE, ["cases", field], 1))
        for status in ("signalled", "not_started", "unknown"):
            producer = copy.deepcopy(BASE["observations"]["producer"])
            producer.update(status=status, exit_code=unavailable(),
                            signal=available(15) if status == "signalled" else unavailable())
            self.invalid(replaced(BASE, ["observations", "producer"], producer))

    def test_exact_u64_lexemes_and_generation(self):
        # Hand-fixed arithmetic boundaries, not the schema's regex construction.
        for value in ("0", "1", "9", "10", "18446744073709551614", "18446744073709551615"):
            self.scalar("U64", value, True)
        for value in ("", "00", "01", "+1", "-1", "1.0", "1e0", " 1", "1 ",
                      "1\n", "1\r", "１", "١", "18446744073709551616",
                      "99999999999999999999", "100000000000000000000", 1, True, None):
            self.scalar("U64", value, False)
        self.scalar("Generation", "0", False)
        self.scalar("Generation", "1", True)
        self.scalar("Generation", "18446744073709551615", True)

    def test_ids_digest_count_and_boolean_boundaries(self):
        self.scalar("Id", ID, True)
        for value in (ID.upper(), ID + "\n", ID.replace("42d3", "12d3"),
                      ID.replace("a456", "7456"), "00000000-0000-0000-0000-000000000000"):
            self.scalar("Id", value, False)
        for suffix, expected in (("a" * 64, True), ("0" * 64, True), ("a" * 63, False),
                                 ("a" * 65, False), ("A" * 64, False), ("a" * 64 + "\n", False)):
            self.scalar("Sha", "sha256:" + suffix, expected)
        for value in (0, 1, 4294967295):
            self.scalar("Count", value, True)
        for value in (-1, 4294967296, True, False, "1", 0.5, None):
            self.scalar("Count", value, False)
        self.scalar("Bool", False, True)
        self.scalar("Bool", True, True)
        self.scalar("Bool", 1, False)

    def test_text_name_and_array_bounds(self):
        for value, expected in (("", True), ("a" * 4096, True), ("a" * 4097, False),
                                ("é", True), ("🦀", True), ("\n", True)):
            self.scalar("Text", value, expected)
        for value, expected in (("a", True), ("a" * 128, True), ("a" * 129, False),
                                ("", False), ("é", False), ("\n", True), ("\x00", True)):
            self.scalar("Name", value, expected)
        for path, item in (
            (["identity", "criterion_ids"], "G04"), (["invocation", "argv"], "arg"),
            (["verdict", "reasons"], "reason"),
        ):
            for length, expected in ((0, True), (256, True), (257, False)):
                with self.subTest(path=path, length=length):
                    self.assertEqual(VALIDATOR.is_valid(replaced(BASE, path, [item] * length)), expected)
            self.invalid(replaced(BASE, path, [False]))
        review = INVENTORIES["definitions"]["ReviewV1"]
        for field in ("pre_fix_evidence", "post_fix_evidence"):
            for length, expected in ((0, True), (256, True), (257, False)):
                self.scalar("ReviewV1", replaced(review, [field], [BASE["subjects"]["seed_subject"]] * length), expected)

    def test_ref_is_closed_but_does_not_resolve_artifacts(self):
        value = BASE["subjects"]["seed_subject"]
        self.scalar("Ref", value, True)
        for field in value:
            broken = copy.deepcopy(value)
            del broken[field]
            self.scalar("Ref", broken, False)
        self.scalar("Ref", {**value, "path": "/candidate/forged"}, False)
        self.scalar("Ref", {**value, "byte_length": 4294967295}, True)
        self.scalar("Ref", {**value, "byte_length": 4294967296}, False)
        # A shape-valid digest is deliberately not proof of actual object bytes.
        self.valid(replaced(BASE, ["schema_sha256"], "sha256:" + "2" * 64))

    def test_runtime_gaps_remain_visible(self):
        # JSON Schema counts code points: this exceeds RC04's encoded byte cap.
        too_many_bytes = "é" * 4096
        self.assertEqual(len(too_many_bytes.encode("utf-8")), 8192)
        self.scalar("Text", too_many_bytes, True)
        self.scalar("Text", "\ud800", True)
        self.scalar("Count", json.loads("1.0"), True)
        self.scalar("Count", json.loads("1e0"), True)
        self.assertEqual(json.loads('{"x":1,"x":2}'), {"x": 2})
        for path, value in (
            (["cases", "executed"], 7), (["cases", "primary_credit"], 200),
            (["observations", "end_monotonic_ns"], "0"),
            (["observations", "producer", "exit_code"], available(17)),
        ):
            with self.subTest(gap=path):
                self.valid(replaced(BASE, path, value))

    def test_bootstrap_specs_are_unexecuted_expectations_with_benign_pairs(self):
        specs = json.loads((FIXTURES / "bootstrap-specs.json").read_bytes())
        self.assertEqual(specs["collector_executions"], 0)
        self.assertEqual(specs["module_case_credits"], 0)
        self.assertFalse(specs["collector_admitted"])
        self.assertIn("fictional", specs["fixture_notice"])
        ids = [item["id"] for item in specs["pairs"]]
        self.assertEqual(len(ids), len(set(ids)))
        required = {
            "known-fail", "fabricated-pass-exit-zero", "wrong-candidate-hash",
            "changed-fixture", "changed-oracle", "missing-log", "truncated-log",
            "zero-selected", "zero-executed", "required-skip", "producer-signal",
            "verifier-crash", "timeout", "post-check-mutation", "failed-cleanup",
        }
        self.assertEqual(set(ids), required)
        expected_states = {
            "known-fail": "FAIL", "fabricated-pass-exit-zero": "FAIL",
            "wrong-candidate-hash": "INVALID", "changed-fixture": "INVALID",
            "changed-oracle": "INVALID", "post-check-mutation": "INVALID",
            "missing-log": "INVALID", "truncated-log": "INVALID",
            "zero-selected": "INVALID", "zero-executed": "INVALID",
            "required-skip": "INVALID", "failed-cleanup": "INVALID",
            "producer-signal": "ERROR", "verifier-crash": "ERROR", "timeout": "TIMEOUT",
        }
        for pair in specs["pairs"]:
            with self.subTest(pair=pair["id"]):
                self.assertFalse(pair["fault"]["qualification_eligible"])
                self.assertTrue(pair["benign"]["qualification_eligible"])
                self.assertTrue(pair["fault"]["intended_reason"])
                self.assertTrue(pair["benign"]["difference"])
                self.assertFalse(pair["executed"])
                self.assertEqual(pair["fault"]["expected_verdict_state"], expected_states[pair["id"]])
                self.assertEqual(pair["benign"]["expected_verdict_state"], "PASS_CANDIDATE")

    def test_all_61_hand_authored_addressable_record_and_page_fixtures(self):
        definitions = INVENTORIES["definitions"]
        self.assertEqual(len(definitions), 61)
        for name, value in definitions.items():
            with self.subTest(definition=name):
                self.scalar(name, value, True)
                self.scalar(name, {**value, "unknown": None}, False)
                for field in value:
                    reduced = copy.deepcopy(value)
                    del reduced[field]
                    self.scalar(name, reduced, False)
                    self.scalar(name, {**value, field: None}, False)
        registered = set(SCHEMA["$defs"]["Ref"]["properties"]["schema_id"]["allOf"][1]["enum"])
        self.assertEqual(registered, {"hee3.raw/1", *("hee3.receipt/1:" + name for name in definitions)})
        for unknown in ("fictional-unknown-schema", "hee3.receipt/1:MaybeId", "hee3.receipt/1:U64"):
            self.scalar("Ref", {**BASE["subjects"]["seed_subject"], "schema_id": unknown}, False)

    def test_typed_reference_targets_and_raw_payloads(self):
        for path in (
            ["subjects", "seed_subject"], ["subjects", "result_subject", "value"],
            ["invocation", "expected"], ["cases", "inventory"],
        ):
            self.invalid(replaced(BASE, path + ["schema_id"], "hee3.raw/1"))
        for path in (
            ["subjects", "seed_to_result_patch", "value"],
            ["subjects", "isolation_profile"], ["observations", "producer", "stdout", "value"],
        ):
            self.invalid(replaced(BASE, path + ["schema_id"], "hee3.receipt/1:SubjectV1"))
        case = INVENTORIES["definitions"]["CaseV1"]
        self.scalar("CaseV1", replaced(case, ["expected", "schema_id"], "hee3.receipt/1:HostV1"), False)
        expected = INVENTORIES["definitions"]["ExpectedProducerV1"]
        self.scalar("ExpectedProducerV1", replaced(expected, ["signal"], available(15)), False)
        signal = {"status": "signalled", "exit_code": unavailable(), "signal": available(15)}
        self.scalar("ExpectedProducerV1", signal, True)

    def test_case_mandatory_selection_exclusion_and_reason_guards(self):
        case = INVENTORIES["definitions"]["CaseV1"]
        for field, value in (("selected", False), ("excluded", True), ("outcome", "excluded")):
            self.scalar("CaseV1", replaced(case, [field], value), False)
        for outcome in ("failed", "skipped", "ignored", "broken", "timeout", "invalid", "unmeasured"):
            row = {**case, "outcome": outcome, "reason": "Declared fixture nonpass reason."}
            self.scalar("CaseV1", row, True)
            self.scalar("CaseV1", {**row, "reason": ""}, False)
        excluded = {**case, "mandatory": False, "excluded": True, "selected": False,
                    "executed": False, "outcome": "unmeasured", "reason": "Predeclared exclusion."}
        self.scalar("CaseV1", excluded, True)
        self.scalar("CaseV1", {**excluded, "mandatory": True}, False)
        self.scalar("CaseV1", {**excluded, "executed": True}, False)
        self.scalar("CaseV1", {**excluded, "outcome": "passed"}, False)
        self.scalar("CaseV1", {**excluded, "reason": ""}, False)
        self.scalar("CaseV1", {**excluded, "excluded": False}, True)
        # A required skip is a representable observation, but can never qualify
        # PASS after the trusted collector resolves and inspects the inventory.
        self.scalar("CaseV1", {**case, "executed": False, "outcome": "skipped", "reason": "Required fixture skipped."}, True)

    def test_all_page_caps_empty_inventory_and_link_type(self):
        for name, page in INVENTORIES["definitions"].items():
            if not name.endswith("PageV1"):
                continue
            with self.subTest(page=name):
                for length, expected in ((256, True), (257, False)):
                    row = {**page, "rows": page["rows"] * length, "row_count": length, "total_rows": length}
                    self.scalar(name, row, expected)
                empty = {**page, "rows": [], "row_count": 0, "total_rows": 0}
                self.scalar(name, empty, True)
                self.scalar(name, {**empty, "page_count": 2}, False)
                self.scalar(name, {**empty, "rows": page["rows"]}, False)
                self.scalar(name, {**page, "page_count": 0}, False)
                self.scalar(name, {**page, "rows": []}, False)
                self.scalar(name, {**page, "row_count": 0}, False)
                self.scalar(name, {**page, "next": unavailable("other reason")}, False)
        page = INVENTORIES["definitions"]["CasePageV1"]
        next_ref = {**BASE["cases"]["inventory"], "schema_id": "hee3.receipt/1:CasePageV1"}
        first = {**page, "page_count": 2, "total_rows": 2, "next": available(next_ref)}
        final = {**page, "page_index": 1, "page_count": 2, "total_rows": 2}
        self.scalar("CasePageV1", first, True)
        self.scalar("CasePageV1", final, True)
        self.scalar("CasePageV1", replaced(first, ["next", "value", "schema_id"], "hee3.receipt/1:ToolPageV1"), False)
        # Cross-page arithmetic, cycles and repeated row identities need actual
        # protected traversal. These deliberate shapes do not prove a valid chain.
        self.scalar("CasePageV1", {**final, "page_index": 9}, True)
        self.scalar("CasePageV1", {**page, "row_count": 2}, True)

    def test_relative_paths_subject_kinds_and_excluded_identity(self):
        for value in ("src/file.rs", "Cargo.lock", "é/🦀", "a/.hidden", ".\n", "a/\n"):
            self.scalar("RelPath", value, True)
        for value in ("", "/abs", "a/", "a//b", ".", "..", "a/../b", "a/./b", "a\x00b"):
            self.scalar("RelPath", value, False)
        row = INVENTORIES["definitions"]["SubjectFileV1"]
        self.scalar("SubjectFileV1", {**row, "content": unavailable()}, False)
        self.scalar("SubjectFileV1", {**row, "link_target": available("elsewhere")}, False)
        directory = {**row, "kind": "directory", "content": unavailable()}
        self.scalar("SubjectFileV1", directory, True)
        self.scalar("SubjectFileV1", {**directory, "content": row["content"]}, False)
        symlink = {**directory, "kind": "symlink", "link_target": available("../outside")}
        self.scalar("SubjectFileV1", symlink, True)  # Recorded, never followed or admitted here.
        self.scalar("SubjectFileV1", {**symlink, "link_target": unavailable()}, False)
        self.scalar("SubjectFileV1", {**row, "origin": "excluded"}, False)
        self.scalar("SubjectFileV1", {**row, "origin": "excluded", "exclusion_reason": available("Policy exclusion.")}, True)

    def test_executable_is_only_valid_for_regular_files(self):
        row = INVENTORIES["definitions"]["SubjectFileV1"]
        self.scalar("SubjectFileV1", {**row, "executable": True}, True)
        directory = {**row, "kind": "directory", "content": unavailable()}
        self.scalar("SubjectFileV1", directory, True)
        self.scalar("SubjectFileV1", {**directory, "executable": True}, False)
        for kind in ("symlink", "submodule", "other"):
            candidate = {**directory, "kind": kind}
            if kind == "symlink":
                candidate["link_target"] = available("fixture-target")
            self.scalar("SubjectFileV1", candidate, True)
            self.scalar("SubjectFileV1", {**candidate, "executable": True}, False)

    def test_environment_mutant_and_append_semantics(self):
        environment = INVENTORIES["definitions"]["EnvironmentV1"]
        self.scalar("EnvironmentV1", {**environment, "secret_handle": available(ID)}, False)
        self.scalar("EnvironmentV1", {**environment, "value": unavailable()}, False)
        self.scalar("EnvironmentV1", {**environment, "value": unavailable(), "secret_handle": available(ID)}, True)
        for name in ("BAD=NAME", "BAD\x00NAME"):
            self.scalar("EnvironmentV1", {**environment, "name": name}, False)
        mutant = INVENTORIES["definitions"]["MutantV1"]
        self.scalar("MutantV1", {**mutant, "executed": False}, False)
        self.scalar("MutantV1", {**mutant, "observed_detector": unavailable()}, False)
        self.scalar("MutantV1", {**mutant, "outcome": "reviewed-equivalent"}, False)
        review_ref = {**BASE["subjects"]["seed_subject"], "schema_id": "hee3.receipt/1:ReviewV1"}
        self.scalar("MutantV1", {**mutant, "outcome": "reviewed-equivalent", "review_ref": available(review_ref)}, True)
        self.scalar("MutantV1", {**mutant, "outcome": "survived", "reason": ""}, False)
        self.invalid(replaced(BASE, ["review"], available(INVENTORIES["definitions"]["ReviewV1"])))
        for name in ("ReviewReceiptV1", "AvailabilityReceiptV1"):
            append = INVENTORIES["definitions"][name]
            self.scalar(name, append, True)
            self.scalar(name, replaced(append, ["receipt", "schema_id"], "hee3.raw/1"), False)
        missing = INVENTORIES["definitions"]["MissingObjectV1"]
        self.scalar("MissingObjectV1", {**missing, "object": BASE["subjects"]["seed_subject"]}, False)

    def test_decisive_cleanup_and_cancellation_spellings(self):
        for value in ("not_started", "pending", "failed", "unknown", "arbitrary"):
            self.invalid(replaced(BASE, ["observations", "cleanup"], value))
        for value in ("requested", "unknown", "arbitrary"):
            self.invalid(replaced(BASE, ["observations", "cancellation"], value))
        failure = replaced(BASE, ["verdict", "state"], "FAIL")
        for value in ("not_started", "pending", "settled", "failed", "unknown"):
            self.valid(replaced(failure, ["observations", "cleanup"], value))
        for value in ("not_requested", "requested", "unknown"):
            self.valid(replaced(failure, ["observations", "cancellation"], value))

    def test_inventory_enum_fields_reject_unknown_and_wrong_type(self):
        fields = {
            "CaseV1": ("outcome",), "SubjectFileV1": ("kind", "origin"),
            "ExpectationV1": ("oracle_class", "expected_oracle"),
            "ExpectedProducerV1": ("status",), "ObligationV1": ("state",),
            "ArtifactV1": ("availability",), "CampaignV1": ("language",),
            "MutantV1": ("outcome",), "FindingV1": ("disposition",),
            "OracleResultV1": ("result",),
        }
        for name, names in fields.items():
            for field in names:
                for bad in ("unsupported-enum-value", False):
                    with self.subTest(definition=name, field=field, value=bad):
                        self.scalar(name, replaced(INVENTORIES["definitions"][name], [field], bad), False)
        mutant = INVENTORIES["definitions"]["MutantV1"]
        self.scalar("MutantV1", {**mutant, "outcome": "unviable", "reason": ""}, False)
        empty_diff = {**mutant["diff"], "byte_length": 0}
        for outcome in ("caught", "survived", "timeout", "reviewed-equivalent"):
            row = {**mutant, "outcome": outcome, "diff": empty_diff}
            if outcome == "reviewed-equivalent":
                row["review_ref"] = available({**BASE["subjects"]["seed_subject"], "schema_id": "hee3.receipt/1:ReviewV1"})
            self.scalar("MutantV1", row, False)
        for outcome in ("unviable", "excluded", "unmeasured"):
            row = {**mutant, "outcome": outcome, "diff": empty_diff, "reason": "Retained empty/unattempted fixture diff."}
            self.scalar("MutantV1", row, True)
            self.scalar("MutantV1", {**row, "reason": ""}, False)


if __name__ == "__main__":
    suite = unittest.defaultTestLoader.loadTestsFromTestCase(ReceiptSchemaTests)
    result = unittest.TextTestRunner(verbosity=2).run(suite)
    print(json.dumps({
        "kind": "receipt-schema-development-check", "protected_collector": False,
        "independent_oracle_authorship": False, "module_case_credits": 0,
        "tests_run": result.testsRun, "failures": len(result.failures),
        "errors": len(result.errors), "skipped": len(result.skipped),
        "successful": result.wasSuccessful(), "runtime_gaps": list(RUNTIME_GAPS),
        "python": sys.version.split()[0],
        "jsonschema": importlib.metadata.version("jsonschema"),
        "sha256": {str(path.relative_to(ROOT)): hashlib.sha256(path.read_bytes()).hexdigest()
                   for path in (Path(__file__).resolve(), GENERATOR_PATH, SCHEMA_PATH,
                                *sorted(FIXTURES.glob("*.json")))},
    }, sort_keys=True))
    sys.exit(0 if result.wasSuccessful() else 1)
