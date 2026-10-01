# HEE v4 module funnel

**Status: PLANNING — HOLD** (V4-0, `plan/DECISIONS.md:4`). No code until Luke says "start coding". There are no `.rs`/`.py` stubs: **each card is the stub.**
Written 2026-10-01 by louranicas-2c plus six same-lineage card writers. Reviewed once by V5 (`~/hee4-evidence/verification/V5-module-cards.md`, 2026-10-01, same model family, not cross-lineage): PASS_WITH_GAPS, 0 HIGH · 3 MED · 9 LOW. What was fixed is listed under "Funnel integrity" below.

## What this is
Each v4 module has one card, `modules/<crate>/<module>/MODULE.md`, and one row in the machine-readable `modules/MODULES.toml`. A card collects everything the planning corpus says about its module, and each fact carries a `KEY:line` cite into a v4-owned file:
- identity;
- purpose, owned state and allowed dependencies;
- ATLAS phases and held-for-Luke items;
- the v3 basis, with ERRATA applied;
- decision points and Jev dispositions;
- migrated inputs;
- AP/EX/A/D guards and interfaces;
- measurable done criteria;
- open decisions;
- copy-paste pull commands.

Cards cite **v4 homes only** (V4-9). A v3 fact is quoted through `~/hee4-evidence/reference/v3-evidence-b5367bc/` together with `ERRATA-v3-evidence-b5367bc.md`.

**Source keys** used in the cards. `KEY:n` is a *line number* in the file (not a table row number), and `ops/checks/module_funnel.py` resolves every one through the same legend (`LEGEND` in the script); a key missing from it is reported by name.

| Key | File | Key | File |
|---|---|---|---|
| UM | `~/hee4-evidence/design/ULTRAMAP.md` | AT | `~/hee4-evidence/design/DEPLOYMENT_ATLAS.md` |
| DP | `~/hee4-evidence/design/DECISION_POINTS-b5367bc.md` | JQ | `~/hee4-evidence/design/JEV_QUESTION_RESPONSES.md` |
| PL | `~/hee4-evidence/learnings/PROCESS-LEARNINGS.md` | ER | `~/hee4-evidence/reference/ERRATA-v3-evidence-b5367bc.md` |
| MA | `…/v3-evidence-b5367bc/module-audit-b5367bc.md` | AR | `…/v3-evidence-b5367bc/architecture-review-b5367bc.md` |
| CMAP | `…/v3-evidence-b5367bc/cluster-map-b5367bc.md` | IM | `…/v3-evidence-b5367bc/interface-map-b5367bc.md` |
| JM | `…/v3-evidence-b5367bc/jev-decision-map-b5367bc.md` | RT | `…/v3-evidence-b5367bc/route-to-v010-b5367bc.md` |
| RR | `…/v3-evidence-b5367bc/review-range-7ed3728-dfb32d5.md` | V1-V5 | `~/hee4-evidence/verification/V{1-repo,2-design,3-evidence,4-records,5-module-cards}.md` |
| DEC | `plan/DECISIONS.md` | CH | `CHARTER.md` |
| REQ | `gates/REQUIREMENTS.md` | MIG | `migrated/v3-b5367bc/MIGRATION.md` |
| EXX | `docs/EXEMPLARS.md` | APX | `docs/ANTIPATTERNS.md` |

`…` is `~/hee4-evidence/reference`. Repo paths are relative to `~/herdr-engineering-engine-v4`. `APX AP-nn row`, `REQ rank n` and `CH §n` are row/section locators, not line cites. Ids: AP-nn (`docs/ANTIPATTERNS.md`), EX-nn and A-n (`docs/EXEMPLARS.md`), D-nn (`docs/DRIFT_AND_OVERENGINEERING.md`), J1-J14 (JM), E1-E17 (ER), H-n (the H-list, AT §5; no range quoted *(rev 2026-10-01 V7/V8/V9/V10-fix, V7 F08)*), V4-n and D-Un (DEC).

## How to use it
```bash
cd ~/herdr-engineering-engine-v4
python3 -c "import tomllib;[print(m['crate'],m['name'],m['phases']) for m in tomllib.load(open('modules/MODULES.toml','rb'))['module']]"
# Modules a phase builds (here P3):
python3 -c "import tomllib;[print(m['card']) for m in tomllib.load(open('modules/MODULES.toml','rb'))['module'] if 'P3' in m['phases']]"
# Which cards cite a given anti-pattern:
grep -l 'AP-02' modules/*/*/MODULE.md
# Then open the card and run its §11 pull block.
```

## Before touching a module (checklist)
1. [ ] Luke has said "start coding" (H-5). Until then, only this planning work is allowed.
2. [ ] Read the card top to bottom, then run its §11 pull commands. Re-read every cited `KEY:line` in its source, because line numbers drift.
3. [ ] Check §4 against `ERRATA-v3-evidence-b5367bc.md`. No v3 claim is used without its E-id.
4. [ ] Read every AP/EX/A in §7 in `docs/ANTIPATTERNS.md` / `docs/EXEMPLARS.md`, and D-01…D-16 (`CLAUDE.md` "Before touching a module").
5. [ ] Confirm the phase entry gate in §3 is met, and that no H-item in §3 blocks this step.
6. [ ] Confirm the crate's `[dependencies]` match `depends_on` (strict DAG, UM:60-66). Only `hee4-app` may depend on `hee4-egress`. *(rev 2026-10-01 open-tasks CN-15)* `depends_on` is planning-only: it is retired at P0, when `Cargo.toml` becomes the one home (V4-17). A module never lists its own crate (`main` did, V7 F27; now `[]`).
7. [ ] HARDEN modules: bring files over from `migrated/v3-b5367bc/`. In the same step, strip the anchor blocks, rename to the `hee4` namespace (V4-11) and fix the `tests/t09_route.rs:2352` split (V4-10).
8. [ ] Write the one-page FLOW and a compiling skeleton before any review, with at most 2 design rounds (D-02, D-03). Name the l2 delta (D-01).
9. [ ] Carry §9 done criteria into the slice as read-backs. Plants run under `--cap-lints=warn` and must name the killing test. Scoped `cargo mutants` runs with `CARGO_TARGET_DIR` unset.
10. [ ] Settle §10 open decisions, or record them in `plan/DECISIONS.md`, before the slice lands.

