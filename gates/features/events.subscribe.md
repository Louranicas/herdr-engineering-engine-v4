# events.subscribe

A-10. The only streaming action: a subscriber gives a cursor and topics, receives a bootstrap frame, then one `ControlEventV1` frame per outbox row until it is caught up, parks on the publish wake, and continues; each delivered frame advances `outbox.delivered` through `acknowledge_delivery`, its first production caller (UM-P11). Scope v4.0, phase **P5**, on the done-line, owner `Owner::Notify`, effect `ReadStream` (not an operation; the ack is a delivery write). Trace E2E-04; D-row D8 (the L2 gate test with `parked_forever=0/N`). The stream design has one home: Workflow and Loop Map § RL-5.

## Sub-features

- bootstrap-frame: first frame `{subscription_id, snapshot, high_water_cursor}`, bounded by `bootstrap_limit` 1..256.
- event-frames: `ControlEventV1` per outbox row after the cursor, on `topics` ⊆ {task, thread, roster, service, analysis, delivery, recovery} (1..7), optionally narrowed by `resource_ids` ≤ 100.
- acknowledge-on-write: each written frame acknowledges delivery (`outbox.delivered`); `task.get.delivery` then reads a real count.
- park-and-wake: caught up → park; woken by `runtime::publish` after a K1 commit (accept or terminal stop, the one outbox fn), by RL-10 shutdown, by the 60 s idle budget, or by the cursor's `expires_unix_ms`.
- status-frames: `StreamStatusV1` on `resync_required`, `queue_limit` (256 frames or 8 MiB behind) and `server_draining`, then close.
- resume-cursor: idle or expiry closes with a final frame carrying the resume cursor (or `CursorExpired`).
- write-budget: 10 s per frame; a stalled reader is closed.
- one-connection-each: a stream holds one of the 8 connections; no per-principal sub-cap in v4.0 (DC-31, V4-65).

## How to get to it (user POV)

`hee4 events.subscribe` (binary; prints frames until close), `hee4-sh events.subscribe cursor:=null topics:=JSON resource_ids:=JSON bootstrap_limit:=…`. No Pi tool (v3 `tool: None`). A caller usually starts from the `engine_cursor` a `task.submit` reply carried, or from `null` for a bootstrap. (UNMEASURED: hee4-sh exists in no crate)

Not reachable before P5: the `Reply::Stream` arm does not exist until then; at P1–P4 the action is catalogued and refused `unavailable` by name.

## Driving it with hee4

Concrete, deployed frame (rev 2026-10-05 drive) (run all of it with `tools/drive`):

```bash
hee4 events.subscribe --body '{"since_seq":0,"epoch":null}'
# raw: {"request_id":"r","action":"events.subscribe","action_version":1,"idempotency_key":null,"body":{"since_seq":0,"epoch":"<epoch>"}}
# ack: {"kind":"result",…,"body":{"since_seq":0,"epoch":"<epoch>","high_water":<n>,"stream":"events"}}
# then: {"kind":"event","seq":<n>,"task_id":"t-…","event":…,"phase_after":"admitted","ts":…}
```

The deployed body is `{since_seq: u64|null, epoch: string|null}` (not `EventCursorV1`; the UNWRITTEN stream shape below is resolved by the frames above). Paths (`d_subscribe`; the stream helper keeps one socket open, every read times out at 10 s or less, and it prints `elapsed_s=`):

- `ack`: `stream` `events`, `epoch` a non-empty string, `high_water` an int ≥ 0, `since_seq` 0.
- `replay`: since_seq 0 with the epoch, frames until `seq ≥ high_water`, each `kind`, `seq`, `task_id`, `event`, `phase_after`, `ts`, `seq` strictly ascending; `replay_submit_admitted` (task.submit's task first shows `admitted`); `replay_resolve_producer` (task.resolve's cancelled task shows `cancelled`).
- `live_follow`: open at `high_water`, submit a fresh brief, its `admitted` frame arrives within 10 s.
- `resume_exactly_once`: since_seq = the last seq seen; no frame at or below it within 1 s.
- `bad_since_seq`: `-1` → `invalid_argument` at `/body/since_seq`.
- `resync_wrong_epoch`: epoch `not-the-epoch` → `resync_required` at `/body/epoch`.
- `resync_future_seq`: `high_water + 1000` → `resync_required` at `/body/since_seq`.
- `slot_freed`: `health` answers a result after every stream is closed.

Preconditions: README shared preconditions; a `ReadStream` grant; at least one task that will reach accept or stop during the run (so the outbox gains a row).

```bash
hee4 events.subscribe
hee4-sh events.subscribe cursor:=null 'topics:=["task","delivery"]' 'resource_ids:=[]' bootstrap_limit:=256  # UNMEASURED: hee4-sh exists in no crate
# in another shell, while the stream is parked:
hee4-sh task.submit 'spec:={…}' @idempotency_key=$(uuidgen)      # then let it accept, or task.cancel + Stop  # UNMEASURED: hee4-sh exists in no crate
# resume from the final frame's cursor after an idle close:
hee4-sh events.subscribe 'cursor:={"epoch":"…","sequence":"…","filter_sha256":"…","visibility_revision":"…","issued_unix_ms":"…","expires_unix_ms":"…"}' 'topics:=["task"]' 'resource_ids:=[]' bootstrap_limit:=1  # UNMEASURED: hee4-sh exists in no crate
```

