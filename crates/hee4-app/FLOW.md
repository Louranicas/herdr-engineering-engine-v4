# hee4-app flow

K6. The `hee4` binary: startup order, the control socket, the owner registry over the 22-id
catalogue (eight ids served by three families plus `tools.*`, the rest refused `unavailable` by
scope), the synchronous dispatcher, `doctor`. Defines no contract type; every state change is
`Store::admit` or `Store::apply`, and the only verdict is `hee4_evidence::decide_and_seal`'s.

```
hee4 serve --socket S --ledger L --work W [--budgets F] [--backups B]
  HEE4_REQUIRE_BACKUPS=1, no --backups  `hee4 serve refused: backups: --backups is required (HEE4_REQUIRE_BACKUPS=1)`,
                                      exit 1, before the budgets file and the ledger; no socket
  Budgets::parse(read F)              F = --budgets, else env HEE4_BUDGETS, else Budgets::DEFAULT; read once, before
                                      the ledger; a refusal is `hee4 serve refused: budgets: <field> ...`, exit 1, never listens
  Store::open(L)                      (resets recovery_complete=0)
  open_attempts ≤ recovery.open_attempt_limit   else `hee4 serve refused: budgets: recovery.open_attempt_limit=<n> but the ledger
                                      holds <m> open attempts`, exit 1, no probe, never listens
  probe::observe(open_attempts)       per open attempt: custody from /proc/<pid>/stat vs the recorded (pid, start_ticks),
                                      workspace readback bounded by recovery.workspace_readback_bytes; one
                                      `recovery probe attempt=<id> custody=<..> workspace=<..>` line each
  recovery::reconcile(store, probe)   one `recovery task= rule= reason= workspace= <before> -> <after>` line per task;
                                      findings → exit "recovery incomplete", never listens
  Engine::new                         composed(): Registry::new over the families below (a RegistryFault is ServeError::Compose, never listens)
  registry.on_serve_start(engine)     every family's hook in registration order; the first Err is ServeError::Start, never listens
  Engine::with_budgets(budgets)       the one Budgets every app limit reads (`Engine::budgets`)
  backup (when --backups B)           `Backups::open(B, head12)` (creates B, opens B/backup.log for append, its last PASS
                                      line is the last backup), then K1 `backup::backup_to(store, W, B, SameDisk::Refuse)`
                                      with trigger `backup_due(..)` else `start`; one `backup.log` line; an Err is
                                      `hee4 serve refused: backups: <k1 error>`, never listens (the store goes into service
                                      only after a backup, DC-22). No backup thread, timer or daemon (V4-6)
  dispatcher thread                   loop { step(); sleep dispatcher.idle_ms when idle, dispatcher.error_backoff_ms after an error }
  socket::bind(S)                     dir 0700 (owner = our uid), stale socket removed only if nothing answers, socket 0600
  socket::serve                       one thread per connection, at most socket.max_connections open (the next is refused
                                      too_many_connections, no thread); startup line ends `budgets=default|file:<F> backups=<B>|none`; SO_PEERCRED uid ≠ ours → one `forbidden` frame, close;
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
`effect_unknown` (the member list is `wire::ERROR_MEMBERS`). A line over `socket.frame_bytes`
(default 1,048,576) or EOF inside a line closes the connection with no reply; a line over the bound
is answered `frame_too_large` first, then closed (Socket and IPC Map). Read and write deadlines are
`socket.read_deadline_ms`/`write_deadline_ms`; the CLI client reads with
`Budgets::DEFAULT.socket.client_read_ms` (it reads no budgets file).

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
| `actions/service.rs` | `FAMILY` | `Service` | v4.2 | `service.inspect`, `service.probe`, `service.action` (busctl only through `service_runner.rs` → `spawn::plan/run`; commits through `Store::service_*`; `service.action` takes the durable `service_claim` after every request check and before `runner.act`, releases it after the commit whatever the outcome; no in-process lock) | seed `service_facts` (`self`, `model`, `drive`), pin the busctl digest (`HEE4_BUSCTL_SHA256` or measured), print `busctl_sha256=`, print one `stale_claim service_id= operation_id= generation= age_ms= reason=` line per stale act claim |

Unregistered owners (`Cohort`, `Numerical` v4.2; `Judge` held) have no
module: their ids are catalogued, listed by `tools.list`, inspected by `tools.inspect`, and refused
`unavailable` on invocation. Wave-2 families add one line to `composed()` and nothing in `lib.rs`.

| Action | Body | Result body | Door |
|---|---|---|---|
| `health` | `{}` | `{ok, head_sha, recovery_complete, uptime_s, schema_version, serve_cgroup, budgets, budgets_inert}`; `budgets` is `Engine::budgets` serialised, and re-parses through `Budgets::parse`; `budgets_inert` is `crate::INERT_BUDGETS`, the loaded fields nothing reads yet (`attempt.ctx_tokens`, `ledger.busy_timeout_ms`, `ledger.checkpoint_every`) (`hee4 doctor` row `budgets`, `tools/doctor` check `budgets`) | `Store::recovery_complete` |
| `tools.list` | `{query: null\|string ≤ 256 bytes, page: {limit 1..100, cursor}}` | `{catalogue_revision, page: {items: [{id, version, purpose, effect}], cursor}}`; items = entries whose id or purpose contains `query`, sorted by id, keyset-paged (`actions/page.rs`; cursor `{after_key, boot, filter_sha256}` pinned to `Engine::boot` and the query digest) | `catalogue::CATALOGUE`, `revision()`; no store |
| `tools.inspect` | `{action, version}` | `{action, version, purpose, effect, request_schema_sha256, result_schema_sha256, error_schema_sha256, max_request_bytes, max_deadline_ms, readback_action, scope, served}`; the digests are descriptor digests (SHA-256 over canonical `{action, version, fields}`); `scope` is the entry's `Scope::wire_name`; `served` is `Registry::serve(entry).is_some()` (the owner is composed in this binary; false is exactly the ids dispatch refuses `unavailable`); a held id inspects as a result | `catalogue::find`, `Registry::serve`; no store |
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
| `invalid_argument` | never | line not a JSON object; `request_id`/`action`/`action_version`/`body` missing or mistyped; `precondition` of another shape than null or `{resource, id, generation}`; a `Required(resource)` action without one; mutating action without `idempotency_key`; `tools.list` `query` over 256 bytes or not a string, `page` not an object, `page.limit` outside 1..100, a malformed `page.cursor`; `tools.inspect` `action`/`version` missing or mistyped; brief missing a field, duplicate field, empty RESTATEMENT; bad `task_id`; `resolution` not quarantine/abandon; `reason` not in that resolution's table; `since_seq` not a u64; `service.*` body fields (`probe_id`, `max_cost_microunits` ≠ 0, `unit_id` not the service's, `action`, a malformed `expected_owner_sha256`, `operation`) and a `service.action` precondition naming another resource | the member's pointer (`/`, `/precondition`, `/body/brief`, `/body/page/limit`, `/body/unit_id`, …) |
| `unknown_action` | never | action not in `catalogue::CATALOGUE` (`/action`); `tools.inspect` of an uncatalogued id (`/body/action`) | `/action`, `/body/action` |
| `unsupported_action_version` | never | `action_version` ≠ 1; `service.probe` `probe_version` ≠ 1 | `/action_version`, `/body/probe_version` |
| `not_ready` | after_condition | mutating action while the ledger's `recovery_complete` is false | `/action` |
| `conflict` | after_readback | same idempotency key, other body bytes; `transition` refused the cancel or the resolve; `service.action` `expected_owner_sha256` ≠ the row's; `service.action` while another act (this or another serve on the ledger) holds the service's live claim (`because` "service claim held") | `/idempotency_key`, `/body/task_id`, `/body/expected_owner_sha256`, `/body/service_id` |
| `not_found` | never | `task.get`/`task.cancel`/`task.resolve` of a task never admitted; `service.*` of an unseeded `service_id`; `service.inspect` of an operation key not on this service; `service.action` answered `Unit X not found.`/`not loaded.` by the manager | `/body/task_id`, `/body/service_id`, `/body/operation`, `/body/unit_id` |
| `forbidden` | never | `SO_PEERCRED` uid ≠ the process uid (or unreadable); sent before any request is read, then close; `service.probe` `network_scope` ≠ `none`; `service.action` on a service seeded not actable (`because` "service not actable") | `/`, `/body/network_scope`, `/body/service_id` |
| `internal` | same_exact_request | the ledger failed under the request | `/` |
| `no_route` | after_condition | `task.preview`: `route::select` refused the brief (result body `refusal`, not an error frame) | — |
| `slow_consumer` | after_condition | `events.subscribe`: the subscriber's `stream.queue_frames`-frame queue accepted no frame for `stream.stall_ms` (behind but draining is never dropped); `{"kind":"close",…}` frame, then close. Best effort: written with a `stream.close_deadline_ms` write deadline; if the peer's socket buffer is full the frame is not delivered, the server logs `slow_consumer close frame not delivered`, and the client sees EOF | — |
| `frame_too_large` | never | a request line over `socket.frame_bytes` bytes (the message prints the active bound); error frame, then close | `/` |
| `too_many_connections` | after_condition | `socket.max_connections` connections already open (the message prints the count and the field); error frame written from the accept loop before any thread is spawned, then close | `/` |
| `unavailable` | after_condition | the registry miss in `dispatch`: the action is catalogued but its owner is not registered in this release; `because` is the scope's text (v4.0 "owner not composed", v4.1 the roster family, v4.2 the cohort/numerical families, held "H-8"); `service.probe`/`service.action` before the call: `because` "busctl digest", "head unknown", "user bus absent", "service runner not started", or "manager refused" (a manager error reply, at `/body/unit_id`) | `/action`, `/`, `/body/unit_id` |
| `stale_generation` | never | `service.action`: `precondition.generation` ≠ the service's (`current_generation` set); reserved for the roster/task families | `/precondition/generation` |
| `resync_required` | never | `tools.list`: a page cursor from another `Engine::boot` (`because` "epoch moved") or another query digest ("filter moved"); reserved for the roster/service/task families' listings | `/body/page/cursor` |
| `resource_exhausted` | same_exact_request | `service.probe`: busctl stdout over the 4096 B bound (the message names both numbers); reserved for the roster/task families | `/` |
| `effect_unknown` | after_readback | `service.action`: the call was sent but its read-back did not settle or faulted; no operations row (`readback` `service.inspect`; frame carries `effect: "unknown"`); reserved for the roster/task families | `/` |

`wire::tests::every_emittable_refusal_has_one_row_in_flow` parses this table and asserts its names
equal `wire::Code::ALL`, each once. The API Map names the queue overflow `queue_limit` (A-10);
this slice uses `slow_consumer` as briefed (DC proposal).

`not_ready` is not in the Error map's 18 codes; the map's nearest is `unavailable` (after_condition).
DC proposal below.

## Dispatcher (one task at a time)

| Step | Door | On refusal |
|---|---|---|
| pick (`next_task`, oldest id first) | `Store::task_ids`, `phase`: an `admitted` task is dispatched; a `cancellation_requested` task with no open attempt (`recovery::Facts::attempt_open` over `history_with_seq`: cancelled before its `Dispatch`, or between the pick and `Dispatch`, whose `Dispatch` K1 refuses so the next step picks it) gets `Store::apply(Stop)` → `cancelled`, events `admit,cancel,stop`, no attempt row; with an open attempt it is not picked (its attempt settles first). Tests `dispatcher::tests::a_cancel_before_dispatch_is_stopped_once`, `a_cancel_with_an_open_attempt_is_not_stopped`, e2e `a_cancel_before_dispatch_reaches_cancelled` | — |
| brief | `<W>/briefs/<task>.brief`, `Brief::parse` | `Resolve(Abandon(BriefUnreadable))` |
| playbook | `hee4_contracts::VerifyLine::parse_all` (K0 owns the grammar), mapped exhaustively, no wildcard: `Shell{command}` → `Run{/bin/sh, [-c, command]}` (the line runs inside the sandbox: no network, only `$HEE4_MODEL_SOCKET`); `Exec{program, args}` → `Run` (bare argv); `Unsupported{kind}` → `Unsupported{kind}` (`model:` is `kind:"model"`), a named skip whose reason names `sh:`. Admission already ran `Brief::check_verify`; the dispatcher does not re-check | — |
| route | `route::select` over a one-row roster (`HEE4_MODEL`, default `qwen2.5-coder:7b`), floor local-only, baseline = that model; availability probed (`tags`) only when a model step will run | `Resolve(Abandon(RouteRefused{floor_unmet}))` |
| namespace | per-generation work root: generation = latest attempt's + 1; `NamespaceTask::with_door_root(task, <W>/<task>/<generation>, <control socket dir>, needs_model, min(TIMEBOX else attempt.timebox_default_ms, attempt.deadline_ms))` (`dispatcher::timebox`: minutes saturate, never wrap or panic; every step's kill deadline is at most `attempt.deadline_ms`) → `plan_for` (unaltered), so the work dir is `<W>/<task>/<generation>/<task>` (the trailing `/<task>` is the namespace's; DC ask: a generation-aware constructor). An old generation's dir is never removed (R11 readback only); `needs_model` = a `sh:` step (`dispatcher::wants_model`, shared with `task.preview`) and `HEE4_LIVE_MODEL=1` | `Resolve(Abandon(NamespaceRefused))`; work dir → `WorkDirUnavailable`; unknown head → `HeadUnknown`; door upstream unparsable → `NoPermit` |
| permit | `Permit::mint(ReceiptId "r-<task>-<ns>", scope = the Run programs)` | — |
| backup | with `--backups`: `backup_due(BackupFacts{last, dispatched_since, now, head})` (pure: no backup → `start`; another head → `upgrade`; older than `DC22_FRESHNESS` 15 min, strictly → `stale`; `dispatched_since >= DC22_BATCH_TASKS` 8 → `batch`; else none; `now` from `SystemClock`), then `Backups::take` → K1 `backup_to`; one line `backup id= objects= age_s= trigger= head=<sha12> verdict=PASS\|FAIL [reason=<k1 refusal name>]` on stderr and in `B/backup.log` (a FAIL line once per failing streak; the log fd is held open, so it lands even when B refuses) | `DispatchError::Backup`: no `Dispatch` is applied, the task stays `admitted`, the loop prints `dispatch error: backup: ..`, backs off `dispatcher.error_backoff_ms` and retries |
| dispatch | `Store::apply(Dispatch)` → running and K1 opens the row `a-<task>-<generation>` in the same transaction (refused before reconcile by K1) | — |
| attempt_started | the open row read back (`Store::open_attempts`, never formatted here; its generation must equal the work dir's), then `Store::attempt_started(id, AttemptStart{receipt_id, permit_id, model, head_sha, workspace = ns.work_dir(), lease: None})`; prints `dispatch task= attempt= started workspace=`. No lease is issued (a lease is what R09 compares before reuse), so `attempt.deadline_ms` is not recorded | `StoreError::WorkspaceLeased` → `Resolve(Abandon(WorkDirUnavailable))`, the store's text printed; any other error or a missing/mismatched row → `dispatch task= attempt_started error=`, `Settle(NotReady)` → `Stop`; never a silent run |
| attempt | `Attempt::with_budget(model, head, budgets.door).run(.., on_start)` (bwrap for Run steps; `sh:` steps run without a door when not live, `UNMEASURED` printed) | error or a `Failed` step: `Settle(NotReady)` → `Stop` → failed, no receipt |
| attempt_pid | `on_start(pid, start_ticks)` from `spawn::start` before the wait, per Run step: `Store::attempt_pid(id, pid, start_ticks)`, prints `dispatch task= attempt= pid= start_ticks=`; the worker calls no store | a store error is printed `attempt_pid error=` (the child is already running) |
| repair | D6: `Settle(NotReady)` or `Decide(Fail)` → `repair_pending` → redispatch under `attempt.max_generations`, else `Stop`. `attempt.max_generations` is absent from K0's `AttemptBudget`, so today's parking stands (`Settle(NotReady)` → `Stop` → failed) and `next_task` ignores `repair_pending`; the two repair e2e tests print `UNMEASURED: attempt.max_generations absent` and fail once the field lands | — |
| settle | `Settle(Ready)` → verifying | — |
| ddf | before the attempt, `Snapshot::of(ns.work_dir(), DDF_DIFF_BYTES)`; after it, `workspace_diff(ws, &before, DDF_DIFF_BYTES)`: no `<ws>/.git` → `Diff::NoWorktree`; a `.git` that is a file or symlink (`symlink_metadata`, never followed) → `git_dir_not_dir`; else the workspace walked in-process (no symlink followed, FIFOs/devices skipped, every `.git` entry left out) and diffed against the snapshot as a unified patch. **No git runs on the host over a candidate-written workspace**: a planted `core.fsmonitor`, `filter.*.clean`, `diff.*.textconv` or `gitdir:` pointer is never read (e2e `a_candidate_git_config_runs_nothing_on_the_host`). Then K4 `ddf::for_task(diff, &subject, &SystemClock, timebox)` (the attempt's TIMEBOX). `Observed(o)` of any outcome (Pass, exit-7 advisory `Refused`, timeout `Error`) joins the observations; prints `dispatch task= ddf=observed tool=<name> <version>` | a log line and nothing else: `dispatch task= ddf=skipped reason=no_worktree\|no_diff\|tool_absent\|adapter_error:<variant>\|diff_error:<io\|too_large\|git_dir_not_dir>`; never a refusal, never an abandon, never `Settle(NotReady)`; the dispatcher never reads the outcome, the lattice does |
| observe | per observation (the attempt's, then the ddf one): `observation_id`, `Store::record_observation`, `apply(Observe)` | — |
| decide + seal | `decide_and_seal(chain_head, receipt_id, ids, obs, subject)`; ids: collector = digest(ledger epoch), locks = digest(permit), standards = digest(`gate.toml` baked at build); the one subject (built once in `step`, `subject_of`) has input = digest(VERIFY text), the same subject ddf bound its observation to | — |
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

## Restore

`hee4 restore --into D ID [--backups B]` (B default `/mnt/storage-10tb/hee4-backups`, the unit's
`--backups`): K1 `backup::restore(B/ID, D)`; prints and appends to `B/restore.log` one line
`restore backup=<id> ledger=<sha12> objects=<n>/<total> rto_s=<t> verdict=PASS`, or
`... verdict=FAIL reason=<name>`: `not_found` when `B/ID` is not a directory (or ID is not one
path component), else K1's refusal (`target_occupied`, `incomplete`, `digest_mismatch`,
`objects_missing`, `manifest`, `io`, ...; `dispatcher::backup_error_name`). Exit 0 only on PASS.
No socket, no engine; the ledger is never opened or copied here.

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
A reader thread polls the ledger every `stream.poll_ms` (`stream.batch_rows` rows a read) through
its own read-only connection and offers into a `stream.queue_frames`-frame queue, waiting for space up to `stream.stall_ms` per frame (each accepted frame restarts the window; no ledger read is open while it waits); a queue that accepts nothing for the window sets `slow_consumer`, the writer sends the close
frame and shuts the socket. A watcher thread reads the socket to EOF so a closed client ends all
three threads. Resume with `since_seq` = the last `seq` received: exactly-once by `seq`.

## Gaps

- DC-22 numbers are consts in `dispatcher.rs` (`DC22_FRESHNESS` 15 min, `DC22_BATCH_TASKS` 8,
  plan/DECISIONS.md:199-204) until K0 adds `backup.freshness_ms` / `backup.batch_tasks` to
  `Budgets` (hee4-contracts-architect follow-up); they are the only copy.
- ddf has no per-attempt budget field: it runs under the attempt's TIMEBOX (`dispatcher::timebox`).
  Its timeout observation is tier-0, so a hung deep-diff-forge gates the task (K4 DC, evidence FLOW).
- The sandbox mounts no `/dev`, so `git` cannot run inside a VERIFY (exit 128, `/dev/null`;
  MEASURED); the e2e's candidate makes its `.git` with `mkdir`. The `--dev /dev` mount is a
  hee4-host/hee4-worker follow-up.
- `DDF_DIFF_BYTES` (16 MiB, each walk) is a const in `dispatcher.rs`, an UNMEASURED stand-in
  until K0 adds an `attempt.diff_bytes` `Budgets` field. The patch is whole-file hunks, not a
  minimal diff.
- deep-diff-forge is found on `PATH`; the unit's `PATH` is the user manager's, which may not hold
  `~/.local/bin` (then `ddf=skipped reason=tool_absent`; UNMEASURED under the unit).

- The brief lives in a file, not the ledger (no `Store` brief column or reader). A crash between
  `admit` and the write leaves an admitted task the dispatcher abandons by name.
- `health`'s CLI line prints `database=ready socket=owned` from a successful reply; no flock
  custody is held (S-2), so `socket=owned` is INFERRED.
- `task.cancel` replays are not stored: a second cancel re-applies `Cancel` (legal self-edge).
  `task.resolve` likewise: a replayed resolve re-applies (abandon twice → `conflict`).
- The stream reads `events` through a second, read-only SQLite connection: `Store` has no
  `events_since(seq)` reader (DC proposal). Writes stay `Store::*` only.
- The startup probe reads the ledger's open attempts (`probe::observe(open_attempts)`), not an
  argv scan: an attempt with a recorded `(pid, start_ticks)` whose process is gone is R08
  (`AcknowledgedWorkerLost`, or `DispatchUnacknowledged` with no `attempt_started`); no pid is
  `Unobserved` → R07. A bwrap child can outlive `kill -9` of `hee4` (re-parented to the user
  manager); it is then `LiveSameIdentity` by pid+starttime → R06, observe-only (no reaper, no
  re-attach). Under the unit, `KillMode=control-group` kills it before restart; the kill-9 e2e
  kills it by the recorded pid before restarting.
- Limits not on `Budgets` (kept by design): polling periods (`spawn.rs` 10 ms, `model_door.rs`
  5 ms), `DOOR_PATH_MAX` 107 (the kernel's), the store `busy_timeout` (K1's), `bounds.rs` token
  limits; the roster default `ctx_tokens: 32_768` lives in `actions/roster.rs` (not this slice's
  file; `attempt.ctx_tokens` is not yet read). Every loaded field nothing reads is listed in `health`'s
  `budgets_inert` (`crate::INERT_BUDGETS`), so the reply never reports it as applied.
- `task.cancel.md:9` and `task.resolve.md:10` say `precondition` is required in the v4 design; the
  deployed skeleton takes none (`task.cancel.md:49`, `tools/drive` `d_cancel` sends none), so their
  catalogue `PreconditionRule` is `None` at this release (DC proposal; the feature files are
  drive-completion's).
- `Engine::boot` (the page-cursor epoch) is unix nanoseconds drawn once in `Engine::new`;
  K1-store-foundation's `Store::boot()` replaces the source in a later wave (one line in
  `actions/mod.rs`, `page.rs` unchanged).
- task-family-actions (STALE ROWS above, edits proposed for the FLOW owner: the `health`, `task.preview` and `events.subscribe` family rows and the `resync_required` refusal row, which today names only `tools.list` at `/body/page/cursor`): `task.submit`/`task.preview` also run `Brief::check_verify` after `check_restatement` (`invalid_argument` at `/body/brief`, message starts `VERIFY`; admitted though vacuous: the named-not-caught lines `/usr/bin/test -d /usr`, `sh: true && true`, `sh: true # comment`, `/bin/sh -c true`, `sh: printf ok`, `sh: cat /dev/null` of hee4-contracts FLOW, and, not yet in that table, `/usr/bin/env true`, `sh: "true"`, `sh: exit 0;`, `sh: true;`, `/usr/bin/../bin/true` and its doubled-leading-slash form, pinned by `actions::task::tests::vacuous_verify_lines_k0_does_not_catch_yet_are_admitted_by_name`); `health` adds `schema_version` (`Store::schema_version`) and `serve_cgroup` (`Store::serve_cgroup`); `events.subscribe` takes `{since_seq, epoch: string|null}`, acks `{since_seq, epoch, high_water, stream}`, and a string epoch runs `Store::cursor_check` before the ack: `resync_required` `because` (snake_case verdict names) `prior_epoch_of_restore`/`epoch_changed` at `/body/epoch`, `future_sequence` at `/body/since_seq`; `SnapshotOnly` then streams the gap-free tail (DC proposal: K1 documents it as "no replay"); a null epoch is unchecked (DC proposal).
