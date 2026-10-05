# hee4-app flow

K6. The `hee4` binary: startup order, the control socket, the owner registry over the 22-id
catalogue (eight ids served by three families plus `tools.*`, the rest refused `unavailable` by
scope), the synchronous dispatcher, `doctor`. Defines no contract type; every state change is
`Store::admit` or `Store::apply`, and the only verdict is `hee4_evidence::decide_and_seal`'s.

```
hee4 serve --socket S --ledger L --work W
  Store::open(L)                      (resets recovery_complete=0)
  probe_workers(store, W)             /proc argv scan: <W>/<task> in a live argv → PidReused (R07), else Absent (R08)
  recovery::reconcile(store, probe)   findings → exit "recovery incomplete", never listens
  Engine::new                         composed(): Registry::new over the families below (a RegistryFault is ServeError::Compose, never listens)
  registry.on_serve_start(engine)     every family's hook in registration order; the first Err is ServeError::Start, never listens
  dispatcher thread                   loop { step(); sleep 100 ms when idle }
  socket::bind(S)                     dir 0700 (owner = our uid), stale socket removed only if nothing answers, socket 0600
  socket::serve                       one thread per connection, at most 256 open (the next is refused too_many_connections, no thread); SO_PEERCRED uid ≠ ours → one `forbidden` frame, close;
                                      one JSON frame per LF line each way
```

## Frame

Request (one line): `{"request_id": str, "action": str, "action_version": 1, "idempotency_key": str|null, "precondition": null|{resource, id, generation}, "body": {}}`.
Other API Map envelope members are ignored. `precondition` is parsed once in `wire::parse`: absent
or null is none; an object with exactly `resource` (string), `id` (string) and `generation`
(unsigned integer) is one; any other shape is `invalid_argument` at `/precondition`. Reply (one line):
`{"kind":"result","request_id","replayed","body"}` or
`{"kind":"error","request_id","code","retry","field","message"}` plus `because`,
`current_generation` and `readback` only when set, and `"effect":"unknown"` only for
`effect_unknown` (the member list is `wire::ERROR_MEMBERS`). A line over 1,048,576 bytes or
EOF inside a line closes the connection with no reply; a line over the bound is answered
`frame_too_large` first, then closed (Socket and IPC Map).

## Families

The catalogue is `hee4_contracts::catalogue`; dispatch is `catalogue::find` then
`Registry::get(owner)`; the registry miss is the one `unavailable` site. Dispatch order, exactly:
`wire::parse` → `catalogue::find` (else `unknown_action` at `/action`) → `action_version` ≠ 1
(`unsupported_action_version`) → `Registry::get(entry.owner)` miss (`unavailable` at `/action`,
`because` = `entry.scope.because()`) → `entry.effect.mutates()`: `idempotency_key` required, then
`not_ready` while `recovery_complete` is false → `PreconditionRule::Required(resource)` with no
`precondition` (`invalid_argument` at `/precondition` naming the resource) → the handler.
`Registry::new` (built once in `Engine::new`, `actions::composed()`) refuses a held owner
(`Judge`, `Deploy`), a duplicate owner, an id the catalogue does not carry, an owner mismatch,
and a family that leaves one of its owner's catalogued ids without a handler
(`RegistryFault::MissingHandler`), by typed name; so `Registry::serve(entry)` misses exactly when
`Registry::get(entry.owner)` does, and dispatch has no "registered, no handler" arm.

| Module | Family | Owner | Scope | Ids | Hook |
|---|---|---|---|---|---|
| `actions/task.rs` | `HEALTH` | `App` | v4.0 | `health` | none |
| `actions/task.rs` | `FAMILY` | `Task` | v4.0 | `task.submit`, `task.get`, `task.list`, `task.cancel`, `task.preview`, `task.resolve` | none |
| `actions/task.rs` | `EVENTS` | `Notify` | v4.0 | `events.subscribe` | none |
| `actions/tools.rs` | `FAMILY` | `Actions` | v4.0 | `tools.list`, `tools.inspect` | none |
| `actions/roster.rs` | `FAMILY` | `Roster` | v4.1 | `roster.list`, `roster.inspect`, `roster.update`, `roster.disable` | composes `model:<HEE4_MODEL>` under `Owner::Deploy` (`deploy.install`, principal `deploy`) via `Store::roster_compose_deploy`; prints `roster deploy record=<id> replayed=<bool> operation=<id>` |

Unregistered owners (`Roster` v4.1; `Service`, `Cohort`, `Numerical` v4.2; `Judge` held) have no
module: their ids are catalogued, listed by `tools.list`, inspected by `tools.inspect`, and refused
`unavailable` on invocation. Wave-2 families add one line to `composed()` and nothing in `lib.rs`.

