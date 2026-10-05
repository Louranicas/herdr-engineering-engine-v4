# task.resolve

A-09. An operator records a disposition against a task obligation: `quarantine` moves any non-terminal task to `blocked`; `abandon` closes it through the stop door as `cancelled` (if a cancel was requested) or `abandoned`. "Operator only": the capability is on the grant. Scope v4.0, phase P2, through the socket at P5 (F15), owner `Owner::Task`, effect `RecordDisposition` (**mutating**). Trace E2E-09; D-row D8.

## Sub-features

- quarantine: `Resolve{quarantine}` from every non-terminal state → `blocked` (cancel field carried); closes nothing.
- abandon: `Resolve{abandon}` from every non-terminal state → `cancelled` when cancellation was requested (state or field), else `abandoned`; through `terminal::close`, which inserts the `task_stops` row and the outbox row (UM C2).
- disposition-row: a `task_dispositions` row per resolution, with `one_resolution` (S8).
- generation-precondition: required, as `task.cancel`.
- evidence-refs: `evidence[]` ≤ 64 `EvidenceRefV1{artifact_id, sha256, byte_length, media_type, schema_id}`.
- operator-capability: `forbidden` when the grant lacks the operator capability.
- unmapped-dispositions: `retry` and `acknowledge_external_effect` exist in the `Disposition` enum (FACT) but have no task-side target; they are refused by name until State map P-2 is settled.

## How to get to it (user POV)

`hee4 task.resolve` (binary), `hee4-sh task.resolve obligation_id=… disposition=… reason=… evidence:=JSON @precondition:=JSON` (Command Map §2). No Pi tool (v3 `tool: None`, "operator only"). The operator reaches it after `task.get` shows a waiting state (`blocked`, `effect_unknown`) or after a cancel that cannot settle. (UNMEASURED: hee4-sh exists in no crate)

## Driving it with hee4

Concrete, deployed frame (rev 2026-10-05 drive) (run all of it with `tools/drive`):

```bash
hee4 task.resolve --key $(uuidgen) --body '{"task_id":"t-<24 hex>","resolution":"abandon","reason":"attempt_failed"}'
# raw: {"request_id":"r","action":"task.resolve","action_version":1,"idempotency_key":"<K>","body":{"task_id":"t-…","resolution":"quarantine"}}
```

The deployed body is `{task_id, resolution: "quarantine"|"abandon", reason?}` + `idempotency_key` (crates/hee4-app/FLOW.md Families); there is no `obligation_id`, `disposition` or `evidence` (the UNWRITTEN obligation_id below is resolved by its absence). The catalogue gives task.resolve `PreconditionRule::None` (crates/hee4-contracts/src/catalogue.rs:351), so no `precondition` is sent and `stale_generation` is unreachable: the drive prints `note=stale_generation unreachable reason=task.resolve PreconditionRule::None (catalogue.rs:351)`. Paths (`d_resolve`, E2E-09; the briefs are `drive_d.BRIEF` and `SLEEP_BRIEF`, VERIFY `/usr/bin/sleep 5`):

- `cancel_then_abandon`: submit SLEEP_BRIEF → poll `task.get` until `running` (bound `RUNNING_WAIT_S` = 10 s) → `task.cancel` → `cancellation_requested` → resolve abandon `attempt_failed` → `cancelled`; `task.get` → `cancelled`. The wait is the procedure, not the engine: since wave 5 (c5-cancel-before-dispatch) a cancel that lands before dispatch is Stopped to `cancelled` by the dispatcher, so the abandon met `conflict` at `/body/task_id` (live unit at 9ba0429). A task not `running` within the bound is UNMEASURED naming the bound and the last phase, and no cancel is sent.
- `quarantine`: a second SLEEP_BRIEF task, resolve quarantine, reason omitted → `blocked`; `task.get` → `blocked`.
- `abandon_blocked`: resolve abandon on it → `abandoned`.
- `resolve_terminal`: abandon again → `conflict` at `/body/task_id`.
- `not_found`: `t-` + 24 zeros → `not_found` at `/body/task_id`.
- `bad_resolution`: `"retry"` → `invalid_argument` at `/body/resolution`.
- `bad_reason`: quarantine with reason `attempt_failed` → `invalid_argument` at `/body/reason`.
- `missing_key`: no key → `invalid_argument` at `/idempotency_key`.
- A lifecycle path whose task the dispatcher abandoned before the drive's frame (no eligible model, e.g. roster.disable earlier in the run), or stopped to `cancelled` because the cancel landed before dispatch (the cancel-before-dispatch race), is UNMEASURED naming that, never PASS (`raced()`).