## One home for module data: `MODULES.toml` *(rev 2026-10-01 open-tasks CN-09)*
`modules/MODULES.toml` is the **single home** of module membership, phases, H-ids and dependencies. Any other table of that data is either **checked** against it or **generated** from it, never typed by hand:
- the "Module → crate table" below is a checked copy: `module_funnel.py` fails `readme_row_mismatch` / `readme_row_count` when its card set or phases differ from the manifest;
- the vault's copies (Module Design Index rows, System Schematic "Module membership per crate", Workflow and Loop Map §3.1 module × phase matrix) are to become links here plus the output of the commands below, pasted under a line naming the command (CHARTER §5: no generated `BEGIN…END` blocks). Editing the vault is the vault batch's work, not this file's.

Regenerate, from the repo root:
```bash
# Membership per crate (System Schematic table)
python3 -c "import tomllib;m=tomllib.load(open('modules/MODULES.toml','rb'))['module'];g={};[g.setdefault((x['cluster'],x['crate']),[]).append(x['name']) for x in m];print('| Cluster | Crate | Modules |');print('|---|---|---|');[print('| %s | %s | %s (%d) |'%(k[0],k[1],', '.join(v),len(v))) for k,v in g.items()]"
# Module x phase matrix with H-ids (Workflow and Loop Map 3.1)
python3 -c "import tomllib;m=tomllib.load(open('modules/MODULES.toml','rb'))['module'];P=['P%d'%i for i in range(10)];print('| K | Module | '+' | '.join(P)+' | H-ids |');print('|---'*(len(P)+3)+'|');[print('| %s | %s/%s | '%(x['cluster'],x['crate'],x['name'])+' | '.join('●' if p in x['phases'] else '' for p in P)+' | '+(', '.join(x['h']) or '—')+' |') for x in m]"
```

