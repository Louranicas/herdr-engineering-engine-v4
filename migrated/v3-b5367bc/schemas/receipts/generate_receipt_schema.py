#!/usr/bin/env python3
"""Materialize RC04's closed receipt records and typed inventory schema bundle.

Authoring tooling only; Python standard library, no collection or admission.
Authority: READINESS_CONVENTION_habitat_engine.json revision 4, RC04, projected
in docs/contract-decisions.md. Run --check for an exact artifact-byte comparison.
This structural artifact does not implement protected collection or admission.
"""

import argparse
import json
from pathlib import Path


RUNTIME_OBLIGATIONS = (
    "Validate compact exact UTF-8 JSON bytes: no BOM/trailing LF/duplicate keys/"
    "nonfinite numbers; cap manifest at 1048576 bytes before parsing.",
    "Enforce UTF-8 byte lengths and reject unpaired surrogates; JSON Schema "
    "string lengths count Unicode code points, not encoded bytes.",
    "Enforce integer token spelling if required by RC03; JSON Schema integer "
    "values cannot distinguish parsed 1 from 1.0 or 1e0.",
    "Resolve every Ref using its schema_id, exact digest and byte count from "
    "protected custody, without following candidate-controlled paths.",
    "Recompute all counts from complete typed inventories and pages, detect "
    "omissions/duplicates, and enforce one primary owner per credited case.",
    "Correlate exact seed/result/fixture/oracle/profile identities before and "
    "after collection, including patch scope, immutable source and raw logs.",
    "Check selected mandatory cases, expected producer outcome, independent "
    "oracle truth, diagnostic explanations, and meaningful nonempty credit.",
    "Check deadlines, clock ordering, resources, cancellation, settled cleanup, "
    "empty descendants and absence of unresolved material obligations. Wall/cleanup "
    "cutoffs share invocation start_monotonic_ns; cleanup cutoffs agree and are "
    "<=wall_ms, never reset on retry, and are bounded by the minimum of the "
    "predeclared cutoff, wall_ms and intent_elapsed_ms+10000.",
    "Retain finalized evidence and append separately finalized review and "
    "availability observations; never mutate historical receipt bytes. A missing "
    "availability observation resolves prior manifest bytes/shape without requiring "
    "its missing historical transitive payloads to exist.",
    "PASS_CANDIDATE is structurally eligible for separate review only; schema "
    "validation neither implements a protected collector nor admits a module.",
)


def ref(name):
    return {"$ref": "#/$defs/" + name}


def record(properties, comment=None):
    result = {
        "type": "object", "properties": properties,
        "required": list(properties), "additionalProperties": False,
    }
    if comment:
        result["$comment"] = comment
    return result


def array(items):
    return {"type": "array", "items": items, "maxItems": 256}


def scalar_enum(*values):
    return {"type": "string", "enum": list(values)}


def properties(**values):
    """A partial constraint, always composed with the closed full record."""
    return {"properties": values}


def present():
    return properties(value={"not": {"type": "null"}})


def absent():
    return properties(value={"type": "null"})


def maybe(inner):
    result = record({
        "value": {"oneOf": [inner, {"type": "null"}]},
        "unavailable_reason": {"oneOf": [ref("Text"), {"type": "null"}]},
    })
    result["oneOf"] = [
        properties(value={"not": {"type": "null"}},
                   unavailable_reason={"type": "null"}),
        properties(value={"type": "null"},
                   unavailable_reason=ref("Text")),
    ]
    return result


def decimal_u64():
    ceiling = "18446744073709551615"
    alternatives = ["0", "[1-9][0-9]{0,18}"]
    for index, character in enumerate(ceiling):
        low = 1 if index == 0 else 0
        high = int(character) - 1
        if high >= low:
            digit = str(low) if low == high else f"[{low}-{high}]"
            remaining = len(ceiling) - index - 1
            suffix = f"[0-9]{{{remaining}}}" if remaining else ""
            alternatives.append(ceiling[:index] + digit + suffix)
    alternatives.append(ceiling)
    return {
        "type": "string", "minLength": 1, "maxLength": 20,
        "pattern": "^(?:" + "|".join(alternatives) + ")$",
        # Remove the trailing-newline exception in regular-expression '$'.
        "not": {"pattern": "[^0-9]"},
    }


