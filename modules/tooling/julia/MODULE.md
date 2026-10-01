# tooling · julia
> **Status: PLANNING — HOLD** (V4-0, DEC:4). No code until Luke says "start coding". This card is the stub.

## 1 · Identity
| Field | Value | Source |
|---|---|---|
| Crate | none — **outside the crate DAG** (`julia/` analysis scripts) | UM:80 |
| Cluster | outside (v3 design: K4) | CMAP:3; UM:80 |
| v3 origin | `julia` (`bin/analysis.jl`, `Cohesion.jl`) | MA:27; AR:15; IM:61 |
| Companion | the Rust decode side is `modules/hee4-evidence/julia-decoders/` | UM:77 |
| Status | PLANNING — HOLD; v4.2 | DEC:4; UM:174, UM:269 |

**Design section:** [T tooling › julia (v4.2)](obsidian://open?vault=herdr-engineering-engine-v4.vault&file=15%20Module%20Design/T%20tooling%23julia%20%28v4.2%29): logic flow, interfaces, S-n/actions, RL/E2E, refusals, Must not, size budget. Elaboration only; this card and the authorities win. Index: [Module Design Index](obsidian://open?vault=herdr-engineering-engine-v4.vault&file=15%20Module%20Design%2F00%20-%20Module%20Design%20Index)

## 2 · Purpose, owned state, allowed dependencies
| Item | Value | Source |
|---|---|---|
| Purpose | Offline, 1-thread analysis script spawned by K0h for K4 `numerical` | IM:61; UM:174 |
| Owned state | None; pinned artifact | UM:80 |
| Reads | the one shared versioned schema (constants generated for both sides) | AR:15; UM:77 |
| Must | pin the julia depot; drop the Cohesion pin (unreachable); no re-implementation of `cohort::join` or budget conservation | UM:77, UM:262; AR:15, AR:23 |

## 3 · Deployment
| Phase | Entry gate | Exit evidence | Held-for-Luke |
|---|---|---|---|
| P9 Used (post-tag), numerical flow | tag pushed | per-slice gate (AT:88) | **H-14** production Julia pins (AT:199) |

## 4 · v3 basis
Flag **NOT FINISHED** (MA:27): "Only descriptive is reachable; cohesion unreachable". Recommendation **REFACTOR** (AR:15): Cohesion pinned but unreachable; re-implements cohort::join and budget conservation; wrapping UInt64 sum; no shared versioned schema.

| Finding | v3 file:line | Source | Errata |
|---|---|---|---|
| derive_join/conservation duplicate cohort::join | `Cohesion.jl:657/824` | DP:124 | — |
| comparison is built at `:921` | `Cohesion.jl:921` | DP:125 | **E12** |
| analysis.jl has test callers only | `numerical/process.rs:220-231` | IM:61 | — |
| dependency depot unpinned | — | AR:23 | — |

## 5 · Decision points
| site · kind · note | Jev |
|---|---|
| julia derive_join/conservation `Cohesion.jl:657/824` · EXACT (DP:124) | not-Jev |
| julia cohesion comparison `:921` · JUDGMENT · descriptive only (DP:125) | **J14 not-Jev** (JM:42) |

## 6 · Migrated inputs
None: julia is REFACTOR (MIG:22). The only julia-adjacent migrated input is the Rust caller `migrated/v3-b5367bc/src/numerical/process.rs` (spawn + `JULIA_*` env) and fixture needs noted at MIG:25.

## 7 · Quality guard
| id | why for THIS module |
|---|---|
| AP-01 | a second join/conservation engine in julia |
| AP-13 | Cohesion pinned but unreachable: an artifact nothing runs |
| AP-21 | Rust reference and julia must stay independent implementations |
| AP-04 | the julia output is caller-controlled input to K4: bounded at acquisition |
| D-05 | julia scripts count as product only when a flow runs them |

## 8 · Interfaces
| Kind | Item | Source |
|---|---|---|
| Spawned by | K0h spawn door, caller K4 | UM:174 |
| Env | `JULIA_*` allowlist | IM:66 |
| Output | decoded by `julia-decoders` (closed code set) | UM:77 |

## 9 · Done criteria
| # | criterion | evidence / read-back |
|---|---|---|
| 1 | Depot pinned by digest | cold clone reads the depot from a committed, digest-pinned manifest (AT:160) |
| 2 | No duplicate join/conservation | Cohesion removed or unpinned (UM:262) |
| 3 | Shared schema drives constants | generator check; plant on one side killed |
| 4 | Runs offline in the gate | the gate's run of analysis.jl shows no network and prints `elapsed/budget` (AT:165) |

## 10 · Open decisions and risks
- "Compose or park" numerical/julia for v4.2 (MIG:14; UM:269).
- H-14 production pins.
- Resolved, not open *(rev 2026-10-01 funnel audit)*: the register rows naming this module are all RESOLVED, ratified under delegation inside H-27's range: DC-44 (V4-66): Julia constants' schema home. The decision text is the V4 row in `plan/DECISIONS.md`; `hee4db highway --dc <DC-nn>` shows the row.

## 11 · Pull commands
```bash
E=~/hee4-evidence; R=$E/reference/v3-evidence-b5367bc
sed -n '77p;80p;172p;260p;267p' $E/design/ULTRAMAP.md
sed -n '83p;190p' $E/design/DEPLOYMENT_ATLAS.md
sed -n '124,125p' $E/design/DECISION_POINTS-b5367bc.md
sed -n '15p;23p' $R/architecture-review-b5367bc.md; sed -n '27p' $R/module-audit-b5367bc.md; sed -n '61p;66p' $R/interface-map-b5367bc.md
grep -n 'E12' $E/reference/ERRATA-v3-evidence-b5367bc.md
grep -n 'JULIA\|julia' ~/herdr-engineering-engine-v4/migrated/v3-b5367bc/src/numerical/process.rs | head
```