## Module → crate table (53 cards)
| Crate | Cluster | Card | v3 origin | MA flag | AR rec | Phases |
|---|---|---|---|---|---|---|
| hee4-contracts | K0 | [contracts](hee4-contracts/contracts/MODULE.md) | contracts | PARTIAL | HARDEN | P0, P1, P5, P6 |
| hee4-contracts | K0 | [state-enums](hee4-contracts/state-enums/MODULE.md) | recovery, store, contracts | PARTIAL | KEEP / REFACTOR | P0, P1 |
| hee4-contracts | K0 | [catalogue-data](hee4-contracts/catalogue-data/MODULE.md) | actions, worker | PARTIAL | HARDEN / REFACTOR | P0, P1, P5, P9 |
| hee4-contracts | K0 | [judge-types](hee4-contracts/judge-types/MODULE.md) | — (new) | n/a | n/a | P0, P9 |
| hee4-contracts | K0 | [bounds](hee4-contracts/bounds/MODULE.md) | contracts, worker, store, app, bash | PARTIAL | HARDEN / REFACTOR | P0, P1, P3 |
| hee4-host | K0h | [spawn](hee4-host/spawn/MODULE.md) | worker | FINISHED (unverified, E §2) | REFACTOR / REUSE lifecycle | P0, P3, P4 |
| hee4-host | K0h | [clients](hee4-host/clients/MODULE.md) | worker | FINISHED (unverified) | REFACTOR | P3, P4, P7 |
| hee4-host | K0h | [clock](hee4-host/clock/MODULE.md) | app, task | PARTIAL | REFACTOR | P1, P2, P3 |
| hee4-host | K0h | [cgroup-io](hee4-host/cgroup-io/MODULE.md) | worker | FINISHED (unverified) | REFACTOR | P3, P4, P6 |
| hee4-egress | K0e | [judge-egress](hee4-egress/judge-egress/MODULE.md) | — (new) | n/a | n/a | P0, P9 |
| hee4-core | K1 | [task](hee4-core/task/MODULE.md) | task | PARTIAL | HARDEN | P1, P2, P5 |
| hee4-core | K1 | [store](hee4-core/store/MODULE.md) | store | PARTIAL | REFACTOR | P1, P2, P5, P6 |
| hee4-core | K1 | [budget](hee4-core/budget/MODULE.md) | budget | NOT FINISHED | REFACTOR | P3, P9 |
| hee4-core | K1 | [recovery](hee4-core/recovery/MODULE.md) | recovery | PARTIAL | KEEP | P1, P2, P7 |
| hee4-worker | K2 | [roster](hee4-worker/roster/MODULE.md) | roster | PARTIAL | HARDEN | P2, P3, P9 |
| hee4-worker | K2 | [route](hee4-worker/route/MODULE.md) | route | PARTIAL | HARDEN | P1, P9 |
| hee4-worker | K2 | [native](hee4-worker/native/MODULE.md) | worker | FINISHED | REFACTOR | P3, P4, P7 |
| hee4-worker | K2 | [namespace](hee4-worker/namespace/MODULE.md) (+ `hee4-namespace-shim`) | worker | FINISHED | REFACTOR | P3, P4, P9 |
| hee4-worker | K2 | [aggregate-policy](hee4-worker/aggregate-policy/MODULE.md) | worker | FINISHED | REFACTOR | P3, P4 |
| hee4-worker | K2 | [inference](hee4-worker/inference/MODULE.md) | worker | FINISHED | REFACTOR | P3 |
| hee4-worker | K2 | [pi-adapter](hee4-worker/pi-adapter/MODULE.md) | worker, pi_extension | NOT FINISHED | REFACTOR | P9 |
| hee4-cohesion | K3 | [cohort](hee4-cohesion/cohort/MODULE.md) | cohort | NOT FINISHED | REFACTOR | P9 |
| hee4-cohesion | K3 | [context](hee4-cohesion/context/MODULE.md) | context | PARTIAL | HARDEN | P9 |
| hee4-cohesion | K3 | [notify](hee4-cohesion/notify/MODULE.md) | notify | NOT FINISHED | REFACTOR | P5, P9 |
| hee4-cohesion | K3 | [skills](hee4-cohesion/skills/MODULE.md) | skills | PARTIAL | HARDEN | P9 |
| hee4-cohesion | K3 | [workflows](hee4-cohesion/workflows/MODULE.md) | workflows | PARTIAL | REFACTOR | P9 |
| hee4-evidence | K4 | [check](hee4-evidence/check/MODULE.md) | check, app/u64_receipt | PARTIAL | REFACTOR | P3, P5 |
| hee4-evidence | K4 | [numerical](hee4-evidence/numerical/MODULE.md) | numerical | NOT FINISHED | HARDEN | P9 |
| hee4-evidence | K4 | [julia-decoders](hee4-evidence/julia-decoders/MODULE.md) | julia | NOT FINISHED | REFACTOR | P9 |
| hee4-evidence | K4 | [judge-admission](hee4-evidence/judge-admission/MODULE.md) | — (new) | n/a | n/a | P9 |
| hee4-habitat | K5 | [service](hee4-habitat/service/MODULE.md) | service | NOT FINISHED | REFACTOR | P9 |
| hee4-habitat | K5 | [herdr](hee4-habitat/herdr/MODULE.md) | herdr | PARTIAL | HARDEN | P1, P9 |
| hee4-app | K6 | [runtime](hee4-app/runtime/MODULE.md) | app | PARTIAL | REFACTOR | P1, P3, P5 |
| hee4-app | K6 | [dispatcher](hee4-app/dispatcher/MODULE.md) | app | PARTIAL | REFACTOR | P2, P3, P5, P9 |
| hee4-app | K6 | [control-socket](hee4-app/control-socket/MODULE.md) | app | PARTIAL | REFACTOR (custody REUSE) | P1, P5, P6 |
| hee4-app | K6 | [actions](hee4-app/actions/MODULE.md) | actions | PARTIAL | HARDEN | P1, P2, P5, P9 |
| hee4-app | K6 | [tasks](hee4-app/tasks/MODULE.md) | app | PARTIAL | REFACTOR | P2, P5, P7 |
| hee4-app | K6 | [startup-coordinator](hee4-app/startup-coordinator/MODULE.md) | app | PARTIAL | REFACTOR | P2, P6, P7 |
| hee4-app | K6 | [backup-target](hee4-app/backup-target/MODULE.md) | app | PARTIAL | REFACTOR (EX-11 kept) | P2, P6, P7 |
| hee4-app | K6 | [main](hee4-app/main/MODULE.md) | app | PARTIAL | REFACTOR | P0, P1, P4, P6 |
| hee4-app | K6 | [class-profile](hee4-app/class-profile/MODULE.md) | app | PARTIAL | REFACTOR | P3, P4 |
| hee4-app | K6 | [native-provider](hee4-app/native-provider/MODULE.md) | app | PARTIAL | REFACTOR | P3, P4, P7 |
| hee4-app | K6 | [live-verifier-adapter](hee4-app/live-verifier-adapter/MODULE.md) | app, check | PARTIAL | REFACTOR | P3 |
| hee4-app | K6 | [candidates](hee4-app/candidates/MODULE.md) | app | PARTIAL | REFACTOR | P3, P5, P9 |
| hee4-app | K6 | [workload](hee4-app/workload/MODULE.md) | app | PARTIAL | REFACTOR | P3, P5, P9 |
| hee4-app | K6 | [repair](hee4-app/repair/MODULE.md) | app | PARTIAL | REFACTOR | P5, P7 |
| hee4-app | K6 | [plan](hee4-app/plan/MODULE.md) | app | PARTIAL | REFACTOR | P3, P4 |
| hee4-app | K6 | [routing](hee4-app/routing/MODULE.md) | app, route | PARTIAL | REFACTOR | P1, P2, P9 |
| hee4-app | K6 | [u64-class](hee4-app/u64-class/MODULE.md) | app, check | PARTIAL | REFACTOR | P3, P5 |
| tooling | outside | [bash](tooling/bash/MODULE.md) | bash | PARTIAL | HARDEN | P1, P5 |
| tooling | outside | [pi-extension](tooling/pi-extension/MODULE.md) | pi_extension | NOT FINISHED | REFACTOR | P9 |
| tooling | outside | [julia](tooling/julia/MODULE.md) | julia | NOT FINISHED | REFACTOR | P9 |
| tooling | outside | [deploy](tooling/deploy/MODULE.md) | deploy | PARTIAL | HARDEN | P0, P4, P6, P7, P8 |

**Counts:** 53 cards. hee4-contracts 5 · hee4-host 4 · hee4-egress 1 · hee4-core 4 · hee4-worker 7 · hee4-cohesion 5 · hee4-evidence 4 · hee4-habitat 2 · hee4-app 17 · tooling 4.

The MA flag belongs to the whole v3 origin module, so it is not a verdict on the v4 sub-module. Every "FINISHED" is code presence only; no mutants record exists (ER §2).

