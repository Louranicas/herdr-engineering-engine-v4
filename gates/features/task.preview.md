# task.preview

A-04. Read-only planning: for a `TaskSpecV1` at a given brief and catalogue revision, which recipes are eligible, which are excluded and why, the cost mode, and the observation cutoff the answer rests on. Scope v4.0, phase P2 (INTERP: routing §3), owner `Owner::Task`, effect `ReadOnlyPlanning` (not mutating). No E2E trace and no D-row names it.

## Sub-features

- eligibility: `eligible` recipes for the spec under the routing config and roster eligibility (K2 `route` over `roster_records` / `roster_observations`).
- exclusions-with-codes: `exclusions[{recipe_id, code}]`, bounded at 128 entries each side (CD RC03 §4).
- cost-mode: `cost_mode` for the spec's budget mode.
- observations-cutoff: `observations_cutoff_unix_ms`, the time the eligibility read is good for.
- revision-pinning: `brief_revision` and `catalogue_revision` in the request; a stale one is refused rather than silently re-planned.
- rc01-profile: under the RC01 profile a spec with `remote_allowed` or non-zero currency is refused (`unavailable`/`forbidden`, CD RC03 §4 last paragraph).

## How to get to it (user POV)

`hee4 task.preview` (binary), `hee4-sh task.preview spec:=JSON brief_revision=… catalogue_revision=…`, Pi `hee4_task_preview` (PROPOSAL). A caller previews before `task.submit` to learn whether a spec would dispatch and under what cost mode; nothing is written.

## Driving it with hee4

Preconditions: README shared preconditions; a `ReadOnlyPlanning` grant; a `catalogue_revision` from `tools.list`; a `brief_revision` (`UNWRITTEN: where a caller obtains brief_revision; no v4.0 action returns it`).

```bash
hee4 task.preview
hee4-sh task.preview 'spec:={"task_class":"…","intent":"…","criteria":[…],"privacy":"…","workspace_id":"…","budget":{"mode":"…","wall_ms":"…","tokens":"…","currency_microunits":"0"},"parent":null}' brief_revision=<r> catalogue_revision=<digest>
hee4-sh --check task.preview spec:=… brief_revision=… catalogue_revision=…
```

Socket: request `body` `{spec: TaskSpecV1, brief_revision, catalogue_revision}`; result `body` `{eligible[], exclusions[{recipe_id, code}], cost_mode, observations_cutoff_unix_ms}` (API Map A-04). `TaskSpecV1` required fields: `task_class, intent, criteria, privacy, workspace_id, budget{mode, wall_ms, tokens, currency_microunits}, parent` (FACT v3 schema; `UNWRITTEN: the value domains of task_class, privacy, budget.mode and the criteria element shape in v4`).

- Success: `eligible` non-empty for the gate's known-good spec; the same spec then submits (task.submit.md) and dispatches.
- Empty: a spec no recipe can serve returns `eligible=[]` with an `exclusions` entry per recipe and its code; still a `result` frame.
- Error: a `catalogue_revision` not equal to the engine's → refused (see Gotchas for the code); `remote_allowed` or `currency_microunits` ≠ 0 under RC01 → `unavailable` or `forbidden` with its own `because` (Error map F-4: preview's "not admitted by RC01" has its own detail).
- Persistence: none; no task row, no operations row (ReadOnlyPlanning, `fn mutates` false). Assert `tasks` count unchanged.
- Side effect: none on disk; reads `roster_observations` up to the cutoff.

## Gotchas

- Two codes for a stale revision: API Map A-04 says `stale_generation` (INTERP); Error map class 4 / F-6 says `resync_required` from the `task.preview` arm of `fn dispatch` (v3 FACT) with its own `because`. `UNWRITTEN: which code v4 raises for a stale catalogue_revision in preview.` Pin the `field`, not the code, until decided.
- A preview is advice, not a reservation: eligibility can change before `task.submit` (observations move; the cutoff says how old the answer is). Do not assert submit success from preview success.
- v3 also raised `unavailable` for "child allocations" in preview (Error map class 10 note): two meanings, one code. v4 separates them by `because`; assert the string.
- Must not: `main` holds no routing policy; `compose_native`-style policy in the binary is the v3 defect (card main §7 AP-08).
- Not in any E2E trace, and ATLAS names no preview D-row (E2E "Not placed"): evidence for this action comes only from the P2 gate.
