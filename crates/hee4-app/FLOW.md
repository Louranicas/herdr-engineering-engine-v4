# hee4-app flow

K6. The `hee4` binary: startup order, the control socket, eight actions (one a stream), the synchronous
dispatcher, `doctor`. Defines no contract type; every state change is `Store::admit` or
`Store::apply`, and the only verdict is `hee4_evidence::decide_and_seal`'s.

```
hee4 serve --socket S --ledger L --work W
  Store::open(L)                      (resets recovery_complete=0)
  probe_workers(store, W)             /proc argv scan: <W>/<task> in a live argv → PidReused (R07), else Absent (R08)
  recovery::reconcile(store, probe)   findings → exit "recovery incomplete", never listens
  dispatcher thread                   loop { step(); sleep 100 ms when idle }
  socket::bind(S)                     dir 0700 (owner = our uid), stale socket removed only if nothing answers, socket 0600
  socket::serve                       one thread per connection; SO_PEERCRED uid ≠ ours → one `forbidden` frame, close;
                                      one JSON frame per LF line each way
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
| `task.preview` | `{brief}` | `{eligible: true, model}` or `{eligible: false, refusal, message}` | `Brief::parse` + `check_restatement` + `dispatcher::route` (`route::select`); admits nothing |
| `task.resolve` | `{task_id, resolution: "quarantine"\|"abandon", reason?}` + `idempotency_key` | `{task_id, phase}` | `Store::apply(Event::Resolve(..))`; `reason` is a typed wire string from `actions.rs` `ABANDON_REASONS` / `effect_unknown_permanent_r01..r14` (default `attempt_failed` / `_r10`); an unknown one is `invalid_argument` |
| `events.subscribe` | `{since_seq: u64\|null}` | ack `{since_seq, stream:"events"}`, then the stream below | read-only SQLite connection + `Store::history` |

`head_sha` is baked by `build.rs`: env `HEE4_HEAD` (40 hex; the gate sets it, its export has no `.git`), else `git rev-parse HEAD`, else `unknown` (the dispatcher refuses to dispatch on `unknown`).

`task_id` = `t-` + 24 hex of SHA-256(`principal \n idempotency_key`), so a key names one task.
The brief text is written to `<W>/briefs/<task>.brief` under the same ledger lock as `admit`.

## Refusal names (one per refusal)

| Name | Retry | When | Field |
|---|---|---|---|
| `invalid_argument` | never | line not a JSON object; `request_id`/`action`/`action_version`/`body` missing or mistyped; mutating action without `idempotency_key`; brief missing a field, duplicate field, empty RESTATEMENT; bad `task_id`; `resolution` not quarantine/abandon; `reason` not in that resolution's table; `since_seq` not a u64 | the member's pointer (`/`, `/body/brief`, …) |
| `unknown_action` | never | action not one of the eight | `/action` |
| `unsupported_action_version` | never | `action_version` ≠ 1 | `/action_version` |
| `not_ready` | after_condition | mutating action while the ledger's `recovery_complete` is false | `/action` |
| `conflict` | after_readback | same idempotency key, other body bytes; `transition` refused the cancel or the resolve | `/idempotency_key`, `/body/task_id` |
| `not_found` | never | `task.get`/`task.cancel`/`task.resolve` of a task never admitted | `/body/task_id` |
| `forbidden` | never | `SO_PEERCRED` uid ≠ the process uid (or unreadable); sent before any request is read, then close | `/` |
| `internal` | same_exact_request | the ledger failed under the request | `/` |
| `no_route` | after_condition | `task.preview`: `route::select` refused the brief (result body `refusal`, not an error frame) | — |
| `slow_consumer` | after_condition | `events.subscribe`: the subscriber is 256 frames behind; `{"kind":"close",…}` frame, then close | — |

`wire::tests::every_emittable_refusal_has_one_row_in_flow` parses this table and asserts its names
equal `wire::Code::ALL`, each once. The API Map names the queue overflow `queue_limit` (A-10);
this slice uses `slow_consumer` as briefed (DC proposal).

`not_ready` is not in the Error map's 18 codes; the map's nearest is `unavailable` (after_condition).
DC proposal below.

## Dispatcher (one task at a time)

| Step | Door | On refusal |
|---|---|---|
| first `admitted` task | `Store::task_ids`, `phase` | — |
| brief | `<W>/briefs/<task>.brief`, `Brief::parse` | `Resolve(Abandon(BriefUnreadable))` |
| playbook | VERIFY lines: absolute path → `Run`; `model: <prompt>` → `Generate`; else `Unsupported` (named skip) | — |
| route | `route::select` over a one-row roster (`HEE4_MODEL`, default `qwen2.5-coder:7b`), floor local-only, baseline = that model; availability probed (`tags`) only when a model step will run | `Resolve(Abandon(RouteRefused{floor_unmet}))` |
| namespace | `NamespaceTask::new(task, W, needs_model, TIMEBOX)` → `plan_for`; `needs_model` = a `Generate` step and `HEE4_LIVE_MODEL=1` | `Resolve(Abandon(NamespaceRefused))`; work dir → `WorkDirUnavailable`; unknown head → `HeadUnknown`; door upstream unparsable → `NoPermit` |
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

`socket::Admitted::check` reads `SO_PEERCRED` through `rustix::net::sockopt::socket_peercred`
(safe API; `unsafe_code = "forbid"` holds) and compares the uid with the process uid from
`/proc/self` (`socket::peer_allowed`). `connection` takes an `Admitted`, so an unchecked stream
cannot be served. The 0700 directory stays as the first gate. The principal is still the
process uid, not the peer's (they are equal by the check).

## Stream (`events.subscribe`)

After the ack, one line per ledger `events` row with `seq > since_seq`, in `seq` order:
`{"kind":"event","seq","task_id","event","phase_after","ts"}`. `event` is the contracts'
`Serialize` spelling; `phase_after` is `TaskState::replay` over the task's history up to that row.
A reader thread polls the ledger every 100 ms through its own read-only connection and
`try_send`s into a 256-frame queue; a full queue sets `slow_consumer`, the writer sends the close
frame and shuts the socket. A watcher thread reads the socket to EOF so a closed client ends all
three threads. Resume with `since_seq` = the last `seq` received: exactly-once by `seq`.

## Gaps

- The brief lives in a file, not the ledger (no `Store` brief column or reader). A crash between
  `admit` and the write leaves an admitted task the dispatcher abandons by name.
- `health`'s CLI line prints `database=ready socket=owned` from a successful reply; no flock
  custody is held (S-2), so `socket=owned` is INFERRED.
- `task.cancel` replays are not stored: a second cancel re-applies `Cancel` (legal self-edge).
  `task.resolve` likewise: a replayed resolve re-applies (abandon twice → `conflict`).
- The stream reads `events` through a second, read-only SQLite connection: `Store` has no
  `events_since(seq)` reader (DC proposal). Writes stay `Store::*` only.
- A bwrap child can outlive `kill -9` of `hee4` (re-parented to the user manager; seen in 2 of 7
  runs of `tests/e2e.rs`). The probe names it R07; nothing kills it in-process. Under the unit,
  `KillMode=control-group` kills it before restart.
