# Multi-surface journeys

The twelve end-to-end traces (E2E-01…12, home: `16 System Maps/End-to-End Flow Traces.md`) as journeys across the feature files in this directory. Each H2 lists the files it crosses in order, the done-line criterion (ATLAS §1 D1–D10) it evidences, and the phase at which it becomes drivable. Per-action invocations and refusals live in the linked files; read those first, then sequence the journey. README preconditions and the `doctor` procedure apply before every journey.

## E2E-01 · submit → accept (happy path)

Files: `task.submit.md` → (dispatch, execute, verify, accept happen inside the engine) → `task.get.md`. Drivable from P4 in a disposable HOME (F06), through the unit at P7 (F06u).

- Submit a known-good spec through the unit's socket; read `task.get` until `state=accepted`.
- Evidence: D6 (`task.get <id>` → `state=accepted`; `worker_settle` with `identity_sha256`; daemon at the pinned exe with `size_vram > 0`; R0 netns differs), D9 (the row's `serve_cgroup` ends in `/hee4.service`), D3 (the socket answered).
- Side effects: three commits around verification (settle, verdict, accept), one `verifications` row = `verdict_of(decide)`, one outbox row from the accept.

## E2E-02 · submit → verify fail → repair → accept (B16)

Files: `task.submit.md` → `task.get.md` (observe `repair_pending` then `accepted`). Drivable at P5 (F07), through the unit at P7.

- Submit a spec whose first candidate fails verification; `task.get` shows `verifying → repair_pending → running → verifying → accepted`; attempts = 2 of `MAX_ATTEMPTS` 3.
- Evidence: D6 (the fail → repair → verify → accept task in the P7 host record), D8 (F07's L2 row).
- Gotcha: the deadline is one `TASK_LIMIT` across repairs (20 min) with `CLEANUP_RESERVE` 5 min; a repair that starts late is refused by the guard, not the driver (Error map F-9).

## E2E-03 · task.get / task.list observation

Files: `task.get.md`, `task.list.md`. Drivable from P2.

- Read a terminal task by id and by submit key; page `task.list` by state with a cursor; force `resync_required` by paging across a restart.
- Evidence: D6 (`task.get` → `accepted`), D9 (`task.list state=accepted` filtered by `serve_cgroup`).

## E2E-04 · events.subscribe stream

Files: `events.subscribe.md` with `task.submit.md` (producer via accept) or `task.resolve.md` (producer via stop). Drivable from P5.

- Open a stream from `cursor:=null`; in a second shell drive a task to accept; one frame arrives; `outbox.delivered` advances; close and resume from the final cursor.
- Evidence: D8 (the events.subscribe L2 gate with `parked_forever=0/N`).

## E2E-05 · serve start: custody → recovery → backup → accept loop

Files: `health.md`; the start itself is `hee4 serve` (Command Map C-1) and `hee4 commission` (C-3), which have no action file; see `crash-restart.md` § unit restart for the drill. Drivable from P1 (socket, health), P2 (backup).

- Start the unit (or `hee4 serve --until-stdin-closes`); `health` → `ready=true recovery=complete database=ready socket=owned`; a second `serve` is refused by name; the start backup is manifest-last on the other disk.
- Evidence: D3 (socket 0600 in 0700 held by MainPID; the health line), D5(a) (start backup). D4 reads back the separate `commission` verb (exit 0 once, refused the second time; `PRAGMA user_version`; modes).

## E2E-06 · restore drill

Files: `crash-restart.md` § restore from backup → `health.md` → `task.list.md` (post-restore epoch). Drivable at P2 (gate world), P7 (host).

- `hee4 restore --into <disposable> <backup-id>`; the restore line prints both object counts and `rto_s`; `health` → `recovery=complete`; every pre-restore cursor is `resync_required`.
- Evidence: D5(b) (restore line, RTO ≤ 10 min).

## E2E-07 · release install and rollback

Files: none of the 22 (operator verbs `hee4 release install | rollback | prune`, Command Map C-5…C-7); `health.md` is the post-restart read-back. Drivable from P4 (install), P6 (unit), P7 (rehearsal).

- Install a release, restart the unit, read `/proc/<MainPID>/exe`; rollback; restart; read again; prune with a dry run first.
- Evidence: D1 (`readlink -f current`; digests = manifest; `seam_strings=0/0`), D2 (`is-enabled`, `is-active`, exe digest, cgroup ends in `/hee4.service`).

## E2E-08 · shutdown drain and real kill → restart

Files: `task.submit.md` (N acked submits) → `crash-restart.md` § kill -9 mid-task and § unit restart → `task.list.md` (`acked_present=N/N`) → `health.md`. Drivable at P6/P7.

- Evidence: D7 (the 07-recover real-kill step with per-step `rc=`), D2 (`TimeoutStopUSec` = 1,200 s + measured seal time).
- The rehearsal record lists the ids this step submitted; D9 excludes them.

## E2E-09 · task.cancel and task.resolve through the socket (F15)

Files: `task.get.md` (generation) → `task.cancel.md` → `task.get.md` (`cancellation_requested`, then `cancelled`) → `task.resolve.md` (quarantine or abandon on a waiting task) → `task.get.md`. Drivable through the socket at P5.

- Evidence: D8 (the F15 flow's L2).
- Gotcha: `stale_generation` on a stale precondition and `conflict` on a replay with other bytes are the two refusals this journey must show.

## E2E-10 · service observation (v4.2)

Files: `service.probe.md` → `service.inspect.md`. Not drivable before P9; at v4.0 both are `unavailable` by name.

- Evidence: none (post-tag); moves `l2` in P9. Label UNMEASURED(P9).

## E2E-11 · judgment, held

Files: `judge.inspect.md` (the only surface, and only its refusal). The flow itself is an internal `Advised<T>` port behind the `DataClass` check and K0e egress (S-8), held for H-8, H-10, H-11, H-12.

- Evidence: none. The reachable assertion is that no egress occurs and the code decision stands; label UNMEASURED(held) and never send anything to the judge endpoint.

## E2E-12 · roster install (v4.1)

Files: `roster.list.md` (count ≤ cap), `roster.inspect.md` (`last_operation` under `Owner::Deploy`, never `roster.update`), `roster.update.md` (its idempotency space empty after install). Not drivable before P9/v4.1.

- Evidence: none directly; `roster.list` count ≤ cap after N synthetic attempts in the gate (ATLAS §6 "Roster / observation growth").

## Coverage of the done-line by journey

| D-row | Journeys |
|---|---|
| D1, D2 | E2E-07 (E2E-08 reads D2's `TimeoutStopUSec`) |
| D3 | E2E-05, E2E-01 |
| D4 | the `commission` verb beside E2E-05 |
| D5 | E2E-05 (a), E2E-06 (b); (c) is the habitat `hee4-backup.timer`, outside the 22 |
| D6 | E2E-01, E2E-02, E2E-03 |
| D7 | E2E-08 (plus the rest of 07-recover: E2E-06, E2E-07) |
| D8 | E2E-02, E2E-04, E2E-09 |
| D9 | E2E-01, E2E-03 |
| D10 | none: the tag is cut from the aggregate line, not driven through the socket |
