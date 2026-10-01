#!/usr/bin/env python3
"""Author the HEE3-IF-skills package manifest schema; this is not an engine validator.

Source: `corpus/public-interfaces.json` → `skills`, whose operations are *"Discover compatible
skill"*, *"Load scoped instructions"* and *"Update a skill revision"*, bounded by *"Bound
reference traversal and reject unsafe paths, missing required dependencies and incompatible
actions. Text is not execution approval."*

Run without arguments to write the adjacent artifact, or `--check` to compare exact bytes.
Only Python's standard library is needed. No transport or effects are started.

Like `workflows/generate_procedure_schema.py`, the action vocabulary is READ FROM
`schemas/actions/control-v1.schema.json` rather than retyped, so a skill cannot declare an
action the engine does not have, and a catalogue change is visible here without anyone
remembering to look.
"""

import argparse
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent
ARTIFACT = HERE / "skill-v1.schema.json"
CATALOGUE = ROOT / "schemas/actions/control-v1.schema.json"

MAX_REFERENCES = 32
MAX_DEPENDENCIES = 16
MAX_ACTIONS = 32
MAX_DEPTH = 4
MAX_PACKET_BYTES = 262_144
MAX_REFERENCE_BYTES = 65_536
MAX_IDENTIFIER = 128
MAX_TEXT = 8192

LIFECYCLE = ("draft", "published", "deprecated", "retired")
# `traversal_budget` is deliberately ABSENT: a reference deeper than the bound refuses the
# manifest, because the path is the manifest's own statement and not the caller's circumstance.
OMISSION_REASONS = (
    "context_budget", "denied_scope", "stale_reference", "reference_too_large",
)
# `denied_scope` and `stale_reference` are deliberately ABSENT: they are omission reasons,
# not refusals. A reference the caller may not see, or whose bytes no longer match what was
# reviewed, is named in the packet and left out -- refusing the whole load over one reference
# would throw away the instructions the caller can legitimately have.
REFUSALS = (
    "unsafe_path", "missing_dependency", "incompatible_action", "unknown_action",
    "traversal_budget", "context_budget", "retired_skill", "version_mismatch",
    "duplicate_reference", "authority_widening", "malformed_manifest",
    "conflicting_dependency",
)


def catalogue_actions():
    """The admitted action names, taken from the schema that defines them."""
    defs = json.loads(CATALOGUE.read_text()).get("$defs", {})
    node = defs.get("ActionId")
    if isinstance(node, dict) and isinstance(node.get("enum"), list) and node["enum"]:
        return tuple(node["enum"])
    raise SystemExit(
        f"{CATALOGUE} does not expose an action enum; the skill schema will not invent one, "
        "because a manifest that may declare any action name is the hole it exists to close"
    )


def ident(description):
    return {"type": "string", "minLength": 1, "maxLength": MAX_IDENTIFIER,
            "pattern": "^[a-z0-9][a-z0-9._-]*$", "description": description}


def text(description, maximum=MAX_TEXT):
    return {"type": "string", "minLength": 1, "maxLength": maximum, "description": description}


