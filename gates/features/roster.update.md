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
hee4-sh roster.update record_id=<id> 'definition:={…}' audit_reason=<text> @idempotency_key=$K      # v4.1
hee4-sh roster.update record_id=<id> 'definition:={…same…}' audit_reason=<text> @idempotency_key=$K # replay
hee4-sh roster.inspect 'selector:={"source_action":"roster.update","idempotency_key":"'$K'"}'      # readback
```

Socket: request `body` `{record_id, definition, audit_reason}`; result `body` `{record, operation_id, change}` (API Map A-13). `UNWRITTEN: the definition schema, the change shape, whether a precondition (record generation) is required on revise, and the generated wrapper spelling.`

- v4.0 path: the `unavailable` refusal.
- v4.1 success: `roster_revisions` gains one row; `operations` gains one row under `roster.update`; `roster.list` shows the record; `roster.inspect` by key returns it.
- Persistence: replay → `replayed=true`, no second revision; after `kill -KILL` post-ack the revision is present on restart (D7 shape, applied to this table).
- Error: `conflict`, `stale_generation` + `current_generation`, `forbidden` at `/action`.
- Empty: `UNWRITTEN: whether an empty definition is a refusal or a no-op revision.`
- Side effects to read: `roster_records`, `roster_revisions`, `operations`.

## Gotchas

- The `roster.update` idempotency space must be **empty** after a fresh install (card roster §9 #4): the installer writes under `Owner::Deploy`. An operations row under this id that no operator sent is the v3 AR SYS HIGH defect reborn.
- "Operator only" is a grant capability, not a uid check (see task.resolve.md).
- Per-principal retention is a pure policy with a failing-first test (ATLAS §6); an update that grows `roster_observations` past the cap must prune the oldest, keeping the latest per record.
- No D-row; v4.1 slice evidence only. The owner trait is new in v4 (card actions §10).
