# task.get

A-06. One task in full: its head, criteria digest, attempts, cleanup, delivery count, and (on request) evidence, selected by id or by the submit key. Scope v4.0, phase P2, owner `Owner::Task`, effect `Read`. Trace E2E-03; D-row D6 (`hee4 task.get <id>` → `state=accepted`). It is also the readback action for `task.submit`, `task.cancel` and `task.resolve`.

## Sub-features

- select-by-id: `selector:{task_id}`.
- select-by-submit-key: `selector:{source_action:"task.submit", idempotency_key}`, the lost-reply readback.
- head-and-state: `task` with its typed `TaskStateV1` (11 values; no `queued`).
- attempts-view: `attempts[]` with `state ∈ {running, settled, unknown}`, `effect`, `cleanup`; ≤ 100 (CD RC03 §4).
- evidence-view: `evidence=none` or a bounded list (≤ 64) of evidence refs from `artifacts`, `acceptances`, `verifications`.
- delivery-count: `delivery` is the real `outbox.delivered` count (`delivery_of`, UM §3b OUT-5), not a placeholder.
- cleanup-state: `cleanup` from the attempt's cleanup field.
- cursor: `cursor` for continuing a bounded view.

## How to get to it (user POV)

`hee4 task.get <id>` (binary, ATLAS D6 positional form), `hee4-sh task.get selector:=JSON evidence=none`, Pi `hee4_task_get` (PROPOSAL). An in-task refusal (`failed`, `cancelled`, `repair_pending`, `effect_unknown`) is not an error frame; it is read here (Error map §0 "In-task refusal").

## Driving it with hee4

Preconditions: README shared preconditions; a `Read` grant; a task id from `task.submit` or the submit key.

```bash
hee4 task.get <task_id>
hee4-sh task.get 'selector:={"task_id":"<id>"}' evidence=none
hee4-sh task.get 'selector:={"source_action":"task.submit","idempotency_key":"<uuid>"}' evidence=none
hee4-sh task.get 'selector:={"task_id":"<id>"}' 'evidence:={…}'
```

Socket: request `body` `{selector: TaskSelectorV1, evidence}` (FACT required both); result `body` `{task, criteria_sha256, attempts[], cleanup, delivery, evidence[], cursor}` (API Map A-06). `UNWRITTEN: the v4 evidence request value domain (v3 wrapper used evidence=none; the non-none form is not in the maps).`

- Success (D6): after E2E-01, `task.state=accepted`; `attempts[0].state=settled`, and `verifications` holds `verdict_of(decide)` for that attempt (host `mode=ro` row read). `delivery` ≥ 1 after an `events.subscribe` consumer acknowledged the accept row.
- Readback: the submit-key selector returns the same `task.id` as the submit reply; it must work from values the caller held before sending (CD RC03 §6).
- Error: unknown id → `not_found`; a view past its bound → `resource_exhausted`; a missing or corrupt object behind an evidence ref → `unavailable` (FACT `trait Tasks` fn `get` doc).
- Empty: a task with no attempts yet (`admitted`) returns `attempts=[]`, `delivery=0`; still a `result`.
- Persistence: none written. Repeat yields the same bytes for a terminal task; for a live task the `task.generation` moves and later reads differ (not a flake).
- Side effect: none; assert `operations` count unchanged.

Concrete, deployed frame (rev 2026-10-05 drive) (run all of it with `tools/drive`):

```bash
hee4 task.get <task_id>        # raw body: {"task_id":"<id>"}
```

- Found: body `{task_id, phase, events, last_receipt_hash}`. Unknown valid id (`t-` + 24 zeros): `not_found` at `/body/task_id`. A malformed id (not a token, e.g. `bad id/..`): `invalid_argument` at `/body/task_id`.

## Gotchas

- `queued` is gone from both `TaskStateV1` and `attempts[].state` in v4 (API Map §3 note; UM §5b). A reply carrying `queued` is a v3 binary.
- `delivery` reads the real delivered count; v3's "pending forever" is the failure it exists to expose (RL-5 failure modes). A delivery count that never moves while a subscriber runs is a defect in `acknowledge_delivery`, not in this action.
- A task in `effect_unknown` after a kill stays readable here with `cancel` carried as a field of the state, not a second flag (UM-P3, D-U5). There is no `cancellation` column to read (card store §9 #2).
- Evidence refs are content-addressed objects (`objects/sha256/`); `unavailable` for a missing object is an IO/host refusal and says the object store, not the ledger, is damaged.
- Must not: never read the ledger directly as the user path; the host `mode=ro` query is the side-effect check, `task.get` is the behaviour.
