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
hee4-sh roster.disable record_id=<id> active_attempt_policy=<policy> audit_reason=<text> @idempotency_key=$(uuidgen) '@precondition:={"resource":"roster","id":"<id>","generation":"<g>"}'   # v4.1; UNMEASURED: no hee4-sh exists in the six crates
hee4-sh roster.inspect 'selector:={"record_id":"<id>"}'                                                               # readback; UNMEASURED: no hee4-sh
hee4-sh task.get 'selector:={"task_id":"<running on it>"}' evidence=none                                              # UNMEASURED: no hee4-sh
```

Socket: request `body` `{record_id, active_attempt_policy, audit_reason}` with `precondition{resource:"roster", …}`; result `body` `{record, operation_id, active_attempts[], cancellation_obligations[]}` (API Map A-14).

(rev 2026-10-05 drive) Served from v4.1 by `actions/roster.rs` (`tools/drive.d/roster.py` `d_disable`, run last: it disables the serve's only model record):

```bash
hee4 roster.disable --key "$(uuidgen)" --precondition '{"resource":"roster","id":"model:qwen2.5-coder:7b","generation":1}' --body '{"record_id":"model:qwen2.5-coder:7b","active_attempt_policy":"let_finish","audit_reason":"retire"}'
hee4 roster.list --body '{"kinds":[],"capability":null,"locality":null,"include_disabled":true,"page":{"limit":100,"cursor":null}}'
hee4 task.preview --body '{"brief":"<eleven fields>"}'   # eligible:false, exclusions names the record
```

- Policy domain: `active_attempt_policy` is `let_finish` or `request_cancel`; anything else → `invalid_argument` at `/body/active_attempt_policy`.
- Precondition: required (`PreconditionRule::Required("roster")`); absent → `invalid_argument` at `/precondition`, refused by dispatch before the handler. `generation` is an unsigned integer, not a string.
- Success: the record reads `disabled:true`, generation + 1; `roster.list` hides it, `include_disabled:true` shows it; both arrays present (empty with no active attempts); a later `task.preview` answers `eligible:false` with `exclusions` holding the record id, and a submitted task ends `abandoned` (`RouteRefused`): no fallback to `HEE4_MODEL` while records exist.
- `request_cancel`: each listed task gets `Store::apply(Cancel)` after the disable commits; the obligation is `{task_id, phase_after}` or `{task_id, refusal}` (a refused edge is text in the obligation, never an error frame).
- Error: stale generation → `stale_generation` at `/precondition/generation` with `current_generation`; unknown record → `not_found` at `/body/record_id`; over `MAX_VIEW_ITEMS` active attempts → `resource_exhausted` naming both numbers (the earlier "arrays over 100 → invalid_argument" line is superseded).
- `forbidden` (operator capability): UNMEASURED, no grant exists in this release (owner: the grants slice; README.md:67 UNWRITTEN). It is not a drive path: no code emits it, so the procedure cannot drive it, and it stays UNMEASURED here until the grants slice adds the emitter.
- Live socket: the disable, the preview-after-disable and the task.submit paths run only against a disposable serve (re-enable is not an action); against the live unit's socket they print UNMEASURED and only write-nothing refusals run. The deploy record's id is read from the serve, not from the drive's `HEE4_MODEL`.

- v4.0 path: the `unavailable` refusal.
- v4.1 success: the record reads disabled; with `request_cancel`, each listed task shows `cancellation_requested` (or the `cancel` field on a waiting variant) and `cancellation_obligations` has one entry per task.
- Error: `stale_generation`, `not_found`, `forbidden`; arrays over 100 → `invalid_argument`.
- Empty: no active attempts → both arrays empty, still a `result`.
- Persistence: replay; the disabled mark survives restart; the cancel intents are ordinary `task.cancel`-shaped transitions and follow task.cancel.md's persistence.
- Side effects to read: `roster_records`, `operations`, `tasks.state` of the affected tasks.

## Gotchas

- The cancel on affected tasks is an **intent** (task.cancel.md): they close only when their attempts settle and `Stop` runs. The reply lists obligations, not closed tasks.
- A dispatcher permit (RL-2 "roster permit") must refuse the disabled record for new dispatches; read a subsequent `task.preview` to see the record in `exclusions` (every disabled model record, and every `model:` id whose kind is not `model`: since 2026-10-06 `roster.update` refuses creating one, `invalid_argument` at `/body/definition/kind`, so only a row written before that refusal can be one).
- The `request_cancel` → `transition(Cancel)` coupling is INTERP from the reply shape; no authority states it in prose. Pin it when the v4.1 slice names it.
- Active attempts (rev 2026-10-06, U-harden-05 h5-roster-attempts) are read from K1's attempts ledger, not guessed from routing: `Store::open_attempts()` (every `attempts` row in state `running`), keeping the rows whose acknowledged model (`started.model`, written by the dispatcher's `attempt_started`) equals the record's model name; one task id per row, task id order. A record whose id is not `model:` lists none. Routing (`dispatcher::route_as_dispatched`, now deleted) is not consulted, so with two eligible models a disable of one lists and cancels only that model's tasks (`actions::roster::tests::disable_lists_only_attempts_on_that_model`), and a disable of the routed model lists none when no open row names it (`disable_of_the_routed_model_with_no_open_rows_lists_none`).
- An unacknowledged row (`started` is `None`: dispatched, `attempt_started` not yet written) matches a record only when its dispatch record names the model; the `Dispatch` event carries no model, so today it never matches and the task is not listed. Limit: a task dispatched onto the disabled model in that window is neither listed nor cancelled by `request_cancel`; it runs its attempt. A task in `verifying` has no open row (its `Settle` closed it) and is not listed either.
- No D-row; v4.1 slice evidence only.
