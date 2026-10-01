#!/usr/bin/env python3
"""Discover, load and revise skill packages under HEE3-IF-skills.

The schema states a manifest's shape. It cannot state that a path stays inside its package,
that a dependency is present, that a reference still hashes to what was reviewed, or that a
skill has not asked for authority its caller does not hold -- so those are obligations of
this loader, and each is named in the schema's `hee3.refusals`.

Two rules are structural rather than advisory:

* **A skill cannot widen authority.** `load()` takes the actions the caller ALREADY holds and
  refuses a manifest naming one outside that set. There is no branch that adds an action, so
  "a loaded skill granted itself a capability" is not a state this module can reach.
* **A reference that is not carried is named.** Scope denial, a budget cutoff, a stale or
  absent body and an oversize body all produce an OMISSION carrying the reference id and the
  reason. A packet is never quietly short, because a short packet that looks complete is the
  failure that this module's whole bounding exists to make visible. A traversal bound is not
  an omission: a reference deeper than the bound, or one whose declared depth disagrees with
  its path, refuses the whole manifest as `traversal_budget`.

Nothing here executes instruction text, grants an action, accepts a task or changes a task's
criteria. Text is not execution approval.
"""

import ast
import hashlib
import json
import os
import stat
from pathlib import Path

HERE = Path(__file__).resolve().parent
SCHEMA_PATH = HERE / "skill-v1.schema.json"


class SkillRefusal(Exception):
    """A skill was refused. `code` is one of the schema's `hee3.refusals`."""

    def __init__(self, code, detail):
        super().__init__(f"{code}: {detail}")
        self.code = code
        self.detail = detail


def refuse(code, detail):
    raise SkillRefusal(code, detail)


def schema():
    return json.loads(SCHEMA_PATH.read_text())


def bounds():
    return schema()["hee3"]["bounds"]


def refusal_sites():
    """Every `refuse(...)` code this file can raise, read from its own syntax tree."""
    return [
        node.args[0].value
        for node in ast.walk(ast.parse(Path(__file__).read_text()))
        if isinstance(node, ast.Call) and isinstance(node.func, ast.Name)
        and node.func.id == "refuse" and node.args and isinstance(node.args[0], ast.Constant)
    ]


def safe_path(reference_id, path):
    """Refuse a reference path that could leave its package.

    The schema's pattern refuses a leading slash. Everything a pattern would express badly is
    here, where it can be read: a parent segment, an empty segment from a doubled or trailing
    separator, and a Windows drive or backslash that a forward-slash pattern would let past.
    """
    if "\\" in path or ":" in path:
        refuse("unsafe_path", f"{reference_id}: {path!r} is not a forward-slashed relative path")
    segments = path.split("/")
    if any(segment in ("", ".", "..") for segment in segments):
        refuse("unsafe_path",
               f"{reference_id}: {path!r} contains an empty, current or parent segment")
    return segments


def require_fields(record, required, where):
    """Refuse a record lacking a field the schema requires, before anything indexes it."""
    missing = [field for field in required if field not in record]
    if missing:
        refuse("malformed_manifest", f"{where} lacks required field(s) {missing}")


def structure(manifest):
    """The checks that need only the manifest: shape, bounds, paths, pins and depth.

    `discover()` runs these and then the checks that need the caller's world; `revise()` and
    `read_package()` run these alone. One function, so there is one door per rule.
    """
    if manifest.get("schema") != "hee3.skills.skill.v1":
        refuse("version_mismatch", f"schema is {manifest.get('schema')!r}")
    declared = schema()
    require_fields(manifest, declared["required"], "manifest")

    # The admitted names come from this schema's own enum, which the generator took from
    # the action catalogue. Reading the catalogue again here would be a second door.
    admitted = set(declared["properties"]["requires_actions"]["items"]["enum"])
    required = manifest.get("requires_actions", [])
    if len(required) > bounds()["max_actions"]:
        refuse("incompatible_action",
               f"{len(required)} required actions against a limit of {bounds()['max_actions']}")
    for action in required:
        if action not in admitted:
            refuse("unknown_action", f"{action!r} is not in the action catalogue")

    dependencies = manifest.get("requires_skills", [])
    if len(dependencies) > bounds()["max_dependencies"]:
        refuse("missing_dependency",
               f"{len(dependencies)} dependencies against a limit of "
               f"{bounds()['max_dependencies']}")
    pins = {}
    for index, dependency in enumerate(dependencies):
        require_fields(dependency, declared["$defs"]["dependency"]["required"],
                       f"dependency {index}")
        pins.setdefault(dependency["skill_id"], []).append(dependency["skill_version"])
    for skill_id, versions in sorted(pins.items()):
        if len(versions) > 1:
            # Schema `uniqueItems` compares whole objects, so {a, 1} and {a, 2} both pass it.
            # Two pins for one skill cannot both be the one this skill was reviewed against.
            refuse("conflicting_dependency",
                   f"{skill_id} is pinned {len(versions)} times, at versions {sorted(versions)}")

    references = manifest.get("references", [])
    if len(references) > bounds()["max_references"]:
        refuse("traversal_budget",
               f"{len(references)} references against a limit of {bounds()['max_references']}")
    seen = set()
    for index, reference in enumerate(references):
        require_fields(reference, declared["$defs"]["reference"]["required"],
                       f"reference {index}")
        identity = reference["reference_id"]
        if identity in seen:
            refuse("duplicate_reference", f"{identity} appears twice")
        seen.add(identity)
        # The depth is the PATH's. A declared `depth` is a claim the manifest makes about
        # itself, so it may only agree: were it allowed to override, the bound would be set
        # by the thing it bounds.
        depth = len(safe_path(identity, reference["path"])) - 1
        if depth > bounds()["max_depth"]:
            refuse("traversal_budget",
                   f"{identity}: depth {depth} against a limit of {bounds()['max_depth']}")
        if reference.get("depth", depth) != depth:
            refuse("traversal_budget",
                   f"{identity}: declares depth {reference['depth']} but its path is "
                   f"{depth} deep")
    return required, dependencies


