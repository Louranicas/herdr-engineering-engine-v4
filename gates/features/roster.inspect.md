# roster.inspect

A-12. One roster record with its last operation, selected by id or by the submit key of a `roster.update` — the mandatory readback for a lost create. Scope **v4.1**, phase P9, owner `Owner::Roster`, effect `Read`.

## Sub-features

- select-by-id: `selector` by record id.
- select-by-operation: `selector{source_action, idempotency_key}`, the readback after a lost `roster.update` or `roster.disable` reply.
- record-and-last-operation: `{record, last_operation}` from `roster_records`, `roster_revisions`, `operations`.
- v4.0-refusal: `unavailable` by name at the v4.0 tag.

## How to get to it (user POV)

`hee4 roster.inspect` (binary); wrapper "generated"; Pi `hee4_roster_inspect` (PROPOSAL). At v4.0, `unavailable` with `because` = v4.1. From v4.1, a caller reaches it after `roster.list`, or directly with the key of an update whose reply was lost.

## Driving it with hee4

Preconditions (v4.1): a `Read` grant; a record id from `roster.list` or a `roster.update` key.

```bash
hee4 roster.inspect                                                      # v4.0: unavailable by name
hee4-sh roster.inspect 'selector:={"record_id":"<id>"}'                 # v4.1 (selector shape UNWRITTEN below)
hee4-sh roster.inspect 'selector:={"source_action":"roster.update","idempotency_key":"<uuid>"}'
```

Socket: request `body` `{selector}` (FACT required); result `body` `{record, last_operation}` (API Map A-12). `UNWRITTEN: the roster selector field names (by id and by operation) and the record/last_operation shapes.`

- v4.0 path: the `unavailable` refusal with its `because`.
- v4.1 success: the record after a `roster.update` carries the new revision and `last_operation.operation_id` equals the update reply's.
- Readback: with the update's key, the same record is returned; this must work from values the caller held before sending (CD RC03 §6).
- Error: unknown record → `not_found`.
- Empty: none; `selector` is required.
- Side effect: none.

## Gotchas

- A record installed by deploy has a `last_operation` under `Owner::Deploy`, never a wire action id (NF-ROSTER-INSTALL). A `last_operation.action = roster.update` on a freshly installed record means the installer used the wire id — the v3 defect.
- The by-operation selector reads the K1 primitive's binding `(principal, action, version, key)`; a different principal cannot read another's key (the socket is 0600 so this is moot in v4.0, and the trigger for a second principal is `admit_peer` admitting one).
- No D-row; v4.1 slice evidence only.
