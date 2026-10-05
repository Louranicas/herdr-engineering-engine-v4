# roster.update

A-13. Create or revise a roster record with a definition and an audit reason, recorded as a revision and an operation. Operator only. Scope **v4.1**, phase P9, owner `Owner::Roster`, effect `ConfigurationMutation` (**mutating**).

## Sub-features

- record-mutation: writes `roster_records`, `roster_revisions`, `operations`.
- change-report: reply `{record, operation_id, change}`.
- idempotent-replay: op key under `roster.update`; readback `roster.inspect {selector:{source_action, idempotency_key}}` (mandatory for a lost create).
- conflict-and-stale: `conflict` (same key, other bytes), `stale_generation` on a stale record generation.
- operator-capability: `forbidden` without the operator capability on the grant.
- retention: observations under the record are capped per principal (card roster §9 #3).
- v4.0-refusal: `unavailable` by name.

## How to get to it (user POV)

`hee4 roster.update` (binary); wrapper "generated"; no Pi tool ("operator only", v3 `tool: None`). At v4.0, `unavailable`. From v4.1 an operator reaches it to add or revise an agent, model or runtime record; the deploy installer does **not** use it (E2E-12).

## Driving it with hee4

Preconditions (v4.1): a `ConfigurationMutation` grant with the operator capability; a `record_id` (new or existing); a fresh UUID.

```bash
K=$(uuidgen)
hee4 roster.update                                                                  # v4.0: unavailable by name
hee4-sh roster.update record_id=<id> 'definition:={…}' audit_reason=<text> @idempotency_key=$K      # v4.1; UNMEASURED: no hee4-sh exists in the six crates
hee4-sh roster.update record_id=<id> 'definition:={…same…}' audit_reason=<text> @idempotency_key=$K # replay; UNMEASURED: no hee4-sh
hee4-sh roster.inspect 'selector:={"source_action":"roster.update","idempotency_key":"'$K'"}'      # readback; UNMEASURED: no hee4-sh
```

Socket: request `body` `{record_id, definition, audit_reason}`; result `body` `{record, operation_id, change}` (API Map A-13).

(rev 2026-10-05 drive) Served from v4.1 by `actions/roster.rs` (`tools/drive.d/roster.py` `d_update`):

```bash
K=$(uuidgen)
D='{"kind":"model","caps":{"ctx_tokens":4096,"json_mode":true,"tool_use":false,"local":true},"cost_milli":0,"latency_ms":0,"quality":1,"capability":null,"locality":"local"}'
hee4 roster.update --key "$K" --body '{"record_id":"model:x","definition":'"$D"',"audit_reason":"add"}'
hee4 roster.update --key "$K" --body '{"record_id":"model:x","definition":'"$D"',"audit_reason":"add"}'   # replay
hee4 roster.update --key "$(uuidgen)" --precondition '{"resource":"roster","id":"model:x","generation":1}' --body '{"record_id":"model:x","definition":'"$D"',"audit_reason":"revise"}'
hee4 roster.inspect --body '{"selector":{"source_action":"roster.update","idempotency_key":"'"$K"'"}}'
```

- Definition schema: `{kind: agent|model|runtime, caps: {ctx_tokens: u32, json_mode: bool, tool_use: bool, local: bool}, cost_milli: u32, latency_ms: u32, quality: u32, capability: null|string, locality: local|remote}`, every member required, unknown members refused. `caps.local` must equal `locality == "local"` (one privacy fact, refused at `/body/definition` when the two disagree).
- Record id: `<kind-word>:<name>` (`^[a-z]+:[A-Za-z0-9._:/-]+$`-shaped, at most MAX_TOKEN_BYTES; `/` admits Ollama namespaces such as `model:library/qwen2.5:0.5b`); another shape → `invalid_argument` at `/body/record_id`.
- Kind: fixed at creation; a revise naming another `kind` → `invalid_argument` at `/body/definition/kind` (a kind change would drop a model from routing without a disable).
- Change: `change` is `"created"` (generation 1) or `"revised"` (generation + 1); each writes one `roster_revisions` row carrying the `operation_id`.
- Precondition: optional on revise; when given it is `{resource:"roster", id:<record_id>, generation}`; `id` ≠ `record_id` → `invalid_argument` at `/precondition/id`; a generation behind the record's → `stale_generation` at `/precondition/generation` with `current_generation`. The generation is compared inside the write transaction, so two serves on one ledger cannot both revise from one generation.
- Live socket: the drive procedure runs the create/revise paths only against a disposable serve; against the live unit's socket they print UNMEASURED and only write-nothing refusals run.
- Error: same key, other bytes → `conflict` at `/idempotency_key`; no key → `invalid_argument` at `/idempotency_key`.
- Empty: `definition: {}` (or missing `kind`/`caps`) → `invalid_argument` at `/body/definition`: a refusal, never a no-op revision.
- `forbidden` (operator capability): UNMEASURED, no grant exists in this release (owner: the grants slice; README.md:67 UNWRITTEN). It is not a drive path: no code emits it, so the procedure cannot drive it, and it stays UNMEASURED here until the grants slice adds the emitter.
- Retention of `roster_observations`: UNMEASURED, no writer in this release (owner: the retention slice).

- v4.0 path: the `unavailable` refusal.
- v4.1 success: `roster_revisions` gains one row; `operations` gains one row under `roster.update`; `roster.list` shows the record; `roster.inspect` by key returns it.
- Persistence: replay → `replayed=true`, no second revision; after `kill -KILL` post-ack the revision is present on restart (D7 shape, applied to this table).
- Error: `conflict`, `stale_generation` + `current_generation`, `forbidden` at `/action`.
- Empty: a refusal (see the marker block: `invalid_argument` at `/body/definition`).
- Side effects to read: `roster_records`, `roster_revisions`, `operations`.

## Gotchas

- The `roster.update` idempotency space must be **empty** after a fresh install (card roster §9 #4): the installer writes under `Owner::Deploy`. An operations row under this id that no operator sent is the v3 AR SYS HIGH defect reborn.
- "Operator only" is a grant capability, not a uid check (see task.resolve.md).
- Per-principal retention is a pure policy with a failing-first test (ATLAS §6); an update that grows `roster_observations` past the cap must prune the oldest, keeping the latest per record.
- No D-row; v4.1 slice evidence only. The owner trait is new in v4 (card actions §10).