| Action | Body | Result body | Door |
|---|---|---|---|
| `health` | `{}` | `{ok, head_sha, recovery_complete, uptime_s}` | `Store::recovery_complete` |
| `tools.list` | `{query: null\|string ≤ 256 bytes, page: {limit 1..100, cursor}}` | `{catalogue_revision, page: {items: [{id, version, purpose, effect}], cursor}}`; items = entries whose id or purpose contains `query`, sorted by id, keyset-paged (`actions/page.rs`; cursor `{after_key, boot, filter_sha256}` pinned to `Engine::boot` and the query digest) | `catalogue::CATALOGUE`, `revision()`; no store |
| `tools.inspect` | `{action, version}` | `{action, version, purpose, effect, request_schema_sha256, result_schema_sha256, error_schema_sha256, max_request_bytes, max_deadline_ms, readback_action}`; the digests are descriptor digests (SHA-256 over canonical `{action, version, fields}`); a held id inspects as a result | `catalogue::find`; no store |
| `task.submit` | `{brief: "<eleven fields>"}` + `idempotency_key` | `{task_id, phase}`; `replayed` | `Brief::parse` + `check_restatement`, then `Store::admit` |
| `task.get` | `{task_id}` | `{task_id, phase, events, last_receipt_hash}` | `Store::phase`, `history`, `chain_head` |
| `task.list` | `{}` | `{tasks: [{task_id, phase}]}` | `Store::task_ids`, `phase` |
| `task.cancel` | `{task_id}` + `idempotency_key` | `{task_id, phase}` | `Store::apply(Event::Cancel)` |
| `task.preview` | `{brief}` | `{eligible: true, model}` or `{eligible: false, refusal, message}` | `Brief::parse` + `check_restatement` + `dispatcher::route` (`route::select`); admits nothing |
| `task.resolve` | `{task_id, resolution: "quarantine"\|"abandon", reason?}` + `idempotency_key` | `{task_id, phase}` | `Store::apply(Event::Resolve(..))`; `reason` is a typed wire string from `actions/task.rs` `ABANDON_REASONS` / `effect_unknown_permanent_r01..r14` (default `attempt_failed` / `_r10`); an unknown one is `invalid_argument` |
| `events.subscribe` | `{since_seq: u64\|null}` | ack `{since_seq, stream:"events"}`, then the stream below | read-only SQLite connection + `Store::history` |

Families notes: `tools.inspect` of an unknown id is `unknown_action` at `/body/action` (Error map
F-2, the one `catalogue::find` site; `tools.inspect.md` left it UNWRITTEN against API Map A-03
`not_found`: DC proposal, the feature file edit belongs to drive-completion). The CLI is
`hee4 <action> [--key K] [--body JSON | --body-file F] [--precondition JSON] [--socket P]` for any
catalogued id, the five positional forms unchanged; an uncatalogued action is exit 2 naming the
catalogue. `hee4 --version` prints `hee4 <VERSION> <head12> catalogue=<revision12>`.

`head_sha` is baked by `build.rs`: env `HEE4_HEAD` (40 hex; the gate sets it, its export has no `.git`), else `git rev-parse HEAD`, else `unknown` (the dispatcher refuses to dispatch on `unknown`).

`task_id` = `t-` + 24 hex of SHA-256(`principal \n idempotency_key`), so a key names one task.
The brief text is written to `<W>/briefs/<task>.brief` under the same ledger lock as `admit`.

## Refusal names (one per refusal)