def discover(manifest, held_actions, available_skills):
    """Whether this manifest is loadable here, refusing by symbol when it is not.

    `held_actions` is what the caller already holds; `available_skills` maps skill_id to the
    set of versions present. Returns the manifest's compatibility summary.
    """
    required, dependencies = structure(manifest)
    if manifest["lifecycle"] == "retired":
        refuse("retired_skill",
               f"{manifest['skill_id']} version {manifest['skill_version']} is retired")
    for action in required:
        if action not in held_actions:
            # The refusal, never a grant: this is the only place a skill's action claim meets
            # the caller's authority, and it can only narrow.
            refuse("authority_widening",
                   f"{manifest['skill_id']} requires {action!r}, which the caller does not hold")
    for dependency in dependencies:
        if not dependency.get("required", True):
            continue
        present = available_skills.get(dependency["skill_id"], set())
        if dependency["skill_version"] not in present:
            refuse("missing_dependency",
                   f"{dependency['skill_id']} version {dependency['skill_version']} is "
                   "required and not present")

    return {
        "skill_id": manifest["skill_id"],
        "skill_version": manifest["skill_version"],
        "lifecycle": manifest["lifecycle"],
        "deprecated": manifest["lifecycle"] == "deprecated",
        "requires_actions": sorted(required),
        "reference_count": len(manifest.get("references", [])),
        "requires_skills": sorted(
            ({"skill_id": d["skill_id"], "skill_version": d["skill_version"],
              "required": d.get("required", True)} for d in dependencies),
            key=lambda pin: pin["skill_id"]),
    }


def read_package(root, manifest):
    """Acquire each reference's bytes from under `root`, returning the map `load()` takes.

    The bounds hold at the point of acquisition: a path whose resolution leaves `root` --
    through a symlinked file or directory -- is refused as `unsafe_path` before it is opened,
    provided the tree is not changed between resolution and open (O_NOFOLLOW narrows that
    window for the final component only; no test can pin it), and no body is read past `max_reference_bytes + 1`, so an oversize file costs one byte
    over the bound and reaches `load()` as `reference_too_large`. A reference that is absent,
    or is not a regular file, is left out of the map, which `load()` names as
    `stale_reference`. A FIFO is opened non-blocking and never read.
    """
    structure(manifest)
    base = Path(root).resolve()
    ceiling = bounds()["max_reference_bytes"] + 1
    contents = {}
    for reference in manifest.get("references", []):
        identity = reference["reference_id"]
        resolved = (base / reference["path"]).resolve()
        if not resolved.is_relative_to(base):
            refuse("unsafe_path",
                   f"{identity}: {reference['path']!r} resolves outside the package root")
        try:
            descriptor = os.open(resolved, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK)
        except OSError:
            continue
        try:
            # Judged on the descriptor itself: a file object cannot be built over a directory.
            if not stat.S_ISREG(os.fstat(descriptor).st_mode):
                continue
            with os.fdopen(descriptor, "rb", closefd=False) as handle:
                contents[identity] = handle.read(ceiling)
        finally:
            os.close(descriptor)
    return contents