## v3 module → card coverage (module-audit, 23 modules)
| v3 module | Card(s) |
|---|---|
| worker | hee4-host/{spawn,clients,cgroup-io}, hee4-worker/{native,namespace,aggregate-policy,inference,pi-adapter}, hee4-contracts/{catalogue-data,bounds} |
| contracts | hee4-contracts/{contracts,state-enums,bounds} |
| task | hee4-core/task, hee4-host/clock |
| store | hee4-core/store, hee4-contracts/{state-enums,bounds} |
| recovery | hee4-core/recovery, hee4-contracts/state-enums |
| route | hee4-worker/route, hee4-app/routing |
| roster | hee4-worker/roster |
| context · skills · workflows · cohort · notify | hee4-cohesion/{context,skills,workflows,cohort,notify} |
| check | hee4-evidence/check, hee4-app/{live-verifier-adapter,u64-class} |
| herdr · service | hee4-habitat/{herdr,service} |
| app | 16 hee4-app cards (all except actions), hee4-host/clock, hee4-contracts/bounds |
| actions | hee4-app/actions, hee4-contracts/catalogue-data |
| budget | hee4-core/budget |
| numerical | hee4-evidence/numerical |
| julia | hee4-evidence/julia-decoders, tooling/julia |
| bash · deploy · pi_extension | tooling/{bash,deploy,pi-extension}, hee4-worker/pi-adapter |

**Unplaced: none.** No v3 module is outside or deferred without a card. Deferred *functionality* (v4.1/v4.2 actions, Jev) is recorded on the owning card's §3/§10.

## Deviations from the brief
1. **`hee4-app/u64-class` added** (17 app cards, not 16). v3 `app/u64_receipt` and the U64 class content become a `Class` port implemented in K6 (UM:77; AR row 3). The brief's K6 list had no home for it.
2. **`startup/coordinator` becomes one directory, `startup-coordinator`**, because a path cannot contain `/`.
3. **v3 `julia` has two cards.** `hee4-evidence/julia-decoders` holds the Rust decode side in K4 (UM:77), and `tooling/julia` holds the analysis scripts outside the DAG (UM:80).
4. **New modules with no v3 origin**, with flag/rec `n/a (new)`: judge-types, judge-egress and judge-admission.
5. **`inference` and `pi-adapter` rest only on UM:75.** No reference copy or DP row names them, and each card says so instead of inventing detail.
6. **Mixed origins.** Some cards carry two recommendations: state-enums, catalogue-data, bounds, spawn and control-socket.
7. **Phases marked INTERP.** The ATLAS gives no phase for bash or pi-extension, so the card writers inferred them.
8. **The manifest has extra fields** beyond the brief: `anti`, `h`, `v4`, `errata`.
9. **Counting units.** Anchor-block counts in §6 are *lines* matching `HEE3-ANCHORS|obsidian://`. V4-10's "7,198 refs in 15 files" uses a different unit. Likewise, `install-release` names the v3 install prefix on 3 lines (`:3`, `:366`, `:445`; `:347` and `:350` are cargo-tree test strings naming the v3 package, corrected after V5) against V4-10's "8 install paths".

## Open questions raised by the cards (for the owning phase's flow contract)
- **Clock trait home.** The trait goes in K0 and the impl in K0h. P6 implies this, but no design file states it (`hee4-host/clock`). *(rev 2026-10-01 open-tasks CN-28)* **Resolved (DC-29).**
- **Endpoint-holder I/O** has two homes: UM:170 says K0h and UM:75 says K2 `native/endpoint` (`hee4-host/clients`, `hee4-worker/native`). *(rev 2026-10-01 open-tasks CN-28)* → DC-10 (~~PROPOSED, H-27~~ resolved, rev 2026-10-01 ratified V4-60: raw `/proc` scan in K0h, policy in K2).
- **`cgroup-io` could be K6-private** rather than K0h. *(rev 2026-10-01 open-tasks CN-28)* ~~No DC row; settle in the P3 flow contract (open-tasks SC-04).~~ Settled with DC-38 (rev 2026-10-01 ratified V4-60): it stays in K0h, which holds every I/O door.
- **The K0e HTTP client** cannot spawn curl through K0h, because UM:64 lets K0e depend on K0 only. The choice is deferred to P9 (`hee4-egress/judge-egress`). *(rev 2026-10-01 open-tasks CN-28)* → DC-11 (~~PROPOSED, decide at P9~~ resolved, rev 2026-10-01 ratified V4-66: K0e owns its own in-process HTTPS client).
- **The U64 oracle split** between K4 check and K6 u64-class is undecided, and so is the owner of the class build (`plan` or `u64-class`). *(rev 2026-10-01 open-tasks CN-28)* No DC row; settle in the P3 flow contract (SC-04).
- **Skills and workflows** could stay Python or become Rust (UM:76 says "Rust loader"). *(rev 2026-10-01 open-tasks CN-28)* No DC row; settle in the owning phase's flow contract (SC-04).
- **recovery (KEEP)** was not migrated. Re-derive it from EX-04/05/06, or migrate it on Luke's word (MIG:23). *(rev 2026-10-01 open-tasks CN-28)* → ATLAS H-26 / DC-27: ~~Luke's, at P2~~ decided **migrate** (rev 2026-10-01 ratified V4-59, under the 2026-10-01 delegation; Luke may revoke).

## Funnel integrity
Not self-certification: the checker is same-lineage, and its only independent reader so far is V5, which reviewed the cards, not this script.