def build():
    actions = catalogue_actions()
    return {
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "$id": "https://hee3.local/schemas/skills/skill-v1.schema.json",
        "title": "HEE3 skill package manifest, version 1",
        "description": (
            "A versioned package of instructions and bounded references. A skill supplies "
            "text, never authority: nothing in this document grants an action, accepts a "
            "task, changes a task's criteria or starts anything."
        ),
        "type": "object",
        "additionalProperties": False,
        "required": ["schema", "skill_id", "skill_version", "lifecycle", "purpose", "entry"],
        "properties": {
            "schema": {"type": "string", "const": "hee3.skills.skill.v1"},
            "skill_id": ident("Stable identity across revisions."),
            "skill_version": {
                "type": "integer", "minimum": 1,
                "description": (
                    "Monotonic and immutable: a published revision is never edited in "
                    "place, so a consumer that pinned one keeps what it pinned."
                ),
            },
            "lifecycle": {
                "type": "string", "enum": list(LIFECYCLE),
                "description": (
                    "`retired` is refused at load. A deprecated skill still loads and says "
                    "so, because silently dropping it would hide the drift it signals."
                ),
            },
            "purpose": text("What this skill is for, in one paragraph a reader can check."),
            "supersedes": {
                "type": "array", "maxItems": 8, "uniqueItems": True,
                "items": {"type": "integer", "minimum": 1},
                "description": "Versions this one replaces, for the drift record.",
            },
            "requires_actions": {
                "type": "array", "maxItems": MAX_ACTIONS, "uniqueItems": True,
                "items": {"type": "string", "enum": list(actions)},
                "description": (
                    "Actions the instructions assume. This is a COMPATIBILITY claim, not a "
                    "request: a loader intersects it with what the caller already holds and "
                    "refuses when the skill names something the caller does not — it never "
                    "adds one. That refusal is `authority_widening`."
                ),
            },
            "requires_skills": {
                "type": "array", "maxItems": MAX_DEPENDENCIES, "uniqueItems": True,
                "items": {"$ref": "#/$defs/dependency"},
            },
            "entry": text("The entry instructions, loaded whenever the skill is loaded."),
            "references": {
                "type": "array", "maxItems": MAX_REFERENCES,
                "items": {"$ref": "#/$defs/reference"},
                "description": (
                    f"At most {MAX_REFERENCES} references, each at most "
                    f"{MAX_REFERENCE_BYTES} bytes, traversed at most {MAX_DEPTH} deep. "
                    "Every bound is stated here so the loader and the manifest cannot "
                    "disagree about what it is."
                ),
            },
        },
        "$defs": {
            "dependency": {
                "type": "object", "additionalProperties": False,
                "required": ["skill_id", "skill_version"],
                "properties": {
                    "skill_id": ident("A skill this one requires."),
                    "skill_version": {
                        "type": "integer", "minimum": 1,
                        "description": (
                            "Pinned exactly. A range would let a dependency change under a "
                            "consumer that pinned this skill precisely to stop that."
                        ),
                    },
                    "required": {"type": "boolean", "default": True},
                },
            },
            "reference": {
                "type": "object", "additionalProperties": False,
                "required": ["reference_id", "path", "scope"],
                "properties": {
                    "reference_id": ident("Stable within this skill; named in omissions."),
                    "path": {
                        "type": "string", "minLength": 1, "maxLength": 512,
                        "pattern": "^[A-Za-z0-9._][A-Za-z0-9._/-]*$",
                        "description": (
                            "Relative, forward-slashed, inside the package. The pattern "
                            "refuses a leading slash; the loader also refuses any `..` "
                            "segment, a trailing slash and a doubled separator, because a "
                            "pattern that tried to express all of that would be the kind "
                            "nobody can read or check."
                        ),
                    },
                    "scope": {
                        "type": "array", "minItems": 1, "maxItems": 16, "uniqueItems": True,
                        "items": ident("A role or task scope permitted to see this."),
                        "description": (
                            "A reference outside the caller's scope is OMITTED BY NAME, "
                            "never silently dropped: the packet says what it did not carry "
                            "and why, so a short answer cannot be mistaken for a complete one."
                        ),
                    },
                    "depth": {"type": "integer", "minimum": 0, "maximum": MAX_DEPTH},
                    "sha256": {
                        "type": "string", "pattern": "^[0-9a-f]{64}$",
                        "description": (
                            "The content this revision was reviewed against. A reference "
                            "whose bytes no longer hash to it is `stale_reference`, not a "
                            "silent substitution."
                        ),
                    },
                },
            },
        },
        "hee3": {
            "lifecycle": list(LIFECYCLE),
            "omission_reasons": list(OMISSION_REASONS),
            "refusals": list(REFUSALS),
            "bounds": {
                "max_references": MAX_REFERENCES, "max_dependencies": MAX_DEPENDENCIES,
                "max_actions": MAX_ACTIONS, "max_depth": MAX_DEPTH,
                "max_packet_bytes": MAX_PACKET_BYTES,
                "max_reference_bytes": MAX_REFERENCE_BYTES,
            },
            "action_catalogue": {
                "source": "schemas/actions/control-v1.schema.json", "count": len(actions),
            },
            "scope": (
                "Instructions and bounded references. Grants nothing, accepts nothing, "
                "rewrites no task criterion and widens no authority."
            ),
        },
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true",
                        help="compare the adjacent artifact byte-for-byte instead of writing it")
    if parser.parse_args().check:
        if not ARTIFACT.exists():
            raise SystemExit(f"{ARTIFACT} is absent")
        if ARTIFACT.read_text() != json.dumps(build(), indent=2) + "\n":
            raise SystemExit(f"{ARTIFACT.name} differs from this generator; regenerate it "
                             "or fix the generator")
        print(f"skill schema: matches_generator=yes actions={len(catalogue_actions())}")
        return
    ARTIFACT.write_text(json.dumps(build(), indent=2) + "\n")
    print(f"wrote {ARTIFACT.name}: actions={len(catalogue_actions())}")


main()
