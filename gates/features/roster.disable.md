# roster.disable

A-14. Disable a roster record with a policy for its active attempts: either let them finish or request their cancellation, recorded as an operation. Operator only; precondition required. Scope **v4.1**, phase P9, owner `Owner::Roster`, effect `ConfigurationMutation` (**mutating**).

## Sub-features

- disable-record: `roster_records` marked disabled; `roster.list include_disabled=false` hides it.
- active-attempt-policy: `active_attempt_policy`; with `request_cancel` the owning tasks get `transition(Cancel)` (INTERP from the reply shape, API Map A-14).
- reply-obligations: `{record, operation_id, active_attempts[], cancellation_obligations[]}`, arrays ≤ 100.
- generation-precondition: required on the roster record.
- idempotent-replay: readback `roster.inspect`.
- v4.0-refusal: `unavailable` by name.

## How to get to it (user POV)

`hee4 roster.disable` (binary); wrapper "generated"; no Pi tool ("operator only"). At v4.0, `unavailable`. From v4.1 an operator reaches it to retire a record while tasks may still be running on it.

## Driving it with hee4

Preconditions (v4.1): a `ConfigurationMutation` grant with the operator capability; the record's current generation from `roster.inspect`; optionally a running task on that record to exercise `request_cancel`.

```bash
hee4 roster.disable                                                                                                   # v4.0: unavailable by name
hee4-sh roster.disable record_id=<id> active_attempt_policy=<policy> audit_reason=<text> @idempotency_key=$(uuidgen) '@precondition:={"resource":"roster","id":"<id>","generation":"<g>"}'   # v4.1
hee4-sh roster.inspect 'selector:={"record_id":"<id>"}'                                                               # readback
hee4-sh task.get 'selector:={"task_id":"<running on it>"}' evidence=none                                              # cancellation_requested if request_cancel
```

Socket: request `body` `{record_id, active_attempt_policy, audit_reason}` with `precondition{resource:"roster", …}`; result `body` `{record, operation_id, active_attempts[], cancellation_obligations[]}` (API Map A-14). `UNWRITTEN: the active_attempt_policy value domain (request_cancel is the one named; the others are not) and the generated wrapper spelling.`

- v4.0 path: the `unavailable` refusal.
- v4.1 success: the record reads disabled; with `request_cancel`, each listed task shows `cancellation_requested` (or the `cancel` field on a waiting variant) and `cancellation_obligations` has one entry per task.
- Error: `stale_generation`, `not_found`, `forbidden`; arrays over 100 → `invalid_argument`.
- Empty: no active attempts → both arrays empty, still a `result`.
- Persistence: replay; the disabled mark survives restart; the cancel intents are ordinary `task.cancel`-shaped transitions and follow task.cancel.md's persistence.
- Side effects to read: `roster_records`, `operations`, `tasks.state` of the affected tasks.

## Gotchas

- The cancel on affected tasks is an **intent** (task.cancel.md): they close only when their attempts settle and `Stop` runs. The reply lists obligations, not closed tasks.
- A dispatcher permit (RL-2 "roster permit") must refuse the disabled record for new dispatches; read a subsequent `task.preview` to see the record in `exclusions`.
- The `request_cancel` → `transition(Cancel)` coupling is INTERP from the reply shape; no authority states it in prose. Pin it when the v4.1 slice names it.
- No D-row; v4.1 slice evidence only.