| Name | Retry | When | Field |
|---|---|---|---|
| `invalid_argument` | never | line not a JSON object; `request_id`/`action`/`action_version`/`body` missing or mistyped; `precondition` of another shape than null or `{resource, id, generation}`; a `Required(resource)` action without one; mutating action without `idempotency_key`; `tools.list` `query` over 256 bytes or not a string, `page` not an object, `page.limit` outside 1..100, a malformed `page.cursor`; `tools.inspect` `action`/`version` missing or mistyped; brief missing a field, duplicate field, empty RESTATEMENT; bad `task_id`; `resolution` not quarantine/abandon; `reason` not in that resolution's table; `since_seq` not a u64 | the member's pointer (`/`, `/precondition`, `/body/brief`, `/body/page/limit`, …) |
| `unknown_action` | never | action not in `catalogue::CATALOGUE` (`/action`); `tools.inspect` of an uncatalogued id (`/body/action`) | `/action`, `/body/action` |
| `unsupported_action_version` | never | `action_version` ≠ 1 | `/action_version` |
| `not_ready` | after_condition | mutating action while the ledger's `recovery_complete` is false | `/action` |
| `conflict` | after_readback | same idempotency key, other body bytes; `transition` refused the cancel or the resolve | `/idempotency_key`, `/body/task_id` |
| `not_found` | never | `task.get`/`task.cancel`/`task.resolve` of a task never admitted | `/body/task_id` |
| `forbidden` | never | `SO_PEERCRED` uid ≠ the process uid (or unreadable); sent before any request is read, then close | `/` |
| `internal` | same_exact_request | the ledger failed under the request | `/` |
| `no_route` | after_condition | `task.preview`: `route::select` refused the brief (result body `refusal`, not an error frame) | — |
| `slow_consumer` | after_condition | `events.subscribe`: the subscriber is 256 frames behind; `{"kind":"close",…}` frame, then close. Best effort: written with a 200 ms write deadline; if the peer's socket buffer is full the frame is not delivered, the server logs `slow_consumer close frame not delivered`, and the client sees EOF | — |
| `frame_too_large` | never | a request line over 1,048,576 bytes; error frame, then close | `/` |
| `too_many_connections` | after_condition | 256 connections already open; error frame written from the accept loop before any thread is spawned, then close | `/` |
| `unavailable` | after_condition | the registry miss in `dispatch`: the action is catalogued but its owner is not registered in this release; `because` is the scope's text (v4.0 "owner not composed", v4.1 the roster family, v4.2 the service/cohort/numerical families, held "H-8") | `/action` |
| `stale_generation` | never | no emitter in this release; reserved for the roster/service/task families (`precondition.generation` behind the resource's; `current_generation` set) | `/precondition` |
| `resync_required` | never | `tools.list`: a page cursor from another `Engine::boot` (`because` "epoch moved") or another query digest ("filter moved"); reserved for the roster/service/task families' listings | `/body/page/cursor` |
| `resource_exhausted` | same_exact_request | no emitter in this release; reserved for the roster/service/task families (a bound on work or storage reached) | `/` |
| `effect_unknown` | after_readback | no emitter in this release; reserved for the roster/service/task families (`readback` names the read that settles it; frame carries `effect: "unknown"`) | `/` |

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
| playbook | VERIFY lines: absolute path → `Run` (bare argv); `sh: <line>` → `Run{/bin/sh, [-c, line]}` (the line runs inside the sandbox: no network, only `$HEE4_MODEL_SOCKET`); `model: <prompt>` → `Generate`, a recorded skip whose reason names `sh:`; else `Unsupported` (named skip) | — |
| route | `route::select` over a one-row roster (`HEE4_MODEL`, default `qwen2.5-coder:7b`), floor local-only, baseline = that model; availability probed (`tags`) only when a model step will run | `Resolve(Abandon(RouteRefused{floor_unmet}))` |
| namespace | `NamespaceTask::with_door_root(task, W, <control socket dir>, needs_model, TIMEBOX)` → `plan_for`; `needs_model` = a `Generate` or `sh:` step and `HEE4_LIVE_MODEL=1` | `Resolve(Abandon(NamespaceRefused))`; work dir → `WorkDirUnavailable`; unknown head → `HeadUnknown`; door upstream unparsable → `NoPermit` |
| permit | `Permit::mint(ReceiptId "r-<task>-<ns>", scope = the Run programs)` | — |
| dispatch | `Store::apply(Dispatch)` → running (refused before reconcile by K1) | — |
| attempt | `Attempt::run` (bwrap for Run steps; Generate steps skip with no loopback when not live, `UNMEASURED` printed) | error or a `Failed` step: `Settle(NotReady)` → `Stop` → failed, no receipt |
| settle | `Settle(Ready)` → verifying | — |
| observe | per observation: `observation_id`, `Store::record_observation`, `apply(Observe)` | — |
| decide + seal | `decide_and_seal(chain_head, receipt_id, ids, obs, subject)`; ids: collector = digest(ledger epoch), locks = digest(permit), standards = digest(`gate.toml` baked at build); subject input = first model prompt, else VERIFY text | — |
| receipt | `Store::append_receipt` (K1 re-runs `verify_chain`) | — |
| verdict | `apply(Decide(verdict))`; `Pass` → `apply(Accept)` | — |

Every `Run` step records a tier-0 observation (tool `command`): `Pass` on exit 0, `Fail` otherwise,
evidence `exit` and `stdout`. Every observation's `input_sha256` is the digest of the VERIFY text,
which is also the receipt subject's input, so `decide` reconciles them (an observation over any
other input is `Refused(Unreconciled)`: the first live run hit exactly that with the door
observation's old request-digest input). A step that fails still ends `failed` with no receipt
(`Settle(NotReady)`, `Stop`); the `Fail` observation is in the outcome only. A `sh:` step that
exits 0 and reaches the model also adds the door observation (one `model_request` row per
request): `Pass` → `accepted`. A `/usr/bin/true` brief now ends `accepted` (exit evidence);
the `live_model_attempt_through_the_door` e2e test (`HEE4_LIVE_MODEL=1`, model on 11434, else an
`UNMEASURED` line) drives a `curl --unix-socket "$HEE4_MODEL_SOCKET"` line to `accepted`.

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
- `task.cancel.md:9` and `task.resolve.md:10` say `precondition` is required in the v4 design; the
  deployed skeleton takes none (`task.cancel.md:49`, `tools/drive` `d_cancel` sends none), so their
  catalogue `PreconditionRule` is `None` at this release (DC proposal; the feature files are
  drive-completion's).
- `Engine::boot` (the page-cursor epoch) is unix nanoseconds drawn once in `Engine::new`;
  K1-store-foundation's `Store::boot()` replaces the source in a later wave (one line in
  `actions/mod.rs`, `page.rs` unchanged).
