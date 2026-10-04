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
hee4-sh roster.list 'kinds:=[…]' capability:=null locality:=null include_disabled:=false 'page:={"limit":100,"cursor":null}'   # v4.1
hee4-sh roster.list 'kinds:=[…]' capability:=null locality:=null include_disabled:=true  'page:={"limit":100,"cursor":null}'
```

Socket: request `body` `{kinds[], capability, locality, include_disabled, page}` (FACT required all five); result `body` `{page: PageOutV1<RosterHeadV1>}` (API Map A-11). `UNWRITTEN: RosterHeadV1 fields, the kinds/capability/locality value domains, and the generated wrapper spelling (not spelled before the v4.1 schema exists in K0).`

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
