# hee4-evidence · numerical
> **Status: PLANNING — HOLD** (V4-0, DEC:4). No code until Luke says "start coding". This card is the stub.

## 1 · Identity
| Field | Value | Source |
|---|---|---|
| Crate | `hee4-evidence` | UM:77 |
| Cluster | K4 | UM:77; CMAP:3 |
| v3 origin | `numerical` (`src/numerical.rs`, `src/numerical/process.rs`) | MA:26; MIG:14 |
| Status | PLANNING — HOLD; release scope **v4.2** (analysis.* actions) | DEC:4; UM:163, UM:269 |

**Design section:** [K4 hee4-evidence › numerical](obsidian://open?vault=herdr-engineering-engine-v4.vault&file=15%20Module%20Design/K4%20hee4-evidence%23numerical): logic flow, interfaces, S-n/actions, RL/E2E, refusals, Must not, size budget. Elaboration only; this card and the authorities win. Index: [Module Design Index](obsidian://open?vault=herdr-engineering-engine-v4.vault&file=15%20Module%20Design%2F00%20-%20Module%20Design%20Index)

## 2 · Purpose, owned state, allowed dependencies
| Item | Value | Source |
|---|---|---|
| Purpose | Offline numerical analysis of a dataset (descriptive recipe) checked against an independent Rust reference; caller of the julia `bin/analysis.jl` process | UM:77, UM:174; IM:61 |
| Owned state | None durable | UM:77 |
| Allowed deps | `hee4-contracts` (K0), `hee4-host` (K0h, for the spawn door). The v3 edge numerical → worker existed only to reach `worker::process`; in v4 it goes through K0h | UM:62, UM:57; CMAP:12; ER E1 |
| Absorbs | `ProcessReport::settled()` + one hasher (from the numerical/service copies) move to K0h (*rev 2026-10-01 ratified V4-55, DC-16*). The local 10 s cleanup reserve (`numerical/process.rs:23`) is renamed **`NUMERICAL_CLEANUP_RESERVE`** before it moves to K0, and `CLEANUP_RESERVE` stays RC01's 5 min (*rev 2026-10-01 ratified V4-63, DC-41*); pin the julia depot; uncertainty + memory bound **or declared absent** | UM:72, UM:77; AR:23 |

## 3 · Deployment
| Phase | Entry gate | Exit evidence | Held-for-Luke |
|---|---|---|---|
| P9 Used (post-tag), milestone-2 flow "numerical" | tag pushed | per-slice gates; l2 moves (AT:88) | **H-14** (O-15: production Julia pins, T15 isolation) (AT:199); C04 production needs O-15 (UM:163) |
Not on the v4.0 done-line; feeds D8 only as a `reasoned` scoreboard row until P9 (AT:65).

## 4 · v3 basis
Flag **NOT FINISHED** (MA:26): "Descriptive recipe only; no uncertainty, memory bound or caller". Recommendation **HARDEN** (AR:23): settle check and pinned-hash loop copied from service; dependency depot unpinned → `ProcessReport::settled()` + one hasher in worker; pin the depot.

| Finding | v3 file:line | Source | Errata |
|---|---|---|---|
| Undeclared edge numerical → worker (process door) | `numerical/process.rs:4` | CMAP:12; V1:46 | E1 (7 edges, not 6) |
| julia process has **test callers only** | `numerical/process.rs:220-231` | IM:61 | — |
| `classify` lives in the submodule, not numerical.rs | `numerical/process.rs:143` | DP:122 | **E16** (numerical.rs:143 is an anchor comment) |
| Library unit test `include_bytes!`s a fixture not migrated | `src/numerical/process.rs:417` → `tests/fixtures/t21/J01.json` | MIG:25; V1:59 | — |
| t21 has a 12 s margin on a 180 s step | t21 gate step | PL:43 (L7) | — |

## 5 · Decision points
| site · kind · note | Jev |
|---|---|
| numerical validate :794 · classify `numerical/process.rs:143` · EXACT (DP:122) | not-Jev (EXACT) |
| numerical report tolerance :733 · THRESHOLD · 1e-12, 8·ε (DP:123; `numerical.rs:752-754` in the migrated copy) | not-Jev (numbers, JM:23) |
| Cohesion comparison (the julia side) · JUDGMENT (DP:125) | **J14 not-Jev** (JM:42); see `julia-decoders` |

## 6 · Migrated inputs
| File | Lines | Anchor lines | Notes |
|---|---|---|---|
| `migrated/v3-b5367bc/src/numerical.rs` | 842 | 272 | strip the anchor block (DEC:27, V4-10) |
| `migrated/v3-b5367bc/src/numerical/process.rs` | 454 | 0 | `use crate::worker::process` (:4) must become a K0h import |
| `migrated/v3-b5367bc/tests/t21_analysis.rs` | 641 | 0 | needs `tests/fixtures/t21/J01.json` (not copied, MIG:25) |
| `migrated/v3-b5367bc/tests/t21_process.rs` | 683 | 0 | same fixture; spawns julia |
MIG:14 lists 4 files / 2,620 lines (it counts the two `.rs` sources + two tests). Rename every `hee3`/`habitat-engine` spelling to the `hee4` namespace (DEC:31, V4-11). The J01 fixture must come from a recording, not be retyped (F113; AT:138).

## 7 · Quality guard
| id | why for THIS module |
|---|---|
| AP-01 | settle check + pinned-hash loop duplicated from service (AR:23); one hasher only |
| AP-11 | the numerical → worker edge was undeclared; K0h is the only route to spawn |
| AP-21 | the Rust reference must be independent of the julia recipe it checks (DP:123 tolerances) |
| AP-19 | tolerances `1e-12`, `8·ε` must be asserted against the contract text, not by their own name |
| AP-06 | julia wall time and host state must not decide a branch; policy pure over the report |
| AP-31 | every julia run needs a budget asserted with both numbers |
| EX-07 | imitate the pure settle step (one poll → decision) |
| EX-20 | constants checked against contract text |
| D-01 | numerical is post-tag: a numerical slice must name the flow it moves |
| D-14 | no numerical detector outlives its catches |

## 8 · Interfaces
| Kind | Item | Source |
|---|---|---|
| Actions | `analysis.request`, `analysis.get` — refused (C04) in v3; v4.2 | IM:24; UM:163 |
| Process | julia `bin/analysis.jl`, offline, 1 thread, via K0h spawn; K4 is the caller | IM:61; UM:174 |
| Env | `JULIA_*` allowlist for the child | IM:66 |

## 9 · Done criteria
| # | criterion | evidence / read-back |
|---|---|---|
| 1 | No crate edge except K0/K0h | planted `use hee4_worker` fails `cargo check` with E0432/E0433 naming the path (UM:84) |
| 2 | One settle + one hasher in K0h, numerical reads them | one-door census `duplicate_sites=0` (UM:39) |
| 3 | Uncertainty + memory bound, or each declared absent with a reason | MA:26 criteria; a refusal named when a bound is exceeded, both numbers printed |
| 4 | Julia depot pinned by digest | manifest row; mismatch refuses by name (UM:77; H-14 for production pins) |
| 5 | analysis.* served through main | gate test through the socket; scoreboard flow row moves (AT:88) |
| 6 | Tolerance rule pinned | plants on tolerance/classify killed under `--cap-lints=warn` by a named test; scoped `cargo mutants` with `CARGO_TARGET_DIR` unset (AT:132, AT:163) |

## 10 · Open decisions and risks
- O-15 production Julia pins are Luke's (H-14, AT:199).
- "Compose or park" (MIG:14): whether numerical ships at all in v4.2 is open; DEFERRED to v4.2 (UM:269).
- Risk: t21 timing margin (PL:43) — use logical clocks, print `margin=` (AT:165).

## 11 · Pull commands
```bash
E=~/hee4-evidence; R=$E/reference/v3-evidence-b5367bc; V=~/herdr-engineering-engine-v4; M=$V/migrated/v3-b5367bc
sed -n '57p;72p;77p;161p;172p;267p' $E/design/ULTRAMAP.md
sed -n '83p;190p' $E/design/DEPLOYMENT_ATLAS.md
sed -n '122,125p' $E/design/DECISION_POINTS-b5367bc.md
sed -n '23p' $R/architecture-review-b5367bc.md; sed -n '26p' $R/module-audit-b5367bc.md
sed -n '24p;61p;66p' $R/interface-map-b5367bc.md
grep -n 'E1 \|E16' $E/reference/ERRATA-v3-evidence-b5367bc.md
sed -n '14p;25p' $M/MIGRATION.md
ls -l $M/src/numerical.rs $M/src/numerical/ $M/tests/t21_*.rs
grep -n 'crate::\|include_bytes' $M/src/numerical.rs $M/src/numerical/process.rs
```