def typed_ref(name):
    return {"allOf": [ref("Ref"), properties(
        schema_id={"const": "hee3.receipt/1:" + name},
    )]}


def add_inventory_definitions(defs):
    """RC04 revision-4 inventory refinement; every record is explicit and closed."""
    defs["Payload"] = {"allOf": [ref("Ref"), properties(
        schema_id={"const": "hee3.raw/1"},
    )]}
    defs["Reason"] = {"allOf": [ref("Text"), {"minLength": 1}]}
    defs["RelPath"] = {"allOf": [ref("Reason"), {"not": {
        "pattern": r"(^/|//|\u0000|/$(?![\s\S])|(^|/)\.{1,2}(/|$(?![\s\S])))",
    }}], "$comment": "Descriptive UTF-8 inventory path only; never candidate import authority. "
                      "UTF-8 byte cap, sorted order, aliases and file custody require runtime checks."}
    for name in ("Sha", "U64", "Name"):
        defs["Maybe" + name] = maybe(ref(name))
    defs["ExpectedProducerV1"] = record({
        "status": scalar_enum("exited", "signalled"),
        "exit_code": ref("MaybeCount"), "signal": ref("MaybeCount"),
    })
    defs["ExpectedProducerV1"]["oneOf"] = [
        properties(status={"const": "exited"}, exit_code=present(), signal=absent()),
        properties(status={"const": "signalled"}, exit_code=absent(), signal=present()),
    ]
    defs["CaseV1"] = record({
        "case_id": ref("Name"), "primary_module_id": ref("Name"),
        "criterion_ids": array(ref("Name")), "fixture_sha256": ref("Sha"),
        "oracle_id": ref("Name"), "expected": typed_ref("ExpectationV1"),
        "mandatory": ref("Bool"), "excluded": ref("Bool"),
        "selected": ref("Bool"), "executed": ref("Bool"),
        "outcome": scalar_enum("passed", "failed", "skipped", "ignored", "broken",
                               "timeout", "invalid", "unmeasured"),
        "producer_exit_or_signal": ref("ProducerV1"), "detector_id": ref("Name"),
        "benign_pair_id": ref("MaybeId"), "raw_evidence_refs": array(ref("Ref")),
        "reason": ref("Text"),
    })
    defs["CaseV1"]["allOf"] = [
        {"if": properties(mandatory={"const": True}),
         "then": properties(selected={"const": True}, excluded={"const": False})},
        {"if": properties(executed={"const": True}),
         "then": properties(selected={"const": True})},
        {"if": properties(excluded={"const": True}),
         "then": properties(selected={"const": False}, executed={"const": False},
                            outcome={"const": "unmeasured"}, reason=ref("Reason"))},
        {"if": properties(selected={"const": False}),
         "then": properties(outcome={"const": "unmeasured"}, reason=ref("Reason"))},
        {"if": properties(outcome={"not": {"const": "passed"}}),
         "then": properties(reason=ref("Reason"))},
    ]
    defs["CaseV1"]["$comment"] = "Mandatory selection is frozen before observation. "
    defs["CaseV1"]["$comment"] += (
        "A resolved PASS inventory needs meaningful mandatory rows, each executed/passed; "
        "case shape alone cannot establish oracle truth or distinct primary case credit."
    )
    defs["SubjectV1"] = record({
        "subject_id": ref("Id"), "files": typed_ref("SubjectFilePageV1"),
        "tree_sha256": ref("Sha"), "dirty_patch": maybe(ref("Payload")),
    })
    defs["SubjectFileV1"] = record({
        "path": ref("RelPath"),
        "kind": scalar_enum("file", "directory", "symlink", "submodule", "other"),
        "content": maybe(ref("Payload")), "executable": ref("Bool"),
        "link_target": ref("MaybeText"),
        "origin": scalar_enum("authored", "generated", "excluded"),
        "exclusion_reason": ref("MaybeText"),
    })
    defs["SubjectFileV1"]["allOf"] = [
        {"if": properties(executable={"const": True}),
         "then": properties(kind={"const": "file"})},
        {"if": properties(kind={"const": "file"}),
         "then": properties(content=present(), link_target=absent())},
        {"if": properties(kind={"const": "directory"}),
         "then": properties(content=absent(), link_target=absent())},
        {"if": properties(kind={"const": "symlink"}),
         "then": properties(link_target=present())},
        {"if": properties(origin={"const": "excluded"}),
         "then": properties(exclusion_reason=properties(value=ref("Reason")))},
    ]
    defs["LockV1"] = record({
        "lock_id": ref("Name"), "ecosystem": ref("Name"), "path": ref("RelPath"),
        "content": ref("Payload"), "dependencies": typed_ref("DependencyPageV1"),
    })
    defs["DependencyV1"] = record({
        "dependency_id": ref("Name"), "name": ref("Name"), "version": ref("Name"),
        "source": ref("Text"), "checksum": ref("MaybeSha"), "locked_by": ref("Payload"),
    })
    defs["ToolV1"] = record({
        "tool_id": ref("Name"), "executable": ref("Payload"),
        "executable_path": ref("Text"), "version": ref("Text"),
        "version_output": ref("Payload"), "target": ref("Name"),
    })
    defs["BuildProfileV1"] = record({
        "target": ref("Name"), "features": array(ref("Name")),
        "default_features": ref("Bool"), "build_profile": ref("Name"),
        "language_flags": typed_ref("LanguageFlagsPageV1"),
    })
    defs["LanguageFlagsV1"] = record({"language": ref("Name"), "argv": array(ref("Text"))})
    defs["StandardV1"] = record({
        "standard_id": ref("Name"), "revision": ref("Name"), "document": ref("Payload"),
    })
    defs["EnvironmentV1"] = record({
        "name": {"allOf": [ref("Name"), {"not": {"pattern": r"[\u0000=]"}}]},
        "value": ref("MaybeText"), "secret_handle": ref("MaybeId"),
    })
    defs["EnvironmentV1"]["oneOf"] = [
        properties(value=present(), secret_handle=absent()),
        properties(value=absent(), secret_handle=present()),
    ]
    defs["GrantV1"] = record({
        "grant_id": ref("Id"), "scope_sha256": ref("Sha"), "issuer_id": ref("Name"),
        "grant": ref("Payload"),
    })
    defs["ExpectationV1"] = record({
        "oracle_id": ref("Name"),
        "oracle_class": scalar_enum("contract", "reference", "round_trip_invariant",
                                    "metamorphic", "differential"),
        "specification": ref("Payload"), "expected_producer": ref("ExpectedProducerV1"),
        "expected_oracle": {"const": "satisfied"}, "intended_detector": ref("Name"),
        "shared_assumptions": typed_ref("AssumptionPageV1"),
    })
    defs["LimitsV1"] = record({
        **{name: ref("U64") for name in (
            "wall_ms", "memory_bytes", "memory_swap_bytes", "scratch_bytes", "stdout_bytes",
            "stderr_bytes", "artifact_bytes", "external_requests", "external_cost_microunits",
            "term_grace_ms", "cleanup_deadline_ms",
        )},
        **{name: ref("Count") for name in (
            "cpu_quota_percent", "tasks_max", "compiler_jobs", "julia_threads", "blas_threads",
        )},
        "currency": ref("MaybeName"),
    }, "RC01/RC02/RC05 profile-specific limits, positivity and currency correlation are runtime.")
    defs["EffectV1"] = record({
        "effect_id": ref("Name"), "grant_id": ref("Id"), "owner_id": ref("Name"),
        "scope": ref("Text"), "specification": ref("Payload"),
    })
    defs["CleanupContractV1"] = record({
        "owner_id": ref("Name"), "term_grace_ms": ref("U64"), "deadline_ms": ref("U64"),
        "require_empty_descendants": ref("Bool"),
        "obligations": typed_ref("ObligationPageV1"), "readback_specification": ref("Payload"),
    })
    defs["HostV1"] = record({
        "os": ref("Name"), "release": ref("Name"), "architecture": ref("Name"),
        "kernel": ref("Text"), "boot_id": ref("Name"), "logical_cpus": ref("Count"),
        "memory_bytes": ref("U64"), "facts": ref("Payload"),
    })
    defs["ResourceV1"] = record({
        "metric": ref("Name"), "unit": ref("Name"), "value": ref("MaybeU64"),
        "limit": ref("MaybeU64"), "limit_event": ref("Bool"), "evidence": ref("Ref"),
    })
    defs["ObligationV1"] = record({
        "obligation_id": ref("Id"), "owner_id": ref("Name"), "scope": ref("Text"),
        "material": ref("Bool"), "state": scalar_enum("open", "settled", "unknown"),
        "evidence": array(ref("Ref")), "reason": ref("Text"),
    })
    defs["DiagnosticV1"] = record({
        "tool_id": ref("Name"), "baseline": ref("Bool"), "warning_count": ref("Count"),
        "error_count": ref("Count"), "stdout": ref("Payload"), "stderr": ref("Payload"),
        "stdout_truncated": ref("Bool"), "stderr_truncated": ref("Bool"),
        "producer": ref("ProducerV1"),
    })
    defs["ArtifactV1"] = record({
        "object": ref("Ref"), "role": ref("Name"), "required": ref("Bool"),
        "truncated": ref("Bool"), "availability": scalar_enum("available", "missing"),
        "reason": ref("Text"),
    })
    defs["CampaignV1"] = record({
        "campaign_id": ref("Name"), "language": scalar_enum("rust", "julia"),
        "family_id": ref("Name"), "tool": ref("Payload"), "config": ref("Payload"),
        "baseline_receipt": typed_ref("ReceiptV1"), "mutants": typed_ref("MutantPageV1"),
        "planned_mutants": ref("Count"),
    })
    defs["MutantV1"] = record({
        "mutant_id": ref("Name"), "campaign_id": ref("Name"),
        "baseline_subject_sha256": ref("Sha"), "diff": ref("Payload"),
        "expected_detector": ref("Name"), "observed_detector": ref("MaybeName"),
        "outcome": scalar_enum("caught", "survived", "timeout", "unviable",
                               "reviewed-equivalent", "excluded", "unmeasured"),
        "executed": ref("Bool"), "reason": ref("Text"),
        "raw_evidence_refs": array(ref("Ref")), "review_ref": maybe(typed_ref("ReviewV1")),
    })
    # RC04 rev4 Record invariants: consequential mutations bind an exact nonempty
    # diff, while unviable includes invalid diffs and unexecuted/excluded rows
    # retain their actual disposition. The reviewed outcome-specific guard keeps
    # malformed empty diffs representable without treating them as valid mutants.
    defs["MutantV1"]["allOf"] = [
        {"if": properties(outcome=scalar_enum("caught", "survived", "timeout", "reviewed-equivalent")),
         "then": properties(diff=properties(byte_length={"minimum": 1}))},
        {"if": properties(outcome={"const": "caught"}),
         "then": properties(executed={"const": True}, observed_detector=present())},
        {"if": properties(outcome={"const": "reviewed-equivalent"}),
         "then": properties(review_ref=present())},
        {"if": {"anyOf": [properties(executed={"const": False}), properties(
            outcome=scalar_enum("survived", "reviewed-equivalent", "excluded", "unviable", "unmeasured"),
        )]}, "then": properties(reason=ref("Reason"))},
    ]
    defs["MutantV1"]["$comment"] = (
        "Runtime binds caught detector equality and exact nonempty diff. A review_ref "
        "resolves ReviewV1 with action=mutation_disposition and subject_sha256=diff.sha256."
    )
    defs["AssumptionV1"] = record({
        "assumption_id": ref("Name"), "shared_with": array(ref("Name")),
        "statement": ref("Reason"), "evidence": array(ref("Ref")),
    })
    defs["FindingV1"] = record({
        "finding_id": ref("Name"), "owner_id": ref("Name"), "scope": ref("Text"),
        "material": ref("Bool"),
        "disposition": scalar_enum("open", "fixed", "accepted_residual", "not_reproduced", "out_of_scope"),
        "rationale": ref("Reason"), "evidence": array(ref("Ref")),
        "pre_fix_evidence": array(ref("Ref")), "post_fix_evidence": array(ref("Ref")),
    })
    defs["OracleResultV1"] = record({
        "oracle_id": ref("Name"), "expected": typed_ref("ExpectationV1"),
        "result": scalar_enum("satisfied", "violated", "unavailable", "error"),
        "detector_id": ref("MaybeName"), "raw_evidence_refs": array(ref("Ref")),
        "reason": ref("Text"),
    })
    defs["MissingObjectV1"] = record({
        "artifact_id": ref("Id"), "expected_sha256": ref("Sha"),
        "expected_byte_length": ref("Count"), "expected_schema_id": ref("Name"),
        "reason": ref("Reason"),
    }, "Expected identity of unavailable bytes, not a resolving Ref or an import grant.")
    for row in (
        "Case", "SubjectFile", "Lock", "Dependency", "Tool", "LanguageFlags", "Standard",
        "Environment", "Grant", "Effect", "Resource", "Obligation", "Diagnostic", "Artifact",
        "Campaign", "Mutant", "Assumption", "Finding", "MissingObject",
    ):
        name = row + "PageV1"
        page = record({
            "page_index": ref("Count"), "page_count": {"allOf": [ref("Count"), {"minimum": 1}]},
            "row_count": {"allOf": [ref("Count"), {"maximum": 256}]},
            "total_rows": ref("Count"), "rows": array(ref(row + "V1")),
            "next": maybe(typed_ref(name)),
        }, "Runtime verifies <=1 MiB exact bytes, index/count/length arithmetic, complete "
           "acyclic same-type chain, no repeated page/row identities and no omitted rows.")
        page["allOf"] = [
            {"if": properties(next=absent()),
             "then": properties(next=properties(unavailable_reason={"const": "end_of_inventory"}))},
            {"if": properties(page_count={"const": 1}),
             "then": properties(page_index={"const": 0}, next=absent())},
            {"if": properties(total_rows={"const": 0}),
             "then": properties(page_index={"const": 0}, page_count={"const": 1},
                                row_count={"const": 0}, rows={"maxItems": 0}, next=absent()),
             "else": properties(row_count={"minimum": 1}, rows={"minItems": 1})},
        ]
        defs[name] = page
    defs["ReviewReceiptV1"] = record({
        "protocol": {"const": "hee3.receipt.review"}, "version": {"type": "integer", "const": 1},
        "receipt": typed_ref("ReceiptV1"), "review": ref("ReviewV1"),
    }, "Append separately; review.subject_sha256 must equal the referenced prior receipt digest.")
    defs["AvailabilityReceiptV1"] = record({
        "protocol": {"const": "hee3.receipt.availability"}, "version": {"type": "integer", "const": 1},
        "receipt": typed_ref("ReceiptV1"), "availability": ref("AvailabilityV1"),
    }, "Append a new observation; do not replace or mutate prior receipt bytes.")
    targets = {
        "SubjectsV1": {
            **{name: "SubjectV1" for name in (
                "seed_subject", "fixtures", "oracle", "harness", "collector", "launcher",
            )},
            "locks": "LockPageV1", "toolchain": "ToolPageV1",
            "target_features_build_profile": "BuildProfileV1", "standards": "StandardPageV1",
        },
        "InvocationV1": {
            "environment": "EnvironmentPageV1", "grants": "GrantPageV1",
            "expected": "ExpectationV1", "limits": "LimitsV1", "allowed_effects": "EffectPageV1",
            "cleanup_contract": "CleanupContractV1",
        },
        "ObservationsV1": {
            "host": "HostV1", "resources": "ResourcePageV1", "unresolved_obligations": "ObligationPageV1",
        },
        "CasesV1": {"inventory": "CasePageV1"},
        "DiagnosticsV1": {"by_tool": "DiagnosticPageV1"},
        "ArtifactsV1": {"inventory": "ArtifactPageV1"},
        "MutationV1": {"campaigns": "CampaignPageV1"},
        "ReviewV1": {
            "shared_assumptions": "AssumptionPageV1", "findings": "FindingPageV1",
            "residual_obligations": "ObligationPageV1",
        },
        "VerdictV1": {"oracle_result": "OracleResultV1"},
        "AvailabilityV1": {"missing_objects": "MissingObjectPageV1"},
    }
    for name, fields in targets.items():
        defs[name]["properties"].update({field: typed_ref(target) for field, target in fields.items()})
    defs["SubjectsV1"]["properties"].update({
        "result_subject": maybe(typed_ref("SubjectV1")),
        "seed_to_result_patch": maybe(ref("Payload")), "isolation_profile": ref("Payload"),
    })
    defs["ProducerV1"]["properties"].update({
        "stdout": maybe(ref("Payload")), "stderr": maybe(ref("Payload")),
    })
    defs["ObservationsV1"]["properties"].update({
        "cancellation": scalar_enum("not_requested", "requested", "unknown"),
        "cleanup": scalar_enum("not_started", "pending", "settled", "failed", "unknown"),
    })
    defs["ReceiptV1"]["properties"]["review"] = {"allOf": [ref("MaybeReviewV1"), absent()]}
    defs["ReceiptV1"]["allOf"][0]["then"]["properties"]["observations"]["properties"].update({
        "cancellation": {"const": "not_requested"}, "cleanup": {"const": "settled"},
    })
    addressable = sorted(name for name, schema in defs.items()
                         if name.endswith("V1") and not name.startswith("Maybe")
                         and schema.get("type") == "object")
    defs["Ref"]["properties"]["schema_id"] = {
        "allOf": [ref("Name"), {"enum": ["hee3.raw/1", *(
            "hee3.receipt/1:" + name for name in addressable
        )]}],
    }


