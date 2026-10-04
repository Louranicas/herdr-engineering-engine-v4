# task.list

A-07. Keyset-paged task heads filtered by state, class and parent, with a snapshot-pinned cursor. Scope v4.0, phase P2 (cursor through the socket at P5, F15), owner `Owner::Task`, effect `Read`. Trace E2E-03; D-rows D9 (`task.list state=accepted` filtered by `serve_cgroup`) and D7 (`acked_present=N/N` after the real-kill step).

## Sub-features

- state-filter: `states[]` of `TaskStateV1` (11 values).
- class-and-parent-filter: `task_class`, `parent_task_id` (nullable).
- keyset-page: `page{limit 1..100, cursor}` in; `page: PageOutV1<TaskHeadV1>` out with a `PageCursorV1{snapshot_revision, after_key, filter_sha256, expires_unix_ms}`.
- snapshot-pinning: a cursor is bound to a snapshot revision and a filter digest; a moved snapshot or epoch is `resync_required`.
- acked-present-line: the D7 read-back `acked_present=N/N` is a `task.list` over the ids the real-kill step submitted (crash-restart.md).
- use-count: the D9 `use` row is a `task.list state=accepted` whose rows have `serve_cgroup` ending in `/hee4.service`, excluding the D7 rehearsal ids.

## How to get to it (user POV)

`hee4 task.list` (binary), `hee4-sh task.list states:=JSON page:=JSON …`, Pi `hee4_task_list` (PROPOSAL). ATLAS D9 phrases it as `task.list state=accepted`.

## Driving it with hee4

Preconditions: README shared preconditions; a `Read` grant.

```bash
hee4 task.list
hee4-sh task.list 'states:=["accepted"]' task_class:=null parent_task_id:=null 'page:={"limit":100,"cursor":null}'
hee4-sh task.list 'states:=["admitted","running"]' task_class:=null parent_task_id:=null 'page:={"limit":2,"cursor":null}'
# then continue with the returned cursor
hee4-sh task.list 'states:=["admitted","running"]' task_class:=null parent_task_id:=null 'page:={"limit":2,"cursor":{…PageCursorV1…}}'
```

Socket: request `body` `{states[], task_class, parent_task_id, page{limit, cursor}}` (FACT required all four); result `body` `{page: PageOutV1<TaskHeadV1>}` (API Map A-07). `UNWRITTEN: the PageOutV1 field names (items, next cursor) and whether serve_cgroup is a TaskHeadV1 field on the wire or only a ledger column; D9 needs one or the other.`

- Success: after E2E-01, the accepted task's head appears under `states=["accepted"]`; paging with `limit=2` over ≥ 3 tasks returns a cursor and the continuation is disjoint and complete.
- Empty: a filter matching nothing returns an empty page and no cursor; still a `result`.
- Error: a cursor whose snapshot or epoch moved → `resync_required` with its own `because` (Error map F-6, the snapshot-revision site); a bad `after_key` → `invalid_argument`; `after_key` over 256 B → `invalid_argument` naming the bound; an expired cursor (`expires_unix_ms` passed) → `resync_required` (the cursor-expiry site).
- Persistence: none written; a cursor obtained before a restart is `resync_required` after it (restored epoch) and the client must restart paging, not resume.
- Side effect: none; `operations` count unchanged.

Concrete, deployed frame (rev 2026-10-05 drive) (run all of it with `tools/drive`):

```bash
hee4 task.list                 # raw body: {}
```

- Body `{tasks:[{task_id, phase}]}`; the task just submitted is listed. The skeleton has no filter or page, so there is no empty variant.
- Persistence: `systemctl --user restart hee4.service`, wait for `hee4 health`, list again; the submitted id is still there. `tools/drive --allow-restart` does this against the live unit; without the flag the path is UNMEASURED.

## Gotchas

- `resync_required` has three causes with three `because` strings (cursor expiry, snapshot revision, preview catalogue revision; Error map F-6). A test asserting the code alone cannot tell them apart; assert the detail.
- After a restore (E2E-06) the epoch is new: every pre-restore cursor is `resync_required`. That is the design, not a regression.
- D9 counts rows by `serve_cgroup`, written at admission. Tasks admitted by a hand-started `serve` or in a disposable HOME are listed but not counted. If `use=0` with accepted tasks present, read the column before blaming the filter.
- The D7 `acked_present=N/N` line is computed from this action's reply against the ids in the rehearsal record; the ids are then excluded from D9.
- `queued` cannot appear in `states` (I-09); a v3 wrapper sending it gets `invalid_argument`.
- Must not: `MAX_PAGE_LIMIT` lives once in K0 bounds; a client-side page cap is a second spelling (UM-P5).