Preconditions: README shared preconditions; a `RecordDisposition` grant with the operator capability; a task id, generation and `obligation_id` (`RESOLVED (rev 2026-10-05 drive: the deployed body has no obligation_id), was UNWRITTEN: where a caller obtains obligation_id for quarantine/abandon; task.cancel returns cancellation_obligation_id, but no v4.0 read returns the obligations of an effect_unknown or blocked task`).

```bash
hee4 task.resolve
hee4-sh task.resolve obligation_id=<ob> disposition=quarantine reason=<text> 'evidence:=[]' @idempotency_key=$(uuidgen) '@precondition:={"resource":"task","id":"<id>","generation":"<g>"}'  # UNMEASURED: hee4-sh exists in no crate
hee4-sh task.resolve obligation_id=<ob> disposition=abandon   reason=<text> 'evidence:=[]' @idempotency_key=$(uuidgen) '@precondition:={…}'  # UNMEASURED: hee4-sh exists in no crate
hee4-sh task.get 'selector:={"task_id":"<id>"}' evidence=none     # readback  # UNMEASURED: hee4-sh exists in no crate
```

Socket: request `body` `{obligation_id, disposition, reason, evidence[]}` (FACT required all four) with `precondition` set; result `body` `{task, disposition_id, obligation_state}` (API Map A-09). `disposition ∈ {retry, abandon, acknowledge_external_effect, quarantine}` (FACT v3 `enum Disposition`).

- Quarantine from `running`: readback shows `blocked`; no `task_stops` row; `task_dispositions` has one row.
- Abandon from `effect_unknown{cancel:false}`: readback shows `abandoned`; `task_stops` has one row with `state=abandoned`; the outbox has the stop row; a subscriber on `events.subscribe` receives it.
- Abandon after a cancel: from `cancellation_requested` or `blocked{cancel:true}` the terminal is `cancelled`, not `abandoned` (INTERP split, State map §2a).
- Error: `forbidden` at `/action` without the operator capability; `stale_generation`; `not_found`; terminal source → `Illegal` → `conflict` + `after_readback` (PROPOSAL P-2); `disposition=retry` or `acknowledge_external_effect` → refused by name (`UNWRITTEN: the code and field for an unmapped disposition`); `evidence` over 64 → `invalid_argument` naming the bound.
- Empty: `evidence:=[]` is legal.
- Persistence: dispositions row, state write, `task_stops` and outbox row commit atomically; a second resolve on the same task is refused by `one_resolution` (S8) — `UNWRITTEN: as which code`.
- Side effects to read: `task_dispositions`, `tasks.state`, `task_stops`, outbox, `operations`.

## Gotchas

- `blocked` is sticky: `Settle` of any kind keeps it `blocked` (UM §5c "blocked stays blocked"); only `abandon` leaves it. A test expecting a quarantined task to finish on its own is wrong.
- Two of four dispositions have no target state in any authority (State map P-2). Driving them before the P2 resolve slice measures a refusal, and the file must say so.
- `failed + Stop` is `Illegal` (DC-06): a task that already closed cannot be resolved; expect `conflict` with a readback, not a second stop row.
- Operator-only means the grant record, not the uid: the socket is 0600, so every caller is the same uid; the capability check is on `grant_id`'s scope (Error map class 2 second field).
- Must not: `task_stops` has one insert site (the stop door) and the outbox one insert fn; a resolve path that writes either directly fails the one-door census (`duplicate_sites=0`, DEC V4-17).
