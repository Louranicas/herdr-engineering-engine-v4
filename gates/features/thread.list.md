# thread.list

A-19. Paged thread heads under a task, filtered by state. Scope **v4.2**, phase P9, owner `Owner::Cohort`, effect `Read`. No D-row.

## Sub-features

- thread-page: `page: PageOutV1<thread head>`, `limit` 1..100.
- task-scope: `task_id` names the parent task.
- state-filter: `states[]` ≤ 6.
- v4.0-refusal: `unavailable` by name (v3 UNSERVED `thread.list`).

## How to get to it (user POV)

`hee4 thread.list` (binary); wrapper "generated"; Pi `hee4_thread_list` (PROPOSAL). At v4.0, `unavailable`. From v4.2 a caller lists the threads of a cohort task before `thread.get`.

## Driving it with hee4

Preconditions (v4.2): a `Read` grant; a cohort task id.

```bash
hee4 thread.list                                                                                     # v4.0: unavailable by name
hee4-sh thread.list task_id=<id> 'states:=[…]' 'page:={"limit":100,"cursor":null}'                   # v4.2
hee4-sh thread.list task_id=<id> 'states:=[…]' 'page:={"limit":2,"cursor":{…PageCursorV1…}}'         # continuation
```

Socket: request `body` `{task_id, states[], page}` (FACT required all three); result `body` `{page: PageOutV1<thread head>}` (API Map A-19). `UNWRITTEN: the thread head fields, the thread state enum (≤ 6 states), and the generated wrapper spelling.`

- v4.0 path: the `unavailable` refusal.
- v4.2 success: every thread of the task appears across pages; continuation is disjoint and complete.
- Error: stale cursor → `resync_required`; `states` over 6 or page bounds → `invalid_argument`.
- Empty: a task with no threads → empty page.
- Persistence: none; a cursor from before a restart is `resync_required`.
- Side effect: none.

## Gotchas

- Same three-cause `resync_required` as task.list.md: assert the `because`.
- No D-row; v4.2 slice evidence only. The owner trait and tables are new in v4.