Socket: request `body` `{cursor: EventCursorV1|null, topics[], resource_ids[], bootstrap_limit}` (FACT required all four; `EventCursorV1{epoch, sequence, filter_sha256, visibility_revision, issued_unix_ms, expires_unix_ms}`); reply **Stream**: bootstrap frame, then `ControlEventV1` frames, `StreamStatusV1` on the three conditions (API Map A-10). The handshake obeys the 60 s deadline; after it the stream follows cursor and queue policy, not the envelope deadline (CD RC03 §6). `RESOLVED (rev 2026-10-05 drive: the deployed stream frame is {kind, seq, task_id, event, phase_after, ts}), was UNWRITTEN: the ControlEventV1 and StreamStatusV1 field lists and the stream wire shape (card contracts §10: undecided).`

- Success: bootstrap frame, then after an accept in the other shell exactly one event frame for it; host `mode=ro` shows `outbox.delivered` advanced for that row and `task.get.delivery=1`.
- Parked-subscriber test (ATLAS P5): N subscribers park before the outbox fold, the fold lands, every one is woken: `parked_forever=0/N`.
- Cancel/close: client closes → the server drops the stream and frees the connection slot (assert a ninth connection is admitted afterwards). SIGTERM on `serve` → `server_draining` frame then close (RL-10 wakes every parked subscriber).
- Error: stale cursor (expired, restored epoch, missing continuity) → `resync_required` with its own `because`; a subscriber > 256 frames / 8 MiB behind → `queue_limit` and close; `topics` empty or > 7, `resource_ids` > 100, `bootstrap_limit` 0 or > 256 → `invalid_argument` naming the bound.
- Empty: no rows after the cursor → bootstrap frame only, then park; the idle budget closes it with a resume cursor after 60 s.
- Persistence: `delivered` survives restart; resuming with the final cursor after an idle close replays nothing already acknowledged; after a restore (new epoch) every old cursor is `resync_required`.
- Side effects to read: `outbox.delivered`, connection count, `task.get.delivery`.

Concrete, deployed frame (`crates/hee4-app/FLOW.md` Stream; `crates/hee4-app/tests/e2e.rs`; no drive procedure yet, so no drive marker):

```bash
printf '%s\n' '{"request_id":"s1","action":"events.subscribe","action_version":1,"idempotency_key":null,"body":{"since_seq":0,"epoch":null}}' | socat - UNIX-CONNECT:$XDG_RUNTIME_DIR/hee4/control.sock
```

- Request `body` `{since_seq: u64|null, epoch: string|null}`: `since_seq` absent or null is 0 (`invalid_argument` at `/body/since_seq` otherwise); `epoch` absent or null is a legacy cursor; any other type is `invalid_argument` at `/body/epoch` ("string or null"). The six-field `EventCursorV1` above is not adopted: `{since_seq, epoch}` is the minimal cursor (DC proposal).
- Ack: `kind=result`, `replayed=false`, body `{since_seq, epoch, high_water, stream:"events"}`; `epoch` is the ledger's (`Store::epoch`), `high_water` the highest `events.seq` or 0 (`Store::event_high_water`). Then one event frame per ledger row with `seq > since_seq`, in `seq` order: `{"kind":"event","seq","task_id","event","phase_after","ts"}`; `event` is the contracts' `Serialize` spelling, `phase_after` the replayed phase after that row. Resume with `since_seq` = the last `seq` received: exactly once by `seq`.
- `resync_required` (R13, `Store::cursor_check`, retry `never`, before the ack, never a replay authorisation), only when `epoch` is a string, in this order: `because="prior_epoch_of_restore"` at `/body/epoch` (the epoch is the one this ledger was restored from, `meta.restored_from`); `because="epoch_changed"` at `/body/epoch` (any other epoch than the ledger's; the message names the ledger epoch); `because="future_sequence"` at `/body/since_seq` (`since_seq > high_water`; the message names the high water). `since_seq == high_water` with the right epoch is the snapshot boundary: an ack, then only new rows.
- Epoch-null rule: a null epoch is unchecked (the skeleton's legacy cursor) and streams as before; the ack still returns `epoch` and `high_water` so the next subscribe can carry them. Proposal (DC): a later release refuses a null epoch with `invalid_argument` once every client sends one.
- `slow_consumer`: a subscriber 256 frames behind is dropped: `{"kind":"close","code":"slow_consumer","retry":"after_condition",...}` then close, written with a 200 ms deadline; when the peer's buffer is full the frame is not delivered, the server logs `slow_consumer close frame not delivered` and the client sees EOF. Resubscribe with the last `seq` and the ack's `epoch`.
- Restart: the epoch survives a restart of the same ledger; only a restore mints a new one, so a pre-restart cursor with its epoch resumes, and a pre-restore cursor is `prior_epoch_of_restore`.

## Gotchas

- It is `ReadStream`: not an operation, so no `operations` row and no idempotency key — but it **does** write (`delivered`). "Read-only" is wrong for it.
- A lost wake is the v3 W6 failure (31/288 parked forever). Only the parked-subscriber line is evidence; a single subscriber that happened to be mid-write proves nothing.
- The 60 s idle close is the reused v3 budget, not a new one; a stream that closes after a quiet minute with a resume cursor is correct. Resume, do not report.
- `queue_limit` is a correlated status frame, then close; it is not an error frame with a `retry`.
- Two outbox insert sites in v3 (accept and terminal stop) become one fn; both an accept and a `task.resolve abandon` must produce a frame. Test both producers.
- The 8-connection cap counts streams; a sweep that leaves 8 streams open makes every other action `resource_exhausted` (Error map PROPOSAL P-1 shape).
- No pre-P5 evidence exists for this file; before then the reachable path is the `unavailable` refusal.
- Must not: no second outbox or notify-side queue (v3 `notify::Outbox` dropped, UM §6a); the stream budgets live only in RL-5 (DC-31).
