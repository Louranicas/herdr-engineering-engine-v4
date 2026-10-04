# task.cancel

A-08. A cancel intent against a task at a known generation: for a non-waiting task the state moves to `cancellation_requested`; for the two waiting variants (`blocked`, `effect_unknown`) the `cancel` field is set. One source of cancellation, no flag column (UM-P3, D-U5). Scope v4.0, phase P2, through the socket at P5 (F15), owner `Owner::Task`, effect `CancelIntent` (**mutating**). Trace E2E-09; D-row D8 (the F15 flow's L2 test).

## Sub-features

- cancel-intent: `transition(Cancel)` from `admitted`, `running`, `verifying`, `repair_pending` → `cancellation_requested`; from `cancellation_requested` → itself.
- cancel-field-on-waiting: from `blocked` → `blocked{cancel:true}`, from `effect_unknown` → `effect_unknown{cancel:true}` (I-07: never `cancellation_requested`).
- generation-precondition: `precondition{resource:"task", id, generation}` is **required**; a stale generation is `stale_generation` with `current_generation` in the reply.
- cancellation-obligation: the reply names `cancellation_obligation_id` and `worker_settlement` (the attempt's settle intent).
- driver-reads-cancel: the running attempt reads the cancel at its next check; the task then closes `cancellation_requested → cancelled` by `Stop` (E2E-09).
- idempotent-replay: op key `(principal, task.cancel, 1, key)`; readback `task.get {selector:{task_id}}`.
- later-accept-blocked: a committed cancel blocks later acceptance (I-03).

## How to get to it (user POV)

`hee4 task.cancel` (binary), `hee4-sh task.cancel reason=operator_request note:=null @idempotency_key=UUID @precondition:=JSON` (Command Map §2 spelling), Pi `hee4_task_cancel` (PROPOSAL). The caller first reads the task's generation with `task.get`.

## Driving it with hee4

Preconditions: README shared preconditions; a `CancelIntent` grant; a live task id and its current generation from `task.get`.

```bash
G=$(hee4-sh task.get 'selector:={"task_id":"<id>"}' evidence=none | … generation …)
K=$(uuidgen)
hee4 task.cancel
hee4-sh task.cancel reason=operator_request note:=null @idempotency_key=$K '@precondition:={"resource":"task","id":"<id>","generation":"'$G'"}'
hee4-sh task.cancel reason=operator_request note:=null @idempotency_key=$K '@precondition:={…same…}'     # replay
hee4-sh task.get 'selector:={"task_id":"<id>"}' evidence=none                                           # readback
```

Socket: request `body` `{reason, note}` (FACT required both) with `idempotency_key` and `precondition` set; result `body` `{task, cancellation_obligation_id, worker_settlement}` (API Map A-08). `UNWRITTEN: the reason value domain (the wrapper example uses operator_request) and the worker_settlement shape.`

- Success from `running`: readback shows `cancellation_requested`, then `cancelled` once the driver reaches its next check and `Stop` runs; `task_stops` has one row and the outbox has the stop row (UM C2).
- Success from `admitted`: `cancellation_requested` directly, then `cancelled` by `Stop` (no attempt ever began).
- Waiting variants: from `effect_unknown`, readback shows `effect_unknown` with `cancel=true`; the state name does not change (I-07).
- Cancel-of-cancel: a second cancel with a new key from `cancellation_requested` is legal and leaves the state as is; with the same key and bytes it is a replay (`replayed=true`).
- Error: stale generation → `stale_generation` + `current_generation`, retry `never`; unknown id → `not_found`; same key other bytes → `conflict`; missing `precondition` → `invalid_argument` at `/precondition`; a terminal task (`accepted`, `failed`, `cancelled`, `abandoned`) → `Illegal` mapped to `conflict`, retry `after_readback`, `readback` naming `task.get` (Error map PROPOSAL P-2).
- Empty: `note:=null` is the empty form and is legal.
- Persistence: the `operations` row and the state write commit in one transaction (State map §3 "Atomicity"); after `kill -KILL` between the ack and the driver's next check, the restarted engine still shows `cancellation_requested` or `cancelled`, never `running`.
- Side effects to read: `tasks.state` (or the `cancel` field), `operations`, `task_stops` and the outbox row after `Stop`, `events` (INTERP).

Concrete, deployed frame (rev 2026-10-05 drive) (run all of it with `tools/drive`):

```bash
hee4 task.cancel <task_id> --key $(uuidgen)       # raw body: {"task_id":"<id>"}
```

- The skeleton takes no `precondition`, `reason` or `note`. An admitted task: `phase=cancellation_requested` (or `cancelled` once the dispatcher stops it). A terminal task: `conflict` at `/body/task_id`, retry `after_readback`. Unknown id: `not_found`. No key: `invalid_argument` at `/idempotency_key`.

## Gotchas

- There is no `tasks.cancellation` column (card store §9 #2). A test reading one is reading a v3 ledger.
- Cancel is an intent, not a stop: the reply says `cancellation_requested`; `cancelled` arrives only after the attempt settles and `Stop` runs. Wait on the readback state, not on the reply.
- A `Settle{unsettled}` under cancel goes to `effect_unknown{cancel:true}` (State map §2c), so a cancelled task can end in `effect_unknown` and need `task.resolve`.
- A `Verdict` arriving in `cancellation_requested` records its `verifications` row (S4) but never moves the state and never accepts (I-03).
- `Illegal → conflict` is PROPOSAL P-2 of the Error map; until the P2 slice decides, pin the `readback` pointer, not the code.
- Through the socket this lands at P5 (F15), though the action exists from P2 (API Map A-08 "P2; through socket P5"). Card actions §3 lists F15 at both P2 and P5 rows; see the README contradiction note in the handback.
- Must not: `transition` is the only writer of `tasks.state` (PT-03); no SQL site writes `cancellation_requested`.
