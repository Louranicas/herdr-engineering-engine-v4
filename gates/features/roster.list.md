# roster.list

A-11. A paged listing of roster records (the agents, models and runtimes the engine may dispatch to) by kind, capability and locality, with or without disabled ones. Scope **v4.1**, phase P9, owner `Owner::Roster`, effect `Read`. Trace E2E-12 names it as the read-back for roster install (`roster.list` count ≤ cap after N synthetic attempts, ATLAS §6 "Roster / observation growth").

## Sub-features

- roster-page: `page: PageOutV1<RosterHeadV1>`, `limit` 1..100.
- kind-capability-locality-filter: `kinds[]`, `capability`, `locality`.
- include-disabled: `include_disabled` toggles records disabled by `roster.disable`.
- retention-visible: the list reflects the per-principal retention cap with latest-per-record kept (card roster §9 #3); the count after growth is bounded.
- v4.0-refusal: at the v4.0 tag the action is catalogued and refused `unavailable` by name with `because` naming v4.1.

## How to get to it (user POV)

`hee4 roster.list` (binary); wrapper spelling "generated" from the K0 schema once it exists; Pi `hee4_roster_list` (PROPOSAL). At v4.0, every invocation reaches the registry miss and returns `unavailable`, retry `after_condition` (Error map class 10). The served path exists from the v4.1 slice at P9.

## Driving it with hee4

Preconditions (v4.0): README shared preconditions. Preconditions (v4.1): a `Read` grant; roster records installed by the deploy installer (E2E-12) under `Owner::Deploy`.

```bash
hee4 roster.list                                   # v4.0: unavailable by name, because=v4.1
hee4-sh roster.list 'kinds:=[…]' capability:=null locality:=null include_disabled:=false 'page:={"limit":100,"cursor":null}'   # v4.1; UNMEASURED: no hee4-sh exists in the six crates
hee4-sh roster.list 'kinds:=[…]' capability:=null locality:=null include_disabled:=true  'page:={"limit":100,"cursor":null}'    # UNMEASURED: no hee4-sh
```

Socket: request `body` `{kinds[], capability, locality, include_disabled, page}` (FACT required all five); result `body` `{page: PageOutV1<RosterHeadV1>}` (API Map A-11).

(rev 2026-10-05 drive) Served from v4.1 by `actions/roster.rs` (`tools/drive.d/roster.py` `d_list`):

```bash
hee4 roster.list --body '{"kinds":[],"capability":null,"locality":null,"include_disabled":false,"page":{"limit":100,"cursor":null}}'
hee4 roster.list --body '{"kinds":["model"],"capability":null,"locality":"local","include_disabled":true,"page":{"limit":100,"cursor":null}}'
```

- Body: all five members required; a missing or mistyped one is `invalid_argument` at its pointer (`/body/kinds`, `/body/capability`, `/body/locality`, `/body/include_disabled`, `/body/page/limit` for a missing `page`). `kinds` is an array of distinct `agent|model|runtime` (empty admits every kind); `capability` is `null` or a string; `locality` is `null`, `"local"` or `"remote"`.
- RosterHeadV1 is `{id, kind, generation, disabled, capability, locality}`; items sorted by `id`, keyset-paged; the cursor `{after_key, boot, filter_sha256}` is pinned to the serve's boot and the filter's digest (`actions/page.rs`).
- Success: the deploy record `model:<HEE4_MODEL>` is listed (kind `model`, generation 1, `disabled:false`); `include_disabled:true` shows a disabled record with `disabled:true`.
- Error: `page.limit` 0 or 101 → `invalid_argument` at `/body/page/limit`; a cursor minted under another boot → `resync_required` at `/body/page/cursor` (`because` "epoch moved"), another filter → "filter moved".
- Empty: `kinds:["runtime"]` on a fresh serve → `items:[]`, `cursor:null`, a `result`.
- Retention (`roster_observations` cap): UNMEASURED, no writer of observations under a record exists in this release (owner: the retention slice).

- v4.0 path (the only reachable one): `unavailable`, `retry=after_condition`, `because` names the scope; assert the `because` string per unserved id (card actions §9 #2).
- v4.1 success: the installed records appear; `include_disabled=false` hides a record after `roster.disable`; `true` shows it with its disabled marker.
- Retention: after N synthetic attempts in the gate, the count stays ≤ the per-principal cap (ATLAS §6).
- Empty: no records → empty page, a `result`.
- Error: page bounds → `invalid_argument`; stale cursor → `resync_required`.
- Side effect: none; `operations` unchanged.

## Gotchas

- Installer rows are **not** `roster.update` operations: they live under the internal `Owner::Deploy` kind, and the `roster.update` idempotency space must be empty after install (UM §3c NF-ROSTER-INSTALL; card roster §9 #4). A list that shows them is right; an `operations` row under `roster.update` after install is the v3 defect (AR SYS HIGH).
- v3 had a global 4096 cap that was never pruned (a suspected blocker after ~4096 attempts). The v4 per-principal cap with latest-per-record kept is the mechanism this action reads back.
- The roster owner trait did not exist in v3 (card actions §10: 2 of the needed owner traits missing); nothing here is REUSE.
- No D-row reads this action; its evidence is the v4.1 slice gate only.