def build_schema():
    defs = {
        "Id": {
            "type": "string", "minLength": 36, "maxLength": 36,
            "pattern": "^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-"
                       "[89ab][0-9a-f]{3}-[0-9a-f]{12}$",
        },
        "U64": decimal_u64(),
        "Generation": {"allOf": [ref("U64"), {"not": {"const": "0"}}]},
        "Sha": {
            "type": "string", "minLength": 71, "maxLength": 71,
            "pattern": "^sha256:[0-9a-f]{64}$",
        },
        "Text": {
            "type": "string", "maxLength": 4096,
            "$comment": "Necessary code-point bound only. Runtime must enforce "
                        "valid UTF-8 and <=4096 encoded bytes.",
        },
        "Name": {
            "type": "string", "minLength": 1, "maxLength": 128,
            "not": {"pattern": r"[^\u0000-\u007f]"},
            "$comment": "RC04 says ASCII, not printable-only ASCII.",
        },
        "Count": {"type": "integer", "minimum": 0, "maximum": 4294967295},
        "Bool": {"type": "boolean"},
        "Ref": record({
            "artifact_id": ref("Id"), "sha256": ref("Sha"),
            "byte_length": ref("Count"), "media_type": ref("Name"),
            "schema_id": ref("Name"),
        }, "EvidenceRefV1 from RC03. Shape validation does not resolve or authenticate it."),
    }
    for name in ("Id", "Count", "Ref", "Text", "ReviewV1"):
        defs["Maybe" + name] = maybe(ref(name))
    defs["IdentityV1"] = record({
        "run_id": ref("Id"), "task_id": ref("Id"), "attempt_id": ref("Id"),
        "generation": ref("Generation"), "module_id": ref("Name"),
        "criterion_ids": array(ref("Name")), "profile_id": ref("Name"),
        "parent_run": ref("MaybeId"),
    })
    defs["SubjectsV1"] = record({
        "seed_subject": ref("Ref"), "result_subject": ref("MaybeRef"),
        "seed_to_result_patch": ref("MaybeRef"),
        **{name: ref("Ref") for name in (
            "fixtures", "oracle", "harness", "collector", "launcher", "locks",
            "toolchain", "target_features_build_profile", "standards", "isolation_profile",
        )},
    })
    defs["InvocationV1"] = record({
        "argv": array(ref("Text")), "cwd_logical": ref("Name"),
        "environment": ref("Ref"), "grants": ref("Ref"), "expected": ref("Ref"),
        "oracle_id": ref("Name"), "limits": ref("Ref"), "allowed_effects": ref("Ref"),
        "cleanup_contract": ref("Ref"),
    })
    defs["ObservationsV1"] = record({
        "host": ref("Ref"),
        **{name: ref("U64") for name in (
            "start_unix_ms", "end_unix_ms", "start_monotonic_ns", "end_monotonic_ns",
            "cutoff_unix_ms",
        )},
        "resources": ref("Ref"), "producer": ref("ProducerV1"),
        "cancellation": ref("Name"), "cleanup": ref("Name"),
        "unresolved_obligations": ref("Ref"),
    })
    producer = record({
        "status": scalar_enum("exited", "signalled", "not_started", "unknown"),
        "exit_code": ref("MaybeCount"), "signal": ref("MaybeCount"),
        "timeout": ref("Bool"), "stdout": ref("MaybeRef"), "stderr": ref("MaybeRef"),
    })
    producer["oneOf"] = [
        properties(status={"const": "exited"}, exit_code=present(), signal=absent()),
        properties(status={"const": "signalled"}, exit_code=absent(), signal=present()),
        properties(status=scalar_enum("not_started", "unknown"),
                   exit_code=absent(), signal=absent()),
    ]
    defs["ProducerV1"] = producer
    defs["CasesV1"] = record({
        "inventory": ref("Ref"),
        **{name: ref("Count") for name in (
            "discovered", "selected", "executed", "passed", "failed", "skipped",
            "ignored", "broken", "timed_out", "invalid", "excluded", "unmeasured",
            "primary_credit",
        )},
    }, "Counts require complete inventory recomputation; aggregate arithmetic is not validated.")
    defs["DiagnosticsV1"] = record({
        "baseline": ref("Bool"), "warning_count": ref("Count"),
        "error_count": ref("Count"), "by_tool": ref("Ref"),
        "stdout_truncated": ref("Bool"), "stderr_truncated": ref("Bool"),
        "mismatch": ref("MaybeText"),
    })
    defs["ArtifactsV1"] = record({
        "inventory": ref("Ref"), "count": ref("Count"), "total_bytes": ref("U64"),
        "finalized": ref("Bool"),
    })
    defs["MutationV1"] = record({
        "campaigns": ref("Ref"),
        **{name: ref("Count") for name in (
            "campaign_count", "distinct_mutants", "caught", "survived", "timed_out",
            "unviable", "equivalent", "excluded", "unmeasured",
        )},
    }, "Mutant executions are never behavioral-case credits. Campaign sufficiency is runtime.")
    defs["ReviewV1"] = record({
        "reviewer": ref("Text"), "effective_model": ref("MaybeText"),
        "action": ref("Name"), "subject_sha256": ref("Sha"),
        "shared_assumptions": ref("Ref"), "findings": ref("Ref"),
        "disposition": ref("Name"), "residual_obligations": ref("Ref"),
        "pre_fix_evidence": array(ref("Ref")), "post_fix_evidence": array(ref("Ref")),
    })
    defs["VerdictV1"] = record({
        "state": scalar_enum("PASS_CANDIDATE", "FAIL", "INVALID", "ERROR", "TIMEOUT",
                             "CANCELLED", "UNMEASURED"),
        "oracle_result": ref("Ref"), "intended_detector": ref("Name"),
        "benign_pair": ref("MaybeId"), "reasons": array(ref("Text")),
    })
    defs["AvailabilityV1"] = record({
        "observed_unix_ms": ref("U64"),
        "state": scalar_enum("complete", "incomplete", "missing"),
        "missing_objects": ref("Ref"), "retention_policy": {"const": "retain-v1"},
    })
    receipt = record({
        "protocol": {"const": "hee3.receipt"},
        "version": {"type": "integer", "const": 1},
        "serialization": {"const": "json-exact-v1"}, "schema_sha256": ref("Sha"),
        **{name: ref(name.title() + "V1") for name in (
            "identity", "subjects", "invocation", "observations", "cases", "diagnostics",
            "artifacts", "mutation",
        )},
        "review": ref("MaybeReviewV1"), "verdict": ref("VerdictV1"),
        "availability": ref("AvailabilityV1"),
    })
    receipt["allOf"] = [{
        "if": properties(verdict=properties(state={"const": "PASS_CANDIDATE"})),
        "then": properties(
            subjects=properties(result_subject=present(), seed_to_result_patch=present()),
            observations=properties(producer=properties(
                status={"const": "exited"}, timeout={"const": False},
                stdout=present(), stderr=present(),
            )),
            cases=properties(
                selected={"minimum": 1}, executed={"minimum": 1},
            ),
            diagnostics={
                **properties(stdout_truncated={"const": False},
                             stderr_truncated={"const": False}, mismatch=absent()),
                "if": properties(baseline={"const": True}),
                "then": properties(warning_count={"const": 0}, error_count={"const": 0}),
            },
            artifacts=properties(finalized={"const": True}),
            availability=properties(state={"const": "complete"}),
        ),
        "$comment": "Necessary structural guards only. A deliberate negative control may "
                    "have a nonzero expected exit; runtime verifies that exact outcome and "
                    "isolated fault diagnostics. Aggregate inventories and cleanup truth "
                    "need separate validation. Exhaustive outcome totals may include "
                    "unselected/nonmandatory rows; selected mandatory rows must pass, "
                    "which is checked after inventory resolution. No module admission claim.",
    }]
    defs["ReceiptV1"] = receipt
    add_inventory_definitions(defs)
    return {
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "$id": "urn:hee3:receipt:1",
        "title": "hee3.receipt/1 closed receipt and inventory schema bundle",
        "$comment": "Candidate contract artifact, not a published compatibility identity. "
                    "RC04 revision 4 owns the fixed types and reference addressing. "
                    "The default entry point is ReceiptV1; named referenced objects and "
                    "append envelopes validate through their exact addressable $defs entry.",
        "x-hee3-authority": {"decision": "RC04", "readiness_revision": 4},
        "x-hee3-bundle-complete": True,
        "x-hee3-open-decisions": [],
        "x-hee3-runtime-obligations": list(RUNTIME_OBLIGATIONS),
        "$ref": "#/$defs/ReceiptV1", "$defs": defs,
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    destination = Path(__file__).with_name("receipt-v1.schema.json")
    encoded = (json.dumps(build_schema(), ensure_ascii=True, indent=2) + "\n").encode("ascii")
    if args.check:
        if not destination.exists() or destination.read_bytes() != encoded:
            parser.exit(1, "receipt schema artifact is absent or differs from its authoring source\n")
        print("receipt schema exact-byte check passed; structural bundle only, no collector admission")
    else:
        destination.write_bytes(encoded)


if __name__ == "__main__":
    main()
