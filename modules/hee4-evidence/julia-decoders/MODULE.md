# hee4-evidence · julia-decoders
> **Status: PLANNING — HOLD** (V4-0, DEC:4). No code until Luke says "start coding". This card is the stub.

## 1 · Identity
| Field | Value | Source |
|---|---|---|
| Crate | `hee4-evidence` | UM:77 |
| Cluster | K4 | UM:77 |
| v3 origin | `julia` (Rust side: the decode of julia output; the `.jl` scripts themselves are the tooling card `modules/tooling/julia/`) | MA:27; AR:15; UM:80 |
| Status | PLANNING — HOLD; release scope v4.2 (with numerical) | DEC:4; UM:269 |
| Deviation | ULTRAMAP names "julia decoders" in K4 and `julia/` outside the DAG; this funnel splits them into two cards (Rust decoder here, scripts in tooling) | UM:77, UM:80 |

**Design section:** [K4 hee4-evidence › julia-decoders](obsidian://open?vault=herdr-engineering-engine-v4.vault&file=15%20Module%20Design/K4%20hee4-evidence%23julia-decoders): logic flow, interfaces, S-n/actions, RL/E2E, refusals, Must not, size budget. Elaboration only; this card and the authorities win. Index: [Module Design Index](obsidian://open?vault=herdr-engineering-engine-v4.vault&file=15%20Module%20Design%2F00%20-%20Module%20Design%20Index)

## 2 · Purpose, owned state, allowed dependencies
| Item | Value | Source |
|---|---|---|
| Purpose | Rust decoders for the julia analysis output with a **closed code set**, and one schema that generates both sides' constants | UM:77; AR:15 |
| Owned state | None durable | UM:77 |
| Allowed deps | `hee4-contracts` (K0), `hee4-host` (K0h) | UM:62, UM:77 |
| Not here | re-implementations of `cohort::join` and budget conservation in julia (dropped: one join engine lives in K3, conservation in K1) | AR:15; DP:124; UM:76 |

## 3 · Deployment
| Phase | Entry gate | Exit evidence | Held-for-Luke |
|---|---|---|---|
| P9 Used (post-tag), numerical flow | tag pushed | per-slice gates (AT:88) | **H-14** production Julia pins (AT:199) |
Not on the v4.0 done-line (AT:90).

## 4 · v3 basis
Flag **NOT FINISHED** (MA:27): "Only descriptive is reachable; cohesion unreachable". Recommendation **REFACTOR** (AR:15): Cohesion pinned but unreachable; re-implements `cohort::join` and budget conservation; wrapping UInt64 sum; no shared versioned schema → split or unpin Cohesion; Rust decoders + closed code set; one schema generating both sides' constants.

| Finding | v3 file:line | Source | Errata |
|---|---|---|---|
| A second copy of `cohort::join` / conservation | `Cohesion.jl:657/824` | DP:124 | — |
| Cohesion comparison is descriptive (built at `:921`) | `Cohesion.jl:921` | DP:125 | **E12** (was cited :752, the `cohesion()` entry) |
| `julia Cohesion` pin unreachable → unpinned in v4 | — | UM:262 | — |
| Wrapping UInt64 sum | Cohesion.jl | AR:15 | — |

## 5 · Decision points
| site · kind · note | Jev |
|---|---|
| julia derive_join/conservation `Cohesion.jl:657/824` · EXACT · a second copy of cohort::join (DP:124) | not-Jev |
| julia cohesion comparison `:921` · JUDGMENT · descriptive only (DP:125) | **J14 not-Jev**: numbers (JM:42; E12) |

## 6 · Migrated inputs
None: julia is REFACTOR, not carried verbatim (MIG:22). The Rust `numerical` copy (`migrated/v3-b5367bc/src/numerical.rs`) holds the current decode path for the descriptive recipe; read it as reference only. Names follow `hee4` (DEC:31).

## 7 · Quality guard
| id | why for THIS module |
|---|---|
| AP-01 | julia re-implements join and conservation: two doors on one rule (AR:15) |
| AP-21 | expected decode values must come from a recorded julia run, never retyped (AT:138) |
| AP-26 | parse the shared schema; never regex the julia output |
| AP-14 | an unknown code must refuse by name, not default (closed code set) |
| AP-13 | Cohesion was pinned and unreachable: no decoder for an output nothing produces |
| EX-05 | imitate: one spelling per enum, `parse → None` |
| EX-01 | imitate: the acquisition itself is the bound (bounded decode) |
| D-08 | one schema generates both sides; no generated block in source (AP-45) |
| D-10 | if decoder survivors do not fall, move the rule into the type |

## 8 · Interfaces
| Kind | Item | Source |
|---|---|---|
| Input | stdout of julia `bin/analysis.jl` spawned via K0h | IM:61; UM:174 |
| Schema | one versioned schema generating Rust and julia constants, **owned by this card** (K0's emitted schema keeps the catalogue, bounds and tool projection; *rev 2026-10-01 ratified V4-66, DC-44*) | AR:15; UM:77 |
| Called by | `numerical` (K4) | UM:77 |

## 9 · Done criteria
| # | criterion | evidence / read-back |
|---|---|---|
| 1 | Closed code set: every unknown code refuses by name | unit test over a recorded fixture + a planted unknown code |
| 2 | One schema, both sides | a generator check proving the julia constants equal the Rust ones; a plant changing one side is killed |
| 3 | No second join/conservation engine | `Cohesion.jl` join/conservation removed or unpinned (UM:262); one-door census `duplicate_sites=0` |
| 4 | Decoder pinned | scoped `cargo mutants` (`CARGO_TARGET_DIR` unset); plants under `--cap-lints=warn` naming the killing test (AT:132, AT:163) |

## 10 · Open decisions and risks
- Split or unpin Cohesion (AR:15): v4 default is unpinned (UM:262); re-open only if a flow needs it.
- Production julia pins are Luke's (H-14).

## 11 · Pull commands
```bash
E=~/hee4-evidence; R=$E/reference/v3-evidence-b5367bc; V=~/herdr-engineering-engine-v4
sed -n '76,77p;80p;172p;260p;267p' $E/design/ULTRAMAP.md
sed -n '124,125p' $E/design/DECISION_POINTS-b5367bc.md
sed -n '15p' $R/architecture-review-b5367bc.md; sed -n '27p' $R/module-audit-b5367bc.md
grep -n 'E12' $E/reference/ERRATA-v3-evidence-b5367bc.md
grep -n 'J14' $R/jev-decision-map-b5367bc.md
grep -n 'tolerance\|decode' $V/migrated/v3-b5367bc/src/numerical.rs | head -20
```
