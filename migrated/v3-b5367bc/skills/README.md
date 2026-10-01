# Skills

Versioned packages of instructions and bounded references. A skill supplies **text, never
authority**: nothing here grants an action, accepts a task, changes a task's criteria, or
starts anything.

| File | What it is |
|---|---|
| `skill-v1.schema.json` | The manifest shape. A build output. |
| `generate_skill_schema.py` | Authors it. `--check` compares exact bytes. |
| `load_skill.py` | Discovery, acquisition, loading and revision — the obligations a schema cannot state. |
| `examples/receipt-reading/` | A reference package, version 2, superseding 1. |
| `../tests/skill_schema.py` | 69 cases. |

## A skill cannot widen authority

`load()` takes the actions the caller **already holds**. A manifest naming one outside that
set is refused as `authority_widening`, and there is no branch that adds an action — so "a
loaded skill granted itself a capability" is not a state this module can reach. The packet
reports `actions_in_effect`, which is the *intersection*: no set wider than the caller's own
appears anywhere in it.

The test for this is deliberately not a search for the word "grant". The packet's own scope
line says "No grant", so a substring test passes whether or not the rule holds — a needle the
output carries either way. The checkable claim is the subset relation, asserted over four
scope sets.

## What is not carried is named

Scope denial, a budget cutoff, a stale or absent body and a reference over its size bound all
produce an **omission** carrying the reference id and the reason. A packet is never quietly
short, because a short packet that looks complete is the failure that bounding exists to make
visible. `complete` is `false` whenever anything was left out. A test reads every omission
reason `load_skill.py` writes from its syntax tree and requires it to equal
`hee3.omission_reasons`.

`denied_scope` and `stale_reference` are omission reasons and deliberately **not** refusals:
refusing a whole load over one reference would throw away the instructions the caller can
legitimately have. The two sets share exactly one code, `context_budget`, and a test asserts
that intersection: an entry over budget refuses the load (a packet without its entry is a
different skill), while a reference over budget is omitted.

A traversal bound is **not** an omission. A reference's depth is the depth of its path; a
declared `depth` may only agree with it. A path deeper than `max_depth`, a declared depth that
disagrees with the path, or more than `max_references` references refuses the manifest as
`traversal_budget` — otherwise the bound would be set by the thing it bounds.

## What a packet says about where it came from

Each carried reference states its `sha256` as it hashes now and `verified`: `true` only when
the manifest recorded a reviewed hash and the bytes match it. Unhashed content is carried and
marked unverified; mismatched content is omitted as `stale_reference`. The packet's
`requires_skills` lists the exact `(skill_id, skill_version, required)` pins it was loaded
against — its relationship provenance.

## Pins are exact

A dependency names `skill_id` and `skill_version` exactly — never a range. A range would let
a dependency change under a consumer that pinned this skill precisely to stop that. A
published revision is immutable; `revise()` refuses a version that does not advance, and
returns the drift record naming what changed and which consumers pinned the old one.

A skill pinned twice — `{a, 1}` and `{a, 2}`, which schema `uniqueItems` admits because it
compares whole objects — is refused as `conflicting_dependency`: two pins cannot both be the one
the skill was reviewed against. `revise()` runs the same structural checks on both manifests
and its drift record names `dependencies_added`, `dependencies_removed` and
`dependencies_changed` (version or `required` flag).

A reference carries the `sha256` it was reviewed against. Content that no longer hashes to it
is omitted as `stale_reference`, never silently substituted.

## Conflicting instructions

A dependency-pin conflict is the decidable conflict, and it refuses. A **textual** conflict —
instruction text that contradicts the caller, claims an action or tries to accept a task — is
not detected, because the loader never interprets text. It is surfaced only by the no-grant
invariant: `actions_in_effect` is the caller's held set intersected with the skill's claim, so
no text can change it. A test loads a hostile entry beside a benign one and requires the two
packets to differ only in byte counts.

## Acquisition

`read_package(root, manifest)` reads each reference from under `root` and returns the
`contents` map `load()` takes. The bounds hold where the bytes are acquired: a path that
resolves outside `root` — through a symlinked file or directory — is refused as `unsafe_path`
before it is opened — provided nothing swaps a path component between resolution and open;
`O_NOFOLLOW` narrows that window for the final component only, and no deterministic test pins
it — and no file is read past `max_reference_bytes + 1`, so an oversize file
reaches `load()` as `reference_too_large`. An absent reference, or one that is not a regular
file, is left out of the map and named by `load()` as `stale_reference`. A FIFO is opened
non-blocking and never read, so a named pipe cannot stall a load.

## Why the case count is not the evidence

`tools/check-skill-sites` neuters each refusal site in `load_skill.py` — enumerated from the
file's own syntax tree — and requires the suite to go red. Its first run read `sites=15
killed=14 survived=1`: the dependency *count* bound was unpinned, because the case fed it
seventeen dependencies that were all absent, so the per-dependency check answered instead and
the same code came back. The case now supplies every dependency as present, leaving only the
count able to fire. `sites=15 killed=15 survived=0`. After the structural checks, pin conflicts,
acquisition and derived depth were added (2026-09-24): `sites=20 killed=20 survived=0`.

Counting refusal *names* would have read `12/12`, while five of the twelve are raised from
more than one place — `unsafe_path`, `version_mismatch` and `traversal_budget` from three each.

## Consumer integration

A library, called in process; nothing to install or start.

```python
contents = load_skill.read_package(package_root, manifest)            # bounded acquisition
summary = load_skill.discover(manifest, held_actions, available_skills)  # or SkillRefusal
packet = load_skill.load(manifest, held_actions, available_skills,
                         scopes, contents, budget_bytes)              # omissions named
record = load_skill.revise(previous_manifest, proposed_manifest)      # drift record
```

Every refusal is a `SkillRefusal` whose `code` is one of `hee3.refusals`; a caller branches on
the code, never the message. A packet is data, not a grant: a consumer acts only with the
authority it already held.

A packet's `actions_in_effect` is what `workflows/validate_procedure.py`'s `dispatch()` takes
as `held`, so a skill narrows the procedure run under it: a step outside the packet is refused
`authority_widening` before any request is built. The executed composition (T29) is
`tests/fixtures/t29/compose.py`, run against the engine by `tests/t28_socket.rs`; its other end
is `workflows/PROCEDURES.md` § Dispatch and readback.

## Gate dispositions

- **G07 (recovery).** Inapplicable beyond explicit errors: the module holds no state, starts no
  daemon and performs no effect, so there is nothing to replay, cancel or reconcile. Every
  failure is a declared `SkillRefusal` or a named omission.
- **G08 (data and compatibility).** No persistent state, so the migration subcheck is
  inapplicable. The one versioned format is the manifest: `skill-v1` is frozen — its schema is
  a build output compared byte-for-byte — and any new or changed field means `skill-v2`, which
  this loader refuses as `version_mismatch`. A published skill version is immutable; `revise()`
  refuses a version that does not advance.
- **G09 (performance).** No latency, throughput or accelerator budget applies: every operation
  is bounded by count and byte limits in `hee3.bounds` (at most 32 references of at most 64 KiB
  each, a packet of at most 256 KiB). The context-cost budget across skills is owned by T11.
- **G11 (consumer readiness).** Integration is the four calls above; no daemon, socket or
  install step.

## Scope

Instructions and bounded references. Grants nothing, accepts nothing, rewrites no task
criterion, widens no authority, and starts no daemon.
