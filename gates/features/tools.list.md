# tools.list

A-02. A paged listing of the control catalogue: every action id with its version, purpose and effect, under one `catalogue_revision`. Scope v4.0, phase P1 (INTERP: ATLAS names `health` at P1, not `tools.*`; the phase is catalogue-data §3), owner `Owner::Actions`, effect `Read`. The catalogue is K0 data, content-pinned (UM-P15); the reply is static for a given release.

## Sub-features

- catalogue-page: `page{items{id, version, purpose, effect}}` with `limit` 1..100 and a keyset cursor.
- catalogue-revision: the content digest of the catalogue the engine was built with; the wrapper's `--pin` holds a chain to it.
- query-filter: `query` (≤ 256 bytes, `MAX_QUERY_BYTES`) or `null`.
- held-ids-visible: `judge.inspect` appears in the catalogue without a registry entry (card catalogue-data §10: a held id is carried, not served); unserved-in-scope ids (`roster.*`, `service.*`, …) appear too.
- hidden-ids-absent: an action the catalogue hides reads as absent, and `tools.inspect` on it is `unknown_action` (Error map class 5).

## How to get to it (user POV)

`hee4 tools.list` (binary), `hee4-sh tools.list query:=null page:=JSON` (wrapper), or the Pi tool `hee4_tools_list` (PROPOSAL Command Map P-1; v3 `habitat_tools_list`). Also `hee4-sh --actions`, which lists what the *wrapper* can name from its generated catalogue, which must equal this action's items for the same release.

## Driving it with hee4

Preconditions: README shared preconditions; a `Read` grant.

```bash
hee4 tools.list
hee4-sh tools.list query:=null 'page:={"limit":100,"cursor":null}'
hee4-sh tools.list query=task 'page:={"limit":10,"cursor":null}'
hee4-sh --actions
```

Socket: request `body` `{query, page{limit, cursor}}`; result `body` `{catalogue_revision, page{items[{id, version, purpose, effect}], cursor}}` (API Map A-02).

- Success: 22 ids at v4.0 (21 v3 ids + `judge.inspect`), each with `effect` from the K0 enum; `catalogue_revision` equals the digest the gate's catalogue test pins (card actions §9 #3) and the wrapper's `--pin`.
- Empty: `query` that matches nothing returns an empty `items` and no cursor; the reply is still a `result`, not an error.
- Error: `query` over 256 bytes → `invalid_argument` at `/body/query` naming the bound; `page.limit` 0 or over the bound → `invalid_argument` at `/body/page/limit`.
- Persistence: none to verify; repeat yields identical `catalogue_revision` for the same binary, and a different one after `hee4 release install` of a different release.
- Side effect: no `operations` row.
- `UNWRITTEN: which page bound governs tools.list: v3 MAX_PAGE = 32 (actions.rs) or MAX_PAGE_LIMIT = 100 (contracts/control.rs). API Map P-3 says v4 keeps one and the slice records which; assert MAX and MAX+1 once it is chosen.`
- `UNWRITTEN: the cursor type for this page (PageCursorV1 as task.list, or a catalogue-specific one).`

## Gotchas

- The catalogue is served from K0 data and never from a registry walk; an id being listed says nothing about it being *served*. `unavailable` by name on invocation is the scope signal (Error map class 10).
- Two v3 catalogues (JSON schema vs `CATALOGUE`) were pinned by count only (card actions §4); v4 pins by content. Count equality is not evidence; compare digests.
- Pi projection conflict (API Map Conflicts §1): card pi-extension §8 says Pi serves "health, tools.*, task.*", but the v3 catalogue gives `health`, `task.resolve`, `roster.update`, `roster.disable`, `events.subscribe` no tool. If you drive this through Pi and see 7 tools rather than 9, that is the recorded conflict, not a regression.
- `effect` values on the wire are the K0 `Effect` enum; v3 `queued`-era spellings do not exist in v4 (I-09 by construction).
- Must not: the wrapper and Pi keep no list of their own (Command Map generation rule); if `--actions` and `tools.list` disagree, the generator is broken, not the catalogue.
