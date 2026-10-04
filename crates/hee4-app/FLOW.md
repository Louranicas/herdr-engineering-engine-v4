# hee4-app flow

K6. The `hee4` binary: startup order, the control socket, five actions, the synchronous
dispatcher, `doctor`. Defines no contract type; every state change is `Store::admit` or
`Store::apply`, and the only verdict is `hee4_evidence::decide_and_seal`'s.

```
hee4 serve --socket S --ledger L --work W
  Store::open(L)                      (resets recovery_complete=0)
  probe_workers(store, W)             /proc argv scan: <W>/<task> in a live argv → PidReused (R07), else Absent (R08)
  recovery::reconcile(store, probe)   findings → exit "recovery incomplete", never listens
  dispatcher thread                   loop { step(); sleep 100 ms when idle }
  socket::bind(S)                     dir 0700 (owner = our uid), stale socket removed only if nothing answers, socket 0600
  socket::serve                       one thread per connection, one JSON frame per LF line each way
```

## Frame

Request (one line): `{"request_id": str, "action": str, "action_version": 1, "idempotency_key": str|null, "body": {}}`.
Other API Map envelope members are ignored by the skeleton. Reply (one line):
`{"kind":"result","request_id","replayed","body"}` or
`{"kind":"error","request_id","code","retry","field","message"}`. A line over 1,048,576 bytes or
EOF inside a line closes the connection with no reply (Socket and IPC Map).

| Action | Body | Result body | Door |
|---|---|---|---|
| `health` | `{}` | `{ok, head_sha, recovery_complete, uptime_s}` | `Store::recovery_complete` |
| `task.submit` | `{brief: "<eleven fields>"}` + `idempotency_key` | `{task_id, phase}`; `replayed` | `Brief::parse` + `check_restatement`, then `Store::admit` |
| `task.get` | `{task_id}` | `{task_id, phase, events, last_receipt_hash}` | `Store::phase`, `history`, `chain_head` |
| `task.list` | `{}` | `{tasks: [{task_id, phase}]}` | `Store::task_ids`, `phase` |
| `task.cancel` | `{task_id}` + `idempotency_key` | `{task_id, phase}` | `Store::apply(Event::Cancel)` |

`task_id` = `t-` + 24 hex of SHA-256(`principal \n idempotency_key`), so a key names one task.
The brief text is written to `<W>/briefs/<task>.brief` under the same ledger lock as `admit`.

## Refusal names (one per refusal)

| Name | Retry | When | Field |
|---|---|---|---|
| `invalid_argument` | never | line not a JSON object; `request_id`/`action`/`action_version`/`body` missing or mistyped; mutating action without `idempotency_key`; brief missing a field, duplicate field, empty RESTATEMENT; bad `task_id` | the member's pointer (`/`, `/body/brief`, …) |
| `unknown_action` | never | action not one of the five | `/action` |
| `unsupported_action_version` | never | `action_version` ≠ 1 | `/action_version` |
| `not_ready` | after_condition | mutating action while the ledger's `recovery_complete` is false | `/action` |
| `conflict` | after_readback | same idempotency key, other body bytes; `transition` refused the cancel | `/idempotency_key`, `/body/task_id` |
| `not_found` | never | `task.get`/`task.cancel` of a task never admitted | `/body/task_id` |
| `forbidden` | never | reserved for a peer uid ≠ ours; not raised (peer check UNMEASURED, below) | — |
| `internal` | same_exact_request | the ledger failed under the request | `/` |

`not_ready` is not in the Error map's 18 codes; the map's nearest is `unavailable` (after_condition).
DC proposal below.

## Dispatcher (one task at a time)

| Step | Door | On refusal |
|---|---|---|
| first `admitted` task | `Store::task_ids`, `phase` | — |
| brief | `<W>/briefs/<task>.brief`, `Brief::parse` | `Resolve(Abandon)`, reason `brief_unreadable` journalled |
| playbook | VERIFY lines: absolute path → `Run`; `model: <prompt>` → `Generate`; else `Unsupported` (named skip) | — |
| route | `route::select` over a one-row roster (`HEE4_MODEL`, default `qwen2.5-coder:7b`), floor local-only, baseline = that model; availability probed (`tags`) only when a model step will run | `Resolve(Abandon)`, reason journalled |
| namespace | `NamespaceTask::new(task, W, needs_model, TIMEBOX)` → `plan_for`; `needs_model` = a `Generate` step and `HEE4_LIVE_MODEL=1`, so no `--share-net` otherwise | `Resolve(Abandon)` |
| permit | `Permit::mint(ReceiptId "r-<task>-<ns>", scope = the Run programs)` | — |
| dispatch | `Store::apply(Dispatch)` → running (refused before reconcile by K1) | — |
| attempt | `Attempt::run` (bwrap for Run steps; Generate steps skip with no loopback when not live, `UNMEASURED` printed) | error or a `Failed` step: `Settle(NotReady)` → `Stop` → failed, no receipt |
| settle | `Settle(Ready)` → verifying | — |
| observe | per observation: `observation_id`, `Store::record_observation`, `apply(Observe)` | — |
| decide + seal | `decide_and_seal(chain_head, receipt_id, ids, obs, subject)`; ids: collector = digest(ledger epoch), locks = digest(permit), standards = digest(`gate.toml` baked at build); subject input = first model prompt, else VERIFY text | — |
| receipt | `Store::append_receipt` (K1 re-runs `verify_chain`) | — |
| verdict | `apply(Decide(verdict))`; `Pass` → `apply(Accept)` | — |

With `HEE4_LIVE_MODEL` unset no model step runs, so no tier-0 observation exists and `decide`
returns `Refused(invalid)`: the task ends `failed` with a sealed receipt, never `accepted`.
I3 has no "unmeasured" outcome, so none is fabricated.

## Peer credentials

`SO_PEERCRED` needs `getsockopt` (unsafe; `unsafe_code = "forbid"`) or the unstable
`UnixStream::peer_cred` (E0658 on rustc 1.99). `socket::PEER_CHECK = PeerCheck::Unmeasured`;
the 0700 directory owned by our uid is the only gate. The principal is the process uid from
`/proc/self`.

## Gaps

- The brief lives in a file, not the ledger (no `Store` brief column or reader). A crash between
  `admit` and the write leaves an admitted task the dispatcher abandons by name.
- `Event::Resolve` carries no reason: abandon reasons are journalled (stderr), not ledgered.
- `health`'s CLI line prints `database=ready socket=owned` from a successful reply; no flock
  custody is held (S-2), so `socket=owned` is INFERRED.
- `task.cancel` replays are not stored: a second cancel re-applies `Cancel` (legal self-edge).
- A bwrap child can outlive `kill -9` of `hee4` (re-parented to the user manager; seen in 2 of 7
  runs of `tests/e2e.rs`). The probe names it R07; nothing kills it in-process. Under the unit,
  `KillMode=control-group` kills it before restart.
