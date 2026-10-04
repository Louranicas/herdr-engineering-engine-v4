# thread.get

A-18. One cohort thread: its task, brief revision, state, obligations, children and artifacts, read against an expected brief revision. Scope **v4.2**, phase P9, owner `Owner::Cohort`, effect `Read`. Tables `cohort_threads`, `cohort_reports`, `dissent_log` (T-12, new in v4). No D-row.

## Sub-features

- thread-detail: `{thread_id, task_id, brief_revision, state, obligations[], children[], artifacts[]}`; children ≤ 100.
- brief-revision-guard: `expected_brief_revision` in the request; a moved revision is `stale_generation`.
- dissent-visible: `UNWRITTEN: whether dissent_log entries appear in this reply or only in cohort_reports; the API Map lists dissent_log under State read and names no reply field.`
- v4.0-refusal: `unavailable` by name (v3 had `thread.get` in its UNSERVED list; card actions §4).

## How to get to it (user POV)

`hee4 thread.get` (binary); wrapper "generated"; Pi `hee4_thread_get` (PROPOSAL). At v4.0, `unavailable` with `because` = v4.2. From v4.2 a caller reaches it from `thread.list` or from a task that spawned a cohort.

## Driving it with hee4

Preconditions (v4.2): a `Read` grant; a `thread_id` and brief revision from `thread.list`.

```bash
hee4 thread.get                                                                   # v4.0: unavailable by name
hee4-sh thread.get thread_id=<id> expected_brief_revision=<r>                     # v4.2
hee4-sh thread.get thread_id=<id> expected_brief_revision=<stale r>               # stale_generation
```

Socket: request `body` `{thread_id, expected_brief_revision}` (FACT required both); result `body` `{thread_id, task_id, brief_revision, state, obligations, children, artifacts}` (API Map A-18). `UNWRITTEN: the thread state enum, obligation and child shapes, and the generated wrapper spelling.`

- v4.0 path: the `unavailable` refusal.
- v4.2 success: the thread for an accepted cohort task shows its children and artifacts; `task.get` on `task_id` agrees on the task state.
- Error: unknown thread → `not_found`; stale `expected_brief_revision` → `stale_generation` + `current_generation`; children over 100 → `resource_exhausted` past the view bound (INTERP from task.get's rule).
- Empty: a thread with no children returns empty arrays.
- Persistence: none written; `dissent_log` is append-only, so a later read never shows fewer entries.
- Side effect: none.

## Gotchas

- `cohort_*` tables are new in v4 (T-12) and nothing in v3 is REUSE here; the owner trait is new (card actions §10).
- `expected_brief_revision` is a precondition in the body, not the envelope's `precondition` field (the schema has `precondition: None` for this action, i.e. optional). Pin the `field` pointer `/body/expected_brief_revision`.
- No D-row; v4.2 slice evidence only.