def load(manifest, held_actions, available_skills, scopes, contents, budget_bytes):
    """Build a bounded instruction packet, stating everything it did not carry.

    `scopes` is what the caller may see; `contents` maps reference_id to bytes. The packet
    records provenance, omissions and cost, so a reader can tell a complete answer from a
    budgeted one without asking.
    """
    summary = discover(manifest, held_actions, available_skills)
    limit = min(budget_bytes, bounds()["max_packet_bytes"])
    entry = manifest["entry"].encode()
    if len(entry) > limit:
        # The entry is the skill; a packet without it would be a different skill that looks
        # like this one.
        refuse("context_budget",
               f"entry instructions are {len(entry)} bytes against a budget of {limit}")

    carried, omissions, used = [], [], len(entry)
    for reference in manifest.get("references", []):
        identity = reference["reference_id"]
        if not set(reference["scope"]) & set(scopes):
            omissions.append({"reference_id": identity, "reason": "denied_scope"})
            continue
        body = contents.get(identity)
        if body is None:
            omissions.append({"reference_id": identity, "reason": "stale_reference"})
            continue
        if len(body) > bounds()["max_reference_bytes"]:
            omissions.append({"reference_id": identity, "reason": "reference_too_large"})
            continue
        expected = reference.get("sha256")
        if expected is not None and hashlib.sha256(body).hexdigest() != expected:
            omissions.append({"reference_id": identity, "reason": "stale_reference"})
            continue
        if used + len(body) > limit:
            omissions.append({"reference_id": identity, "reason": "context_budget"})
            continue
        used += len(body)
        # Source provenance: what was carried, as it hashes now, and whether that hash is one
        # a reviewer recorded. Unhashed content is carried and says it is unverified.
        carried.append({"reference_id": identity, "path": reference["path"],
                        "bytes": len(body), "sha256": hashlib.sha256(body).hexdigest(),
                        "verified": expected is not None})

    return {
        "schema": "hee3.skills.packet.v1",
        "skill_id": summary["skill_id"],
        "skill_version": summary["skill_version"],
        "deprecated": summary["deprecated"],
        # The packet repeats what the CALLER holds, not what the skill asked for. A reader
        # of a packet can never mistake it for a grant, because no wider set appears in it.
        "actions_in_effect": sorted(set(held_actions) & set(summary["requires_actions"])),
        # Relationship provenance: the exact pins this skill was loaded against.
        "requires_skills": summary["requires_skills"],
        "entry_bytes": len(entry),
        "references": carried,
        "omissions": omissions,
        "cost_bytes": used,
        "budget_bytes": limit,
        "complete": not omissions,
        "scope": "Instructions only. No grant, acceptance, criterion change or effect.",
    }


def revise(previous, proposed):
    """Record a revision, refusing one that is not a forward, immutable step.

    Both manifests pass the structural checks first, so a revision record is never written
    over a manifest that could not load. Returns the drift record a consumer needs: what
    changed, including dependency pins, and which pins are affected.
    """
    structure(previous)
    structure(proposed)
    if proposed["skill_id"] != previous["skill_id"]:
        refuse("version_mismatch",
               f"{proposed['skill_id']!r} is not a revision of {previous['skill_id']!r}")
    if proposed["skill_version"] <= previous["skill_version"]:
        refuse("version_mismatch",
               f"version {proposed['skill_version']} does not follow "
               f"{previous['skill_version']}; a published revision is never edited in place")
    if previous["lifecycle"] == "retired":
        refuse("retired_skill", f"{previous['skill_id']} is retired and takes no revision")
    before = {r["reference_id"]: r.get("sha256") for r in previous.get("references", [])}
    after = {r["reference_id"]: r.get("sha256") for r in proposed.get("references", [])}
    return {
        "skill_id": proposed["skill_id"],
        "from_version": previous["skill_version"],
        "to_version": proposed["skill_version"],
        "entry_changed": previous["entry"] != proposed["entry"],
        "references_added": sorted(set(after) - set(before)),
        "references_removed": sorted(set(before) - set(after)),
        "references_changed": sorted(k for k in set(before) & set(after)
                                     if before[k] != after[k]),
        "actions_added": sorted(set(proposed.get("requires_actions", []))
                                - set(previous.get("requires_actions", []))),
        "actions_removed": sorted(set(previous.get("requires_actions", []))
                                  - set(proposed.get("requires_actions", []))),
        # `structure()` refused a skill pinned twice, so skill_id keys each side exactly.
        **pin_drift(previous.get("requires_skills", []), proposed.get("requires_skills", [])),
        "affected_consumers": "Every consumer pinning "
                              f"{previous['skill_id']} v{previous['skill_version']}",
    }


def pin_drift(before, after):
    """Which dependency pins a revision added, removed, or changed in version or requirement."""
    old = {d["skill_id"]: (d["skill_version"], d.get("required", True)) for d in before}
    new = {d["skill_id"]: (d["skill_version"], d.get("required", True)) for d in after}
    return {
        "dependencies_added": sorted(set(new) - set(old)),
        "dependencies_removed": sorted(set(old) - set(new)),
        "dependencies_changed": sorted(k for k in set(old) & set(new) if old[k] != new[k]),
    }
