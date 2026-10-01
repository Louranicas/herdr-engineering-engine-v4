#!/usr/bin/env python3
"""RC03 structural-schema tests using hand-authored contract fixtures.

Run: python3 -W error tests/control_schema.py
Dependency: installed jsonschema (test tooling only). The schema author also
implemented these tests; independent oracle authorship is NOT claimed. Tests,
subtests, field permutations and fixture counts confer no module case credits.
No transport, peer authorization, dispatch, collector or admission is tested.
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
SCHEMA_PATH = ROOT / "schemas/actions/control-v1.schema.json"
FIXTURES = ROOT / "tests/fixtures/native/control-v1"
ACTIONS = (
    "tools.list", "tools.inspect", "task.preview", "task.submit", "task.get",
    "task.list", "task.cancel", "task.resolve", "thread.get", "thread.list",
    "roster.list", "roster.inspect", "roster.update", "roster.disable",
    "service.inspect", "service.probe", "service.action", "analysis.request",
    "analysis.get", "events.subscribe", "health",
)
ERROR_CODES = (
    "invalid_frame", "unsupported_protocol", "unsupported_version", "unknown_action",
    "unsupported_action_version", "unauthenticated", "forbidden", "invalid_argument",
    "not_found", "conflict", "stale_generation", "deadline_exceeded", "cancelled",
    "resource_exhausted", "unavailable", "resync_required", "effect_unknown", "internal",
)
UUID = "123e4567-e89b-42d3-a456-000000000004"
OTHER_UUID = "123e4567-e89b-42d3-a456-000000000099"
READ_ACTIONS = (
    "tools.list", "tools.inspect", "task.get", "task.list", "thread.get",
    "thread.list", "roster.list", "roster.inspect", "service.inspect",
    "analysis.get", "health",
)
EFFECTFUL = (
    "task.submit", "task.cancel", "task.resolve", "roster.update",
    "roster.disable", "service.probe", "service.action", "analysis.request",
)
RUNTIME_GAPS = (
    "Exact JSON numeric token spelling after parsing (1.0/1e0 versus 1).",
    "UTF-8 encoded byte limits and rejection of unpaired surrogates.",
    "Duplicate keys and frame byte/encoding/LF/depth/acquisition rules before parsing.",
    "Request/result action, digest, identity, generation and returned state correlation.",
    "Snapshot required/complete for null request cursor and bounded by requested limit.",
    "Pagination result count bounded by the correlated request limit.",
    "Peer/grant visibility, deadline/idempotency custody, profile and effect truth.",
    "Event cursor/epoch/sequence equality and retained-interval continuity.",
)


def read_json(path):
    return json.loads(path.read_bytes())


def at(value, path):
    for component in path:
        value = value[component]
    return value


def changed(value, path, replacement):
    result = copy.deepcopy(value)
    at(result, path[:-1])[path[-1]] = copy.deepcopy(replacement)
    return result


def object_paths(value, path=()):
    if isinstance(value, dict):
        yield path, value
        for key, item in value.items():
            yield from object_paths(item, path + (key,))
    elif isinstance(value, list):
        for index, item in enumerate(value):
            yield from object_paths(item, path + (index,))


class RC03SchemaTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.schema = read_json(SCHEMA_PATH)
        cls.root_validator = Draft202012Validator(cls.schema)
        cls.rows = read_json(FIXTURES / "actions.json")["cases"]
        cls.pairs = {row["action"]: row for row in cls.rows}
        cls.observations = read_json(FIXTURES / "observations.json")
        cls.validators = {}

    @classmethod
    def validator(cls, name):
        if name not in cls.validators:
            # References select the subject, never generate expected fixtures.
            cls.validators[name] = Draft202012Validator({
                "$schema": "https://json-schema.org/draft/2020-12/schema",
                "$defs": cls.schema["$defs"], "$ref": "#/$defs/" + name,
            })
        return cls.validators[name]

    def valid(self, name, instance):
        error = next(self.validator(name).iter_errors(instance), None)
        self.assertIsNone(error, str(error))

    def invalid(self, name, instance):
        self.assertIsNotNone(next(self.validator(name).iter_errors(instance), None))

    def envelope(self, action, kind="request"):
        return copy.deepcopy(self.pairs[action][kind])

    @staticmethod
    def definition(action, kind="request"):
        return kind.capitalize() + "_" + action.replace(".", "_")

    def test_01_schema_and_hand_authored_catalogue_coverage(self):
        Draft202012Validator.check_schema(self.schema)
        self.assertEqual(tuple(row["action"] for row in self.rows), ACTIONS)
        self.assertEqual(len(self.pairs), 21)
        for action in ACTIONS:
            for kind in ("request", "result"):
                with self.subTest(action=action, kind=kind):
                    value = self.envelope(action, kind)
                    self.valid(self.definition(action, kind), value)
                    self.assertTrue(self.root_validator.is_valid(value))
            raw_request = json.dumps(self.envelope(action), separators=(",", ":"), ensure_ascii=False).encode()
            self.assertEqual(self.envelope(action, "result")["request_sha256"],
                             "sha256:" + hashlib.sha256(raw_request).hexdigest())
        for action in ("tools.list", "task.list", "thread.list", "roster.list"):
            self.assertGreater(len(self.envelope(action, "result")["body"]["page"]["items"]), 0)

    def test_02_exact_c01_bytes_digest_and_schema(self):
        raw = (FIXTURES / "C01-control-valid-cancel.jsonl").read_bytes()
        metadata = read_json(FIXTURES / "C01-metadata.json")
        self.assertEqual(len(raw), 541)
        self.assertEqual(raw[-1:], b"\n")
        self.assertNotIn(b"\n", raw[:-1])
        self.assertEqual(len(raw[:-1]), 540)
        expected = "b762237226571d87fa1b71a871f055923110aa531efe00a670535450dc589384"
        self.assertEqual(hashlib.sha256(raw[:-1]).hexdigest(), expected)
        self.assertEqual(metadata["payload_sha256"], expected)
        self.assertEqual(metadata["payload_bytes_excluding_lf"], 540)
        self.assertEqual(metadata["fixture_receive_unix_ms"], "1769999995000")
        self.valid("Request_task_cancel", json.loads(raw))
        self.assertNotEqual(hashlib.sha256(raw).hexdigest(), expected)

    def test_03_missing_and_unknown_fields_at_all_exercised_object_depths(self):
        # Parameter permutations are one coverage family, not individual credits.
        for action in ACTIONS:
            for kind in ("request", "result"):
                value = self.envelope(action, kind)
                name = self.definition(action, kind)
                self.valid(name, value)
                for path, node in object_paths(value):
                    with self.subTest(action=action, kind=kind, path=path, fault="unknown"):
                        altered = copy.deepcopy(value)
                        at(altered, path)["unlisted_field"] = True
                        self.invalid(name, altered)
                    for key in node:
                        # RC03 says this field depends on the correlated request.
                        if action == "events.subscribe" and kind == "result" and path == ("body",) and key == "snapshot":
                            continue
                        with self.subTest(action=action, kind=kind, path=path, fault="missing", key=key):
                            altered = copy.deepcopy(value)
                            del at(altered, path)[key]
                            self.invalid(name, altered)

    def test_04_nested_object_and_body_types_refuse(self):
        for action in ACTIONS:
            for kind in ("request", "result"):
                value = self.envelope(action, kind)
                for path, _ in object_paths(value):
                    if not path:
                        continue
                    with self.subTest(action=action, kind=kind, path=path):
                        self.invalid(self.definition(action, kind), changed(value, path, []))

    def test_05_versions_protocol_and_action_are_closed(self):
        base = self.envelope("task.cancel")
        self.valid("ControlRequestV1", base)
        for key, values in {
            "version": [0, 2, 65536, True, False, "1", None, 1.5],
            "action_version": [0, 2, True, "1"],
            "protocol": ["hee3.analysis", "HEE3.control", "hee3.control\n"],
            "kind": ["response", "REQUEST"],
            "action": ["task.delete", "Task.cancel", "task.cancel\n"],
        }.items():
            for value in values:
                with self.subTest(key=key, value=value):
                    self.invalid("ControlRequestV1", changed(base, (key,), value))

    def test_06_uuid_digest_and_numeric_string_boundaries(self):
        for name, maximum in (("U64Decimal", "18446744073709551615"), ("U32Decimal", "4294967295")):
            for value in ("0", "1", str(int(maximum) - 1), maximum):
                self.valid(name, value)
            for value in ("", "00", "01", "+1", "-1", " 1", "1 ", "1\n", "1\r\n", "1.0", "1e2", "١", "１", "1\x00", "1_0", str(int(maximum) + 1), "9" * 21, 1, True, None):
                with self.subTest(name=name, value=value):
                    self.invalid(name, value)
        self.valid("Generation", "1")
        self.valid("Generation", "18446744073709551615")
        self.invalid("Generation", "0")
        self.valid("UuidV4", UUID)
        for value in (UUID.upper(), "00000000-0000-0000-0000-000000000000", UUID.replace("42d3", "12d3"), UUID.replace("a456", "7456"), UUID + "\n", UUID.replace("-", ""), None):
            self.invalid("UuidV4", value)
        digest = "sha256:" + "a" * 64
        self.valid("Sha256", digest)
        for value in (digest.upper(), "a" * 64, "sha256:" + "a" * 63, "sha256:" + "g" * 64, digest + "\n", None):
            self.invalid("Sha256", value)

    def test_07_explicit_enums_refuse_unknown_variants(self):
        vectors = (
            ("task.submit", "request", ("body", "spec", "privacy"), "public"),
            ("task.submit", "request", ("body", "spec", "budget", "mode"), "unlimited"),
            ("task.cancel", "request", ("body", "reason"), "kill"),
            ("task.resolve", "request", ("body", "disposition"), "accept"),
            ("task.get", "request", ("body", "evidence"), "all"),
            ("task.get", "result", ("body", "task", "state"), "complete"),
            ("task.get", "result", ("body", "attempts", 0, "effect"), "rolled_back"),
            ("task.get", "result", ("body", "cleanup"), "successful"),
            ("task.get", "result", ("body", "delivery"), "settled"),
            ("task.preview", "result", ("body", "cost_mode"), "free"),
            ("task.preview", "result", ("body", "exclusions", 0, "code"), "error"),
            ("thread.get", "result", ("body", "state"), "done"),
            ("roster.update", "request", ("body", "definition", "kind"), "provider"),
            ("roster.update", "request", ("body", "definition", "locality"), "internet"),
            ("roster.disable", "request", ("body", "active_attempt_policy"), "kill"),
            ("service.probe", "request", ("body", "network_scope"), "unrestricted"),
            ("service.probe", "result", ("body", "external_effect"), "unknown"),
            ("service.action", "request", ("body", "action"), "enable"),
            ("service.action", "result", ("body", "observed_state"), "ready"),
            ("analysis.request", "request", ("body", "recipe_id"), "eval"),
            ("analysis.get", "result", ("body", "state"), "accepted"),
            ("health", "result", ("body", "recovery"), "ready"),
            ("health", "result", ("body", "database"), "healthy"),
            ("health", "result", ("body", "socket"), "public"),
        )
        for action, kind, path, value in vectors:
            with self.subTest(action=action, path=path):
                base = self.envelope(action, kind)
                self.valid(self.definition(action, kind), base)
                self.invalid(self.definition(action, kind), changed(base, path, value))

    def test_08_integer_bounds_boolean_types_and_text_bounds(self):
        bounds = (
            ("tools.list", "request", ("body", "page", "limit"), 1, 100),
            ("analysis.request", "request", ("body", "limits", "output_bytes"), 1, 65536),
            ("analysis.request", "request", ("body", "dataset", "byte_length"), 0, 4294967295),
            ("task.preview", "result", ("body", "eligible", 0, "recipe_version"), 1, 65535),
            ("events.subscribe", "request", ("body", "bootstrap_limit"), 1, 256),
        )
        for action, kind, path, low, high in bounds:
            base = self.envelope(action, kind)
            name = self.definition(action, kind)
            for value in (low, high):
                self.valid(name, changed(base, path, value))
            for value in (low - 1, high + 1, True, False, "1", 1.5, None):
                with self.subTest(action=action, path=path, value=value):
                    self.invalid(name, changed(base, path, value))
        for value in (0, 1, "true", None):
            self.invalid("Result_health", changed(self.envelope("health", "result"), ("body", "ready"), value))
        for path, maximum in ((("body", "spec", "intent"), 8192), (("body", "spec", "criteria", 0), 1024)):
            base = self.envelope("task.submit")
            self.valid("Request_task_submit", changed(base, path, "a" * maximum))
            self.invalid("Request_task_submit", changed(base, path, ""))
            self.invalid("Request_task_submit", changed(base, path, "a" * (maximum + 1)))
        for value in ("", "a" * 65, "café", "公開"):
            self.invalid("Request_task_submit", changed(self.envelope("task.submit"), ("body", "spec", "task_class"), value))

    def test_09_array_caps_and_duplicate_free_filters(self):
        caps = (
            ("task.submit", "request", ("body", "spec", "criteria"), 64, 1),
            ("task.preview", "result", ("body", "eligible"), 128, 0),
            ("task.preview", "result", ("body", "exclusions"), 128, 0),
            ("task.get", "result", ("body", "attempts"), 100, 0),
            ("task.get", "result", ("body", "evidence"), 64, 0),
            ("thread.get", "result", ("body", "obligations"), 128, 0),
            ("thread.get", "result", ("body", "children"), 100, 0),
            ("thread.get", "result", ("body", "artifacts"), 64, 0),
            ("roster.update", "request", ("body", "definition", "capabilities"), 128, 0),
            ("roster.disable", "result", ("body", "active_attempts"), 100, 0),
            ("roster.disable", "result", ("body", "cancellation_obligations"), 100, 0),
            ("service.probe", "result", ("body", "observation", "evidence"), 64, 0),
            ("events.subscribe", "result", ("body", "snapshot"), 256, 0),
            ("task.list", "result", ("body", "page", "items"), 100, 0),
        )
        for action, kind, path, maximum, minimum in caps:
            base = self.envelope(action, kind)
            item = at(base, path)[0]
            name = self.definition(action, kind)
            with self.subTest(action=action, path=path):
                self.valid(name, changed(base, path, [item] * maximum))
                self.invalid(name, changed(base, path, [item] * (maximum + 1)))
                if minimum:
                    self.invalid(name, changed(base, path, []))
                else:
                    self.valid(name, changed(base, path, []))
        for action, field, minimum in (("task.list", "states", 0), ("thread.list", "states", 0), ("roster.list", "kinds", 1), ("events.subscribe", "topics", 1), ("events.subscribe", "resource_ids", 0)):
            base = self.envelope(action)
            item = base["body"][field][0]
            name = self.definition(action)
            self.valid(name, base)
            self.invalid(name, changed(base, ("body", field), [item, item]))
            if minimum:
                self.invalid(name, changed(base, ("body", field), []))
            else:
                self.valid(name, changed(base, ("body", field), []))
        resource_ids = [f"123e4567-e89b-42d3-a456-{index:012x}" for index in range(101)]
        subscription = self.envelope("events.subscribe")
        self.valid("Request_events_subscribe", changed(subscription, ("body", "resource_ids"), resource_ids[:100]))
        self.invalid("Request_events_subscribe", changed(subscription, ("body", "resource_ids"), resource_ids))

    def test_10_effectful_idempotency_and_mutation_preconditions(self):
        for action in EFFECTFUL:
            base = self.envelope(action)
            self.valid(self.definition(action), base)
            self.invalid(self.definition(action), changed(base, ("idempotency_key",), None))
        for action in READ_ACTIONS:
            self.valid(self.definition(action), changed(self.envelope(action), ("idempotency_key",), None))
        for action, resource in (("task.cancel", "task"), ("task.resolve", "task"), ("roster.disable", "roster"), ("service.action", "service")):
            base = self.envelope(action)
            name = self.definition(action)
            self.assertEqual(base["precondition"]["resource"], resource)
            self.invalid(name, changed(base, ("precondition",), None))
            self.invalid(name, changed(base, ("precondition", "resource"), "analysis"))
            self.invalid(name, changed(base, ("precondition", "generation"), "0"))
        create = self.envelope("roster.update")
        existing = changed(create, ("body", "record_id"), UUID)
        generation = {"resource": "roster", "id": UUID, "generation": "7"}
        self.valid("Request_roster_update", create)
        self.invalid("Request_roster_update", changed(create, ("precondition",), generation))
        self.invalid("Request_roster_update", existing)
        self.valid("Request_roster_update", changed(existing, ("precondition",), generation))
        self.invalid("Request_roster_update", changed(existing, ("precondition",), {**generation, "resource": "task"}))
        self.invalid("Request_task_submit", changed(self.envelope("task.submit"), ("precondition",), {"resource": "task", "id": UUID, "generation": "1"}))

    def test_11_readback_complete_read_actions_and_selector_alternatives(self):
        for action in READ_ACTIONS:
            selector = {"action": action, "action_version": 1, "body": self.envelope(action)["body"]}
            self.valid("ReadbackSelectorV1", selector)
        for action in set(ACTIONS) - set(READ_ACTIONS):
            self.invalid("ReadbackSelectorV1", {"action": action, "action_version": 1, "body": self.envelope(action)["body"]})
        alternatives = (
            ("task.get", {"selector": {"source_action": "task.submit", "idempotency_key": UUID}, "evidence": "none"}),
            ("roster.inspect", {"selector": {"source_action": "roster.update", "idempotency_key": UUID}}),
            ("roster.inspect", {"selector": {"source_action": "roster.disable", "idempotency_key": UUID}}),
            ("service.inspect", {"service_id": UUID, "operation": {"source_action": "service.probe", "idempotency_key": UUID}}),
            ("service.inspect", {"service_id": UUID, "operation": {"source_action": "service.action", "idempotency_key": UUID}}),
            ("analysis.get", {"selector": {"source_action": "analysis.request", "idempotency_key": UUID}}),
        )
        for action, body in alternatives:
            self.valid("ReadbackSelectorV1", {"action": action, "action_version": 1, "body": body})
        for invalid in (
            {"action": "task.get", "action_version": 1, "selector": {"task_id": UUID}},
            {"action": "task.get", "action_version": 1, "body": {"selector": {"task_id": UUID}}},
            {"action": "task.get", "action_version": 1, "body": {"selector": {"task_id": UUID, "source_action": "task.submit", "idempotency_key": UUID}, "evidence": "none"}},
            {"action": "health", "action_version": 1, "body": {}, "endpoint": "http://example.invalid"},
        ):
            self.invalid("ReadbackSelectorV1", invalid)

    def test_12_pending_results_and_nonempty_cross_action_results(self):
        pending = changed(self.envelope("service.action", "result"), ("effect",), "pending")
        self.valid("Result_service_action", pending)
        self.invalid("Result_service_action", changed(pending, ("operation_id",), None))
        self.invalid("Result_service_action", changed(pending, ("readback",), None))
        self.invalid("Result_service_action", changed(pending, ("readback",), {"action": "health", "action_version": 1, "body": {}}))
        for destination, source in (("task.list", "roster.list"), ("thread.list", "task.list"), ("roster.list", "thread.list"), ("task.get", "analysis.get")):
            foreign = changed(self.envelope(destination, "result"), ("body",), self.envelope(source, "result")["body"])
            self.invalid(self.definition(destination, "result"), foreign)

    def test_13_all_error_codes_closed_details_and_unknown_effect_rules(self):
        errors = self.observations["errors"]
        self.assertEqual(tuple(error["code"] for error in errors), ERROR_CODES)
        for error in errors:
            self.valid("ControlErrorV1", error)
            self.assertTrue(self.root_validator.is_valid(error))
            for path, node in object_paths(error):
                bad = copy.deepcopy(error)
                at(bad, path)["unlisted_field"] = 1
                self.invalid("ControlErrorV1", bad)
                for key in node:
                    bad = copy.deepcopy(error)
                    del at(bad, path)[key]
                    self.invalid("ControlErrorV1", bad)
        unknown = next(error for error in errors if error["code"] == "effect_unknown")
        for path, value in ((("effect",), "none"), (("retry",), "never"), (("readback",), None)):
            self.invalid("ControlErrorV1", changed(unknown, path, value))
        self.invalid("ControlErrorV1", changed(errors[0], ("code",), "not_authorized"))
        self.invalid("ControlErrorV1", changed(errors[0], ("message",), "a" * 4097))
        self.valid("ControlErrorV1", changed(errors[0], ("message",), "a" * 4096))

    def test_14_all_event_topics_readbacks_and_stream_statuses(self):
        self.assertEqual(tuple(event["topic"] for event in self.observations["events"]),
                         ("task", "thread", "roster", "service", "analysis", "delivery", "recovery"))
        for event in self.observations["events"]:
            self.valid("ControlEventV1", event)
            self.assertTrue(self.root_validator.is_valid(event))
            self.invalid("ControlEventV1", changed(event, ("topic",), "unknown"))
            other_topic = "service" if event["topic"] != "service" else "task"
            self.invalid("ControlEventV1", changed(event, ("topic",), other_topic))
            self.invalid("ControlEventV1", changed(event, ("subject", "readback"), {"action": "tools.list", "action_version": 1, "body": self.envelope("tools.list")["body"]}))
            self.invalid("ControlEventV1", changed(event, ("subject", "generation"), "0"))
        self.assertEqual(tuple(status["status"] for status in self.observations["statuses"]),
                         ("resync_required", "queue_limit", "server_draining"))
        for status in self.observations["statuses"]:
            self.valid("StreamStatusV1", status)
            self.valid("StreamStatusV1", changed(status, ("last_delivered_cursor",), None))
            self.invalid("StreamStatusV1", changed(status, ("status",), "complete"))
        for name, records in (("ControlEventV1", self.observations["events"]), ("StreamStatusV1", self.observations["statuses"])):
            for value in records:
                for path, node in object_paths(value):
                    bad = copy.deepcopy(value)
                    at(bad, path)["unlisted_field"] = True
                    self.invalid(name, bad)
                    for key in node:
                        bad = copy.deepcopy(value)
                        del at(bad, path)[key]
                        self.invalid(name, bad)

    def test_15_runtime_gap_numeric_tokens_utf8_bytes_and_duplicate_keys(self):
        # Passing these structural checks documents gaps; it is NOT safe admission.
        health = self.envelope("health")
        raw = json.dumps(health, separators=(",", ":")).encode()
        decimal_token = raw.replace(b'"version":1,', b'"version":1.0,', 1)
        exponent_token = raw.replace(b'"version":1,', b'"version":1e0,', 1)
        for value in (decimal_token, exponent_token):
            self.assertNotEqual(value, raw)
            self.valid("Request_health", json.loads(value))
        duplicate = raw.replace(b'"action":"health"', b'"action":"task.submit","action":"health"', 1)
        self.assertNotEqual(duplicate, raw)
        self.valid("Request_health", json.loads(duplicate))
        huge_utf8 = "é" * 4096
        self.assertGreater(len(huge_utf8.encode("utf-8")), 4096)
        self.valid("ControlErrorV1", changed(self.observations["errors"][0], ("message",), huge_utf8))

    def test_16_runtime_gap_request_result_snapshot_and_cursor_correlation(self):
        # These intentionally remain valid structurally and need typed validators.
        created = self.envelope("roster.update", "result")
        self.valid("Result_roster_update", changed(created, ("operation_id",), OTHER_UUID))
        self.valid("Result_roster_update", changed(created, ("request_id",), OTHER_UUID))
        self.valid("Result_roster_update", changed(created, ("request_sha256",), "sha256:" + "f" * 64))
        snapshot = self.envelope("events.subscribe", "result")
        self.assertIsNone(self.envelope("events.subscribe")["body"]["cursor"])
        del snapshot["body"]["snapshot"]
        self.valid("Result_events_subscribe", snapshot)
        snapshot = self.envelope("events.subscribe", "result")
        snapshot["body"]["snapshot"] *= 3
        self.assertGreater(len(snapshot["body"]["snapshot"]), self.envelope("events.subscribe")["body"]["bootstrap_limit"])
        self.valid("Result_events_subscribe", snapshot)
        page = self.envelope("task.list", "result")
        page["body"]["page"]["items"] *= 3
        self.assertGreater(len(page["body"]["page"]["items"]), self.envelope("task.list")["body"]["page"]["limit"])
        self.valid("Result_task_list", page)
        event = changed(self.observations["events"][0], ("previous_sequence",), "999")
        self.valid("ControlEventV1", event)
        empty_page = changed(self.envelope("task.list", "result"), ("body", "page", "items"), [])
        for action in ("task.list", "thread.list", "roster.list"):
            self.valid(self.definition(action, "result"), empty_page)
        self.valid("ControlResultV1", empty_page)


def main():
    suite = unittest.defaultTestLoader.loadTestsFromTestCase(RC03SchemaTests)
    result = unittest.TextTestRunner(verbosity=2).run(suite)
    paths = [Path(__file__).resolve(), SCHEMA_PATH, *sorted(FIXTURES.glob("*.json")), *sorted(FIXTURES.glob("*.jsonl"))]
    print(json.dumps({
        "scope": "RC03 structural schema test implementation; no module acceptance or independent oracle authorship",
        "success": result.wasSuccessful(), "test_methods_run": result.testsRun,
        "failures": len(result.failures), "errors": len(result.errors), "skipped": len(result.skipped),
        "module_case_credits": 0, "action_fixture_pairs": 21, "error_code_fixtures": 18,
        "python_version": sys.version, "python_executable": sys.executable,
        "jsonschema_version": importlib.metadata.version("jsonschema"),
        "subjects_sha256": {str(path.relative_to(ROOT)): hashlib.sha256(path.read_bytes()).hexdigest() for path in paths},
        "runtime_gaps": RUNTIME_GAPS,
    }, sort_keys=True))
    return 0 if result.wasSuccessful() and not result.skipped else 1


if __name__ == "__main__":
    raise SystemExit(main())
