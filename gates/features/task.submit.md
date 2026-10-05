# task.submit

A-05. Durable admission: the one way a task row is created. One frame in, one `TaskHeadV1` out, and a committed `admitted` row with its `operations` row and `serve_cgroup` before the ack. Scope v4.0, phase P2 (host at P4 as F06, P5 as F07), owner `Owner::Task`, effect `DurableAdmission` (**mutating**). Traces E2E-01, E2E-02, E2E-08; D-rows D6, D9 (through the unit's socket), D7 (acked rows survive a kill).

## Sub-features

- durable-admission: `transition(∅, Admit)` → `admitted`, fsynced before the reply ("recorded before the ack", State map §3).
- idempotent-replay: op key `(principal, task.submit, 1, idempotency_key)`; same bytes again → `replayed=true` and the stored result; same key with other bytes → `conflict`.
- readback-after-lost-reply: `task.get {selector:{source_action:"task.submit", idempotency_key}}` finds the row from values the caller had before sending.
- engine-cursor: the reply carries `engine_cursor` for a later `events.subscribe` (INTERP, API Map A-05).
- serve-cgroup-stamp: `tasks.serve_cgroup` written from `/proc/self/cgroup` of the admitting `serve` (DEC V4-15); D9 counts only rows ending in `/hee4.service`.
- dispatcher-wake: the commit wakes RL-2; dispatch is not part of this action's reply.
- admission-budget: the 128 GiB backup budget is enforced fail-closed at admission (card dispatcher §9 #2); the bound comes from K0, not a literal (IN-16).
- rc01-profile: as `task.preview`: `remote_allowed` or non-zero currency is refused.

## How to get to it (user POV)

`hee4 task.submit` (binary), `hee4-sh task.submit spec:=JSON @idempotency_key=UUID`, Pi `hee4_task_submit` (PROPOSAL). ATLAS D6 requires the real task to be submitted **through the unit's socket**, not a transient unit in a disposable HOME; F06/F07 in a disposable HOME are P4/P5 exit evidence only.

## Driving it with hee4

Preconditions: README shared preconditions; a `DurableAdmission` grant; a fresh UUID per intent; a spec that `task.preview` found eligible.

```bash
K=$(uuidgen)
hee4 task.submit
hee4-sh task.submit 'spec:={…TaskSpecV1…}' @idempotency_key=$K
hee4-sh task.submit 'spec:={…same bytes…}' @idempotency_key=$K        # replay
hee4-sh task.submit 'spec:={…other bytes…}' @idempotency_key=$K       # conflict
hee4-sh task.get 'selector:={"source_action":"task.submit","idempotency_key":"'$K'"}' evidence=none
```

Socket: request `body` `{spec: TaskSpecV1}` with `idempotency_key` set and `precondition` `null`; result `body` `{task: TaskHeadV1, engine_cursor}` (API Map A-05). `UNWRITTEN: the TaskHeadV1 field list and the engine_cursor type on the v4 wire.`

- Success: `task.state=admitted`; host `mode=ro` query shows the `tasks` row, one `operations` row for the key, and `serve_cgroup` ending in `/hee4.service` when driven through the unit (D9). `effect` in the envelope is `committed`.
- Persistence: replay returns `replayed=true` with the same `task.id`; `operations` count unchanged. Then `crash-restart.md`: N acked submits survive `kill -KILL` (`acked_present=N/N`, D7).
- Error: `conflict` (same key, other bytes; retry `never`); `unavailable` when the ledger is not writable; `resource_exhausted` with both numbers when the admission headroom or 128 GiB budget is exhausted (Error map PROPOSAL P-3); RC01 refusals as preview.
- Cancel: not applicable to submit itself; the admitted task is cancelled through `task.cancel` (task.cancel.md).
- Empty: `spec` missing → `invalid_argument` at `/body/spec`; a missing `idempotency_key` on a mutating action → `invalid_argument` at `/idempotency_key` (`UNWRITTEN: the exact pointer and constraint text`).
- Side effects to read: `tasks`, `operations`, `events` (INTERP) rows; the dispatcher's wake (the task leaves `admitted` within the gate's budget when a model is present).

Concrete, deployed frame (rev 2026-10-05 drive) (run all of it with `tools/drive`):

```bash
hee4 task.submit --brief-file brief.txt --key $(uuidgen)       # prints the reply frame
# raw: {"request_id":"r","action":"task.submit","action_version":1,"idempotency_key":"<K>","body":{"brief":"GOAL: …\n…RESTATEMENT: …\n"}}
```

- Success: `{task_id:"t-<24 hex>", phase:"admitted"}`, `replayed=false`. Replay (same key, same bytes): `replayed=true`, same `task_id`. Same key, other bytes: `conflict` at `/idempotency_key`.
- Empty brief `""`: `invalid_argument` at `/body/brief`, message names `GOAL` (the first absent field). Missing RESTATEMENT line: `invalid_argument` at `/body/brief` naming `RESTATEMENT`. No key: `invalid_argument` at `/idempotency_key`.
- VERIFY lines: an absolute path or `sh: <line>` runs in the sandbox; `model: <prompt>` and any other line are admitted and recorded as a named skip (`driver has no handler for step kind model; use a `sh:` step ...`), never run and never an observation.

## Gotchas

- `Admit` on an existing task is `Illegal` by construction (I-08): a repeated submit is a replay through the primitive, never a transition. Seeing a second `tasks` row for one key is a store defect.
- Expired but recorded → the stored disposition (readback, not new execution); expired and unrecorded → `deadline_exceeded`. A caller that re-sends after a timeout must reuse the key or it creates a second task.
- A task admitted in a disposable HOME (F06/F07) is correctly **not** counted by D9: `serve_cgroup` does not end in `/hee4.service` (E2E Conflicts note). That is the filter working, not a bug.
- The 60 s deadline applies to admission only; the task's own budget is `TASK_LIMIT` 20 min across repairs, in the task, not the envelope.
- `effect` in the result envelope is `committed` for a fresh admit; a `pending` here would mean the primitive acked before the fsync, which D7's real-kill step exists to catch.
- Backup cadence (RL-6) may run before the dispatch that follows this submit when the last backup is older than 15 min; a slow first dispatch after a quiet period is the backup, not a stuck task (Loop map C2 records the cadence as undecided between two readings).
- Must not: the submit bound is read from K0 (IN-16), the `Dispatcher` is constructed once (card main AP-01), and no code path writes `tasks.state` except through `transition` (card store §9 #1).