**Mechanism (V5 MED-2; hardened 2026-10-01 after an adversarial review, V4-25):** `ops/checks/module_funnel.py` (sha256 `e8e8068fe842b9e14df71d7d253b756991ac6257ebfc1144ea21bb5288fe4a27`) with `ops/checks/cite_pins.py` (sha256 `73c639d3d61261309fc04b214ee98e7c665f4afb764c972487264cba629e4449`), stdlib only. Run from the repo root: `python3 ops/checks/module_funnel.py` (exit 0 PASS · 1 FAIL · 2 usage) plus `--control`, and `python3 ops/checks/cite_pins.py status | pin KEY [--from F] [--force-pin] | repin KEY | --control`. The repo has no commit yet (V4-4), so no `tree=`/`dirty=` can be printed; add them once a commit exists. It checks:
- cards against `MODULES.toml` (and every manifest `migrated_paths` entry exists); the README module table's card set and phases against the manifest; all 23 MA module rows appear in the coverage table;
- all 11 sections present and in order; lines max / mean (rounded) / over 165;
- every AP/EX/A/D/V4/D-U/J/E/H id cited in a card or the manifest matches a **definition row parsed from its defining file**. Fenced blocks and inline-code spans holding regex metacharacters are excluded, so a pull-command fragment such as `AP-0[1249]` is not a cite;
- every `~/…`, `/var/…` and repo path (`docs/ plan/ gates/ migrated/ modules/ ops/`) in a card exists, except paths in the V4-11 `hee4` runtime namespace, which are design targets under the HOLD;
- no v3 working-dir, v3-evidence or v3-worktree path in the cards, this README or the manifest;
- **keylines (V5 MED-1):** every `KEY:n`, `KEY:n-m` or `KEY:n,m` cite in the **cite world** resolves through the legend to a line that exists and is non-blank. The cite world is `modules/**/*.md` (every key; an unknown key fails by name) plus `docs/*.md` (legend keys only). **One scanner** (`module_funnel.scan`) reads it for keyline resolution, cited keys, repin rewriting and the unmapped scan: fenced blocks are excluded everywhere (so repin never rewrites inside a fence), inline code is included. `CITE_WORLD_EXCLUSIONS` in the script names the two excluded classes with their reasons: non-legend keys in `docs/` (CM, FF, MM, SA, … name v3-vault files outside the fence; printed `docs_nonlegend_cites=N`) and the legend source files outside `modules/` and `docs/` (sources, not citers; their cites into each other are a known unchecked class, printed `unchecked_cites=N`). A legend cite in any other repo `.md` fails `cite_outside_world`. It does **not** check that a line is about the cited module;
- **zero measured is a FAIL:** `zero_cards`, `ids_unmeasured`, `paths_unmeasured`, `keyline_unmeasured`, and `pins_unmeasured` (pin status false with no named diagnostic);
- **pins:** `ops/checks/pins/PINS.json` holds, per cited key, the sha256 of the version the cites are written against and the sorted multiset of every cite endpoint for that key across the cite world; `pins/<KEY>.snap` is a copy of that version. Diagnostics: `pin_stale` (the file changed), `pin_missing`, `pin_snapshot_mismatch` (the snapshot is not the pinned sha), `pins_unreadable`, and `keyline_unmapped` with `file:line` for every `KEY:?<spec>` mark, inline code included.

**The pin commands.** After editing any legend file, run `cite_pins.py repin <KEY>` in the same pass (the checker is red until you do). After adding or re-pointing cites by hand while the source is unchanged, run `cite_pins.py pin <KEY>`.
- `pin` is refused when a snapshot exists and the line map from it to the new version is not the identity on every cited line: that is drift, so it says "use repin". `--force-pin` overrides and prints `moved=N of cited=M`.
- `repin` is refused when the current cites differ from the stored multiset, printing the diff count: they were edited by hand, and mapping them again would shift them twice.
- Mapping: equal lines map exactly. Inside a changed block, a markdown table row maps only to the one new row whose stripped first cell is equal (and similar); otherwise the cite is marked. A prose line maps to its most similar prose line (ratio ≥ 0.6). Similarity matches print `similar=N` and each `old→new` pair with both line heads, and are not counted under `unchanged=`.
- A cite with an unmapped endpoint is rewritten `KEY:?<spec>` keeping the **original** spec (snapshot coordinates). Repin writes nothing unless its unmapped count equals the count the status scanner then sees.

