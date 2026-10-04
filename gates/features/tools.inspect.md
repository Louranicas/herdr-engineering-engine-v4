# tools.inspect

A-03. One catalogue entry in full: purpose, effect, the SHA-256 of its request, result and error schemas, its byte and deadline bounds, and the read action a caller uses after a lost reply. Scope v4.0, phase P1 (INTERP as `tools.list`), owner `Owner::Actions`, effect `Read`.

## Sub-features

- entry-detail: `{action, version, purpose, effect, request_schema_sha256, result_schema_sha256, error_schema_sha256, max_request_bytes, max_deadline_ms, readback_action}`.
- schema-digests: the digests of the K0-emitted schemas; `hee4-sh --inspect <action>` must show the same arguments for the same digests.
- readback-selector-source: `readback_action` is where a client learns which read to issue after a lost reply (CD RC03 §6 "Readback selectors"); mutating actions name one, reads name none.
- bounds-per-action: `max_request_bytes` and `max_deadline_ms` from K0 bounds, never a literal in the client.
- unknown-and-version: an unknown id, or a known id at an unsupported version, is refused by name.

## How to get to it (user POV)

`hee4 tools.inspect` (binary), `hee4-sh tools.inspect action=<id> version:=1`, Pi `hee4_tools_inspect` (PROPOSAL). For a wrapper-side view of the same data, `hee4-sh --inspect <action>`.

## Driving it with hee4

Preconditions: README shared preconditions; a `Read` grant.

```bash
hee4 tools.inspect
hee4-sh tools.inspect action=task.submit version:=1
hee4-sh tools.inspect action=judge.inspect version:=1
hee4-sh tools.inspect action=no.such.action version:=1
hee4-sh --inspect task.submit
```

Socket: request `body` `{action, version}` (FACT `BodyRequest_tools_inspect` required both); result `body` `{action, version, purpose, effect, request_schema_sha256, result_schema_sha256, error_schema_sha256, max_request_bytes, max_deadline_ms, readback_action}` (API Map A-03).

- Success: for `task.submit`, `effect` is `DurableAdmission` and `readback_action` is `task.get`; for `health`, `readback_action` is null. `max_deadline_ms` ≤ 60000 for every action (deadline window, Error map class 7).
- Held id: `judge.inspect` inspects successfully (it is catalogued) while invoking it is `unavailable` (judge.inspect.md). Assert both in one run.
- Error: unknown id → `unknown_action` at `/body/action` (Error map F-2: `tools.inspect` calls the same `Catalogue::find`, and its case asserts its own `field`). Known id, wrong `version` → `unsupported_action_version` (F-3).
- Empty: none; both fields are required, a missing one is `invalid_argument` at its pointer.
- Side effect: none; no `operations` row.
- `UNWRITTEN: whether the refusal for an unknown id is unknown_action (Error map F-2, the one Catalogue::find site) or not_found (API Map A-03 "Specific refusals", INTERP). The two maps disagree; the P1 slice names one.`

## Gotchas

- The API Map and the Error map name different codes for a missing action (see the UNWRITTEN above). Do not pin a test to either until the slice decides; pin the `field` pointer `/body/action`, which both agree on.
- v3 raised `unknown_action` from three sites and `internal` from four for catalogue disagreement (Error map F-2, F-5). v4 has no `internal` for that at run time: catalogue = registry is a build-time digest test (card actions §9 #3). An `internal` from this action is a defect, not a path to cover.
- Schema digests differ from v3's by construction (`queued` removed; protocol renamed `hee4.control`, API Map §3 note). A digest equal to a v3 schema is wrong.
- Must not: the client must not carry its own bounds (UM-P5); a wrapper that refuses at 60 s does so from the emitted schema, which is what this action's `max_deadline_ms` shows.