~~Output on 2026-10-01 after the change (exit 1):~~ **(superseded by V4-32; current: `verdict=PASS`. The block below is the historical FAIL output, kept as a record; run the funnel for the current line.)** *(rev 2026-10-01 open-tasks CN-24)*
```
cards=53 hee4-app=17 hee4-cohesion=5 hee4-contracts=5 hee4-core=4 hee4-egress=1 hee4-evidence=4 hee4-habitat=2 hee4-host=4 hee4-worker=7 tooling=4
measured_over=53
sections ok=53/53
lines max=118 mean=90 total=4748 over165=0/53
manifest entries=53 cards=53 only_manifest=0 only_cards=0
manifest migrated_paths=66 exist=66/66
readme_table rows=53 manifest=53 mismatched=0
v3_modules=23 mapped=23 unplaced=0
ids cited=141 found=141/141 missing=0 AP=36 EX=20 A=10 D=14 V4=13 D-U=4 J=14 E=14 H=16
paths distinct=84 v4_runtime_targets=13 (V4-11 namespace; not checked under the HOLD) skipped_patterns=3 checked=68 exist=68/68 missing=0
v3_path_refs=0 files_scanned=55
  keyline_blank AR:37-39 at docs/ANTIPATTERNS.md:12 lands on a blank line
  keyline_out_of_range AR:142-145 at docs/ANTIPATTERNS.md:48 (/var/home/Louranicas/hee4-evidence/reference/v3-evidence-b5367bc/architecture-review-b5367bc.md has 49 lines)
  keyline_out_of_range AR:128-131 at docs/ANTIPATTERNS.md:48 (/var/home/Louranicas/hee4-evidence/reference/v3-evidence-b5367bc/architecture-review-b5367bc.md has 49 lines)
  keyline_out_of_range AR:90 at docs/ANTIPATTERNS.md:61 (/var/home/Louranicas/hee4-evidence/reference/v3-evidence-b5367bc/architecture-review-b5367bc.md has 49 lines)
  keyline_out_of_range AR:204-207 at docs/ANTIPATTERNS.md:61 (/var/home/Louranicas/hee4-evidence/reference/v3-evidence-b5367bc/architecture-review-b5367bc.md has 49 lines)
  keyline_out_of_range AR:58-78 at docs/ANTIPATTERNS.md:87 (/var/home/Louranicas/hee4-evidence/reference/v3-evidence-b5367bc/architecture-review-b5367bc.md has 49 lines)
  keyline_out_of_range AR:75-78 at docs/ANTIPATTERNS.md:122 (/var/home/Louranicas/hee4-evidence/reference/v3-evidence-b5367bc/architecture-review-b5367bc.md has 49 lines)
  keyline_out_of_range AR:142-145 at docs/EXEMPLARS.md:121 (/var/home/Louranicas/hee4-evidence/reference/v3-evidence-b5367bc/architecture-review-b5367bc.md has 49 lines)
cite_world files=58 modules=54 docs=4 docs_nonlegend_cites=216 (excluded)
cite_outside_world=0 unchecked_cites=113 (legend sources, excluded: 2 exclusion classes)
keyline cites=2605 resolved=2597/2605 blank=1 out_of_range=7 source_missing=0 unknown_keys=0 legend_keys=24
pins keys_cited=20 fresh=20/20 stale=0 unpinned=0 snapshot_mismatch=0 unmapped_cites=0
verdict=FAIL checks_failed=2 diagnostics=8 (keyline_blank,keyline_out_of_range)
```
**What it finds in `docs/` (now in the world; not fixed here, `docs/` belongs to another reviewer this hour):** 8 legend cites in `docs/ANTIPATTERNS.md` and `docs/EXEMPLARS.md` do not resolve: 7 `AR` ranges past the end of the 49-line `architecture-review-b5367bc.md` copy and 1 `AR` range ending on a blank line. They look like cites into a longer v3 version of the architecture review, not into the v4 copy the legend names; the owner should re-point them or give `docs/` its own key. The two `DP` cites of line 33 resolve, but to a table separator row. Until they are fixed the verdict is FAIL, and that FAIL is the correct output.

`--control` (exit 0) enumerates, with `ast`, every `fail("<name>", …)` site in `check()` and every `diag("<name>", …)` site in `cite_pins.status()`. It plants at least one fault per site, each in its own temp copy of the repo with its own pins dir (the sites that cannot be planted inside `modules/` are planted in the copy's `plan/`, `gates/`, `docs/`, pins dir or a redirected MA). A case counts only if its diagnostic name **and** the detail text only that site produces are in the planted run and not in the unplanted copy, and the planted verdict is FAIL. It refuses a site with no plant, a plant with no site, and a name raised from two sites. Quiet cases (fenced cites, regex inline code, a docs non-legend key, `KEY:?<n>` prose) must produce no new diagnostic. Seven self-tests check the scoring itself, and the real tree is hashed before and after:
```
control case=zero_cards plant='all cards removed' needles=['no card was measured'] fired=yes
control case=sections_bad plant='heading 3 renamed' needles=['hee4-contracts/contracts/MODULE.md', 'not the 11 numbered sections'] fired=yes
control case=card_too_long plant='200 lines appended' needles=['hee4-contracts/contracts/MODULE.md', 'lines > 165'] fired=yes
control case=manifest_unreadable plant='garbage manifest' needles=['MODULES.toml could not be parsed'] fired=yes
control case=manifest_only plant='ghost manifest entry' needles=['hee4-zz/ghost/MODULE.md', 'has no card'] fired=yes
control case=card_not_in_manifest plant='card copied to a new dir' needles=['hee4-zz/planted/MODULE.md', 'no manifest entry'] fired=yes
control case=migrated_path_missing plant='manifest path that does not exist' needles=['migrated/PLANTED_NO_SUCH does not exist'] fired=yes
control case=readme_row_mismatch plant='README phase changed' needles=['hee4-contracts/contracts/MODULE.md', 'README phases', "'P7'"] fired=yes
control case=readme_row_count plant='README row deleted' needles=['README table rows vs', 'manifest entries'] fired=yes
control case=v3_module_unplaced plant='MA row with no coverage' needles=['plantedmod', 'not in the README coverage table'] fired=yes
control case=v3_modules_unmeasured plant='MA with no rows' needles=['no module rows parsed from MA'] fired=yes
control case=id_source_missing plant='D-family source deleted' needles=['D: docs/DRIFT_AND_OVERENGINEERING.md is unreadable'] fired=yes
control case=manifest_id_malformed plant='non-id in ap list' needles=['ap=APX is not an id'] fired=yes
control case=id_missing plant='undefined AP id in a card' needles=['AP-99', 'cited in hee4-contracts/contracts/MODULE.md', 'not defined in'] fired=yes
control case=id_missing plant='undefined AP id in the manifest' needles=['AP-97', 'cited in MODULES.toml', 'not defined in'] fired=yes
control case=ids_unmeasured plant='every id removed' needles=['no AP/EX/A/D/V4/D-U/J/E/H id was cited'] fired=yes
control case=path_missing plant='missing docs path' needles=['docs/NO_SUCH_FILE_PLANTED.md (in hee4-contracts/contracts/MODULE.md) does not exist'] fired=yes
control case=paths_unmeasured plant='every path removed' needles=['no path was checked'] fired=yes
control case=v3_path_ref plant='v3 path in a card' needles=['hee4-contracts/contracts/MODULE.md:', 'names a v3 home', 'v3-evidence'] fired=yes
control case=v3_path_ref plant='v3 path in the README' needles=['README.md:219', 'names a v3 home', 'v3-evidence'] fired=yes
control case=v3_path_ref plant='v3 path in the manifest' needles=['MODULES.toml:', 'names a v3 home', 'v3-evidence'] fired=yes
control case=keyline_unknown_key plant='unknown key in a card' needles=['ZZQ:5 at modules/hee4-contracts/contracts/MODULE.md:94', 'is not a LEGEND key'] fired=yes
control case=keyline_unknown_key plant='unknown key in the README' needles=['ZZQ:6', 'modules/README.md:219', 'is not a LEGEND key'] fired=yes
control case=keyline_source_missing plant='REQ source deleted' needles=['REQ:', 'gates/REQUIREMENTS.md is unreadable'] fired=yes
control case=keyline_out_of_range plant='line past the end, in a card' needles=['UM:999999 at modules/hee4-contracts/contracts/MODULE.md:94', 'lines)'] fired=yes
control case=keyline_out_of_range plant='line past the end, in docs/' needles=['UM:999999', 'docs/EXEMPLARS.md:631', 'lines)'] fired=yes
control case=keyline_blank plant='blank line' needles=['UM:2 at modules/hee4-contracts/contracts/MODULE.md:94', 'lands on a blank line'] fired=yes
control case=keyline_blank plant='range whose high end is blank' needles=['UM:1-2 at modules/hee4-contracts/contracts/MODULE.md:94', 'lands on a blank line'] fired=yes
control case=keyline_blank plant='second part of a multi-cite is blank' needles=['UM:2 at modules/hee4-contracts/contracts/MODULE.md:94', 'lands on a blank line'] fired=yes
control case=keyline_blank plant='cite inside inline code is scanned' needles=['UM:2 at modules/hee4-contracts/contracts/MODULE.md:94', 'lands on a blank line'] fired=yes
control case=keyline_unmeasured plant='every cite removed' needles=['no KEY:line cite was found'] fired=yes
control case=cite_outside_world plant='legend cite in plan/PLANTED.md' needles=['UM:5 at plan/PLANTED.md:1', 'is outside the cite world'] fired=yes
control case=pins_unmeasured plant='no key cited' needles=['returned false with no named diagnostic'] fired=yes
control case=pin_stale plant='APX source edited' needles=['APX:', 'changed since its cites were pinned'] fired=yes
control case=pin_missing plant='APX pin dropped' needles=['APX: no pin for'] fired=yes
control case=pin_snapshot_mismatch plant='APX snapshot edited' needles=['APX:', 'snapshot sha256'] fired=yes
control case=pins_unreadable plant='garbage PINS.json' needles=['PINS.json could not be parsed'] fired=yes
control case=keyline_unmapped plant='KEY:?n inside inline code in a card' needles=['UM:?3', 'modules/hee4-contracts/contracts/MODULE.md:94'] fired=yes
control case=keyline_unmapped plant='KEY:?n range in docs/' needles=['UM:?4-6', 'docs/EXEMPLARS.md:631'] fired=yes
control quiet='fenced cites are not cites' needles=['ZZQ', 'UM:999998', 'UM:?8'] held=yes
control quiet='regex inline code is not an id' needles=['AP-98'] held=yes
control quiet='docs non-legend key is excluded' needles=['ZZQ'] held=yes
control quiet='KEY:?<n> prose is not a mark' needles=['KEY:?'] held=yes
control count=unchecked_cites baseline=113 planted=114 moved_by_one=yes
control sites=30 clauses=30/30 cases=39/39 quiet=4/4 counts=1/1 self_tests=7/7 baseline_copy=FAIL real_tree_unchanged=yes verdict=PASS
```
`cite_pins.py --control` (exit 0) runs the fixtures in a temp world: a 3-line shift (with an inline-code cite, a fenced cite left alone and a docs cite), a double repin, a changed table-row first cell next to two similar lines, a deletion (ranges keep their original spec; the mark count equals status's), pin over drift, a hand shift followed by repin, and pin after adding a cite:
```
cite_pins control case=shift3_maps_all+inline_code+fence_untouched+docs ok=yes | repin SRC files_rewritten=2 endpoints moved=7 unchanged=0 similar=0 cites_unmapped=0
cite_pins control case=double_repin_idempotent ok=yes | repin SRC files_rewritten=0 endpoints moved=0 unchanged=7 similar=0 cites_unmapped=0
cite_pins control case=row_first_cell_changed_is_?+rewritten_prose_is_?+similar_not_unchanged ok=yes | repin SRC files_rewritten=1 endpoints moved=0 unchanged=1 similar=2 cites_unmapped=2
cite_pins control case=deletion_is_?+original_spec+count_equals_status ok=yes | repin SRC files_rewritten=1 endpoints moved=1 unchanged=0 similar=0 cites_unmapped=3
cite_pins control case=pin_over_drift_refused+force_names_moved ok=yes | pin refused: SRC: 7 of 7 cited lines are not the identity from the pinned snapshot to /tmp/cite-pins-control-rnd7p6y3/6/w/src/S.md (first: [2, 3, 5, 7, 8]); that is drift: use repin SRC
cite_pins control case=hand_shift_then_repin_refused ok=yes | repin refused: SRC: the current cites differ from the stored ones (diff=4: added=2 removed=2); they were edited by hand. Check them; `pin SRC` if the source is unchanged, else `pin SRC --force-pin`
cite_pins control case=pin_unchanged_source_records_new_cites ok=yes | pinned SRC sha256=3276b0dd90b2be3806563f48fc272260060c6610c5af273f69961468a2333de6 cites=8 from=/tmp/cite-pins-control-rnd7p6y3/8/w/src/S.md
cite_pins control cases=7/7 real_tree_unchanged=yes verdict=PASS
```
`cite_pins.py status` (exit 0):
```
pins keys_cited=20 fresh=20/20 stale=0 unpinned=0 snapshot_mismatch=0 unmapped_cites=0
```
**Initialising the stored cite multisets (2026-10-01).** Before this change the tree passed `keyline cites=2572 resolved=2572/2572` with `pins fresh=20/20`. `cite_pins.py pin <KEY> --force-pin` was run once for each of the 20 cited keys against the current files. Every key printed `moved=0`, so no cite was re-declared over drift. The 10 `outside_snapshot` endpoints under `AR` are the out-of-range `docs/` cites above. `plan/DECISIONS.md` had gained V4-26 from another session in the meantime, which is append-only. Then V4-25 was appended and `repin DEC` ran: `moved=0 unchanged=203 similar=0 cites_unmapped=0`.

**What the review found, and the measured result.** The adversarial review's harness neutered 37 mutants; the old `--control` killed 9 of 37 (28 survived). The harness was adapted to the new sites: each `fail`/`diag` site is neutered as `False and fail(…)`, so the site is still enumerable and only its plant can kill it, and the review's semantic mutants are carried over (scope, guards, scanner, mapping, control meta). Result on a copy: **68 of 69 killed** (30 site neuters, 30 of 30 killed, plus 39 semantic mutants). The one survivor is `repin_count_mismatch_off`, which disables the guard that refuses a repin whose unmapped count disagrees with the status scanner. Under the current code the two counts cannot differ, because both come from one scanner and every unmapped cite is written as exactly one mark. The survivor is therefore equivalent: the guard is there for a future divergence. It is recorded here and no test was added for it. A first adapted run scored 67 of 69. It also let `similarity_threshold_off` survive, and that was fixed with a rewritten-prose fixture. The review's `hist.py` replay of the real ATLAS edit gives the same numbers with the old and new mapper (`similarity_mapped=10 rows_same_id=10 rows_diff_id=0`), so MED-1 did not misfire on that edit; the wrong-row case is pinned by the `cite_pins` fixture instead. Fixed: HIGH-1 (stored cites; pin refuses drift; repin refuses hand edits), HIGH-2 (four `*_unmeasured` refusals), HIGH-3 (site-enumerated control, `clauses=N/N`, `cite_pins --control`), MED-1 (first-cell rule; `similar=` reported apart), MED-2/3 (one scanner, fences excluded everywhere, inline code included, counts must agree), MED-4 (docs in the world; exclusions with reasons; `unchecked_cites`), LOW-2 to LOW-5. Known limit: the multiset comparison cannot see a hand edit that leaves the multiset of endpoints unchanged. A cite written straight after a `/` (the `JM` cite after the `DP` cite in the V5 LOW list below) is not scanned (the key regex's look-behind); it is an old limit and is left as is.

**The ATLAS drift, and its repair (2026-10-01).** `DEPLOYMENT_ATLAS.md` was edited at 08:12, after the cards were written: 18 final-lessons edits, then 11 V6 fixes. That moved every `AT:` cite. The phase rows went from 74-83 to 79-88 and the H-list from 177-194 to 185-202. The first checker saw only the 115 cites that landed on a blank line. The other ~387 resolved silently to wrong lines, which is the flattering direction.

The repair:
- **The version the cards were written against was recovered** from the lessons agent's own `cat` of the file in its transcript. It checks out: 0 of the 502 `AT:` cites are blank against it, and all 16 of that agent's `Edit` `old_string`s occur in it verbatim.
- **It was pinned as the AT snapshot, then `repin AT` ran.** 483 cites mapped mechanically. I compared every mapped pair's old line with its new line: 400 are identical, and all the rest start with the same row id (amended rows).
- **19 cites (the H-3 and H-8…H-12 rows and one §7 bullet, whose text was rewritten) were re-pointed by row id**, to 187, 192, 192-195, 192-196 and 223.
- **Every other key was pinned to its current file.** V5 checked its 12 cards' cites against those versions. The other 41 cards' non-AT cites are covered only by the non-blank check.

**Applied from V5 (2026-10-01):**
- MED-1: legend completed with paths (all 24 keys, above); the checker resolves every `KEY:line`. contracts AR line 14 → `AR:19` and MA line 2 → `MA:8` (checked: AR line 19 and MA line 8 are the contracts rows); live-verifier-adapter EXX lines 543-555 → `EXX:543-554` (line 555 is blank; A-5 ends at 554).
- MED-2: checker in the repo with its sha256; mean now rounded (`mean=90`, 4,748/53 = 89.58).
- MED-3: runtime §9 #1 cites the 1,500-line figure to AP-08's TRIGGER column (APX:52; its symptom column says ~2,000) and names a `use`-graph census over `crates/hee4-app/src` printing `cycles=0`, marked to build at "start coding".
- LOW: contracts §6 adds `receipt_schema.py` = 496 lines and states the `include_str!` plainly; roster P3 row marked INTERP (from MA:13) and "N days" moved to backup prune (`hee4-app/backup-target` §10); runtime and candidates put H-9 on P4; candidates uses `:220-236` as DP:38/JM:29 do, with the A-6 `:219` difference explained; candidates §9 #2/#3 name printed numbers (`kinds_stated=7/7`, `cycles=0`); service EX-08 glossed correctly and kept because A-7's "Use instead" names it (EXX:590); deploy install-prefix count corrected to 3 lines and its P0 row marked INTERP; `MODULES.toml` runtime gains V4-0.
- Not changed: ~~V5's six generic §7 rows other than service EX-08 (a rewrite needs the module owner's judgment);~~ *(rev 2026-10-01 open-tasks CN-19)* the other five generic §7 rows are now module-specific (contracts AP-04/05, check AP-22, deploy D-16, store AP-32, candidates D-02/D-03), each marked; the `migrated/` MANIFEST sha256 (not verified by V5 or here).
