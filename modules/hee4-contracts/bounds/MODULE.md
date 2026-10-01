# hee4-contracts · bounds
> **Status: PLANNING — HOLD** (V4-0, DEC:4). No code until Luke says "start coding". This card is the stub.

## 1 · Identity
| Field | Value | Source |
|---|---|---|
| Crate | `hee4-contracts` (module `rc01` + the other limit constants) | UM:71 |
| Cluster | K0 | UM:71 |
| v3 origin | `contracts/rc01.rs` (22 lines) + `contracts/control.rs` consts + bounds spelled across worker, store, app, bash | migrated `src/contracts/rc01.rs`; AR:10, AR:36 |
| Status | PLANNING — HOLD | DEC:4 |

**Design section:** [K0 hee4-contracts › bounds](obsidian://open?vault=herdr-engineering-engine-v4.vault&file=15%20Module%20Design/K0%20hee4-contracts%23bounds): logic flow, interfaces, S-n/actions, RL/E2E, refusals, Must not, size budget. Elaboration only; this card and the authorities win. Index: [Module Design Index](obsidian://open?vault=herdr-engineering-engine-v4.vault&file=15%20Module%20Design%2F00%20-%20Module%20Design%20Index)

## 2 · Purpose, owned state, allowed dependencies
| Item | Value | Source |
|---|---|---|
| Purpose | **P5 one door per rule:** every bound, limit and vocabulary has exactly one definition site; every other site reads it | UM:39 |
| Owned state | None; owns "every limit constant (`rc01`)" | UM:71 |
| Allowed deps | none beyond serde (`const`s) | UM:71 |
| Consumers | K0h (curl 180/5 s, TERM_GRACE), K1 (frame bound, TASK_LIMIT, MAX_ATTEMPTS), K2 (run limit), K6 (rate 100/s, burst 32, cap 8), bash/Pi via emitted schema | UM:39, UM:99, UM:169, UM:80; DP:99 |

## 3 · Deployment
| Phase | Entry gate | Exit evidence | Held |
|---|---|---|---|
| P0 | H-5 | one-door census prints `duplicate_sites=N`, read by the gate | H-5 — AT:79, DEC:37 |
| P1 | P0 | frame bound ≤ 1 MiB + version from here | none — AT:80 |
| P3 | P2 | "limits in one `contracts::rc01`" (run limit spelled 4×) | none — AT:82 |
Feeds D3/D6 indirectly (every served path reads its bound here).

## 4 · v3 basis
- **Flag:** contracts PARTIAL (MA:8). **Recommendation:** contracts HARDEN — "the attempt limit is decided twice" (AR:19); worker REFACTOR — "limits in contracts::rc01" (AR:10); SYS rec 5 "point every bound at its contract constant" (AR:45).

| Finding | v3 file:line | Source | Errata |
|---|---|---|---|
| Run limit spelled 4× | `worker/mod.rs:713` 1,200,000 ms; `:737` 900,000 ms; `native.rs:394` 15 min; `process.rs:541` 20 min | AR:10; UM:39 *(cited)* | — |
| Frame bound re-typed | `store.rs:1236`, `:1617`, `:1705`; `coordinator.rs:53` | AP-01; A-4; AR:36 | — |
| TERM_GRACE "made one" then literal 5 s | `worker/resources.rs:46-48`; `namespace.rs:1364` | DP:52; A-3; EX-18 | — |
| curl 180 s / 5 s literal | `native.rs:1300` | DP:45; UM:169 | — |
| Bash clamps the deadline the engine refuses (>60 s) | `integrations/bash/hee3:264,275` | AR:26; DP:132; A-9 | — |
| `EventCursorV1` unbounded; context `Permit` unbounded | `contracts/events.rs:7` | MA:8; AR:21 | — |
| Roster observations 4,096 global cap, never pruned (a cap that bounds the wrong set) | `contracts/roster.rs:9` *(cited)* | UM:42 | — |
| *(rev 2026-10-01 ratified V4-63, DC-41)* Added rows: the aggregate slice `CPUQuota` 400% (4 s/s), `MemoryMax` 16 GiB and `TasksMax` 256 (CD "Aggregate application slice"), plus `NUMERICAL_CLEANUP_RESERVE` 10 s (v4.2). | CD (v3-records copy) | ratified | — |
| Current rc01: TASK_LIMIT 20 min, CLEANUP_RESERVE 5 min, CLEANUP_GRACE_MS 10,000, MAX_ATTEMPTS 3, MAX_NO_PROGRESS 2, MAX_INPUT_TOKENS 32,768 | migrated `src/contracts/rc01.rs:7-22` | read 2026-10-01 | — |

## 5 · Decision points (all THRESHOLD: stays code, calibrated by measurement, JM:23)
| Site · kind · note | Jev |
|---|---|
| worker/native execute window :1531 MAX_RUN 15 min; ADAPTERS :373; exchange timeout :1300 (DP:42, DP:44, DP:45) | not-Jev |
| worker/process ProcessSpec::validate :464; drive escalation :675-681 TERM_GRACE 5 s, CLEANUP 10 s (DP:49-50) | not-Jev |
| worker/namespace stop kill :1364 literal 5 s (DP:52) | not-Jev |
| task begin_attempt :449-468 TASK_LIMIT, CLEANUP_RESERVE, MAX_ATTEMPTS, MAX_NO_PROGRESS (DP:56) | not-Jev |
| app/dispatcher 15 min/8 tasks; 96/256 GiB; 128 GiB (DP:79); control_socket 100/s, burst 32, cap 8 (DP:99); actions/tasks MAX_QUERY_BYTES (DP:101); acquisition bounds (DP:104) | not-Jev |
| cohort 64 / 8 (DP:110); context MAX_* :329-361 (DP:113, **E16**); numerical 1e-12, 8·ε (DP:123); bash deadline default :264 (DP:132) | not-Jev |

## 6 · Migrated inputs
| Path | Note |
|---|---|
| `src/contracts/rc01.rs` | seed of the module; 0 anchor lines |
| `src/contracts/control.rs` consts `:23-31`, `:771` | MAX_FRAME_BYTES, MAX_DEPTH, MAX_DEADLINE_AHEAD_MS, PROTOCOL (`hee3.control` → `hee4`, V4-11 DEC:31), MAX_PAGE_LIMIT |
| `integrations/bash/hee3` | consumer to be generated from the emitted schema (AR:26) |
| tests: `tests/t01_contracts.rs` | carries constant-vs-contract-text checks shape (EX-20) |

## 7 · Quality guard
| id | why here |
|---|---|
| AP-01 | the whole purpose: one definition site per bound |
| AP-04 | a bound is only a bound at acquisition; derived sets need their own |
| AP-14 | consumers refuse over-bound input by name with both numbers; no clamp |
| AP-19 | `cargo-mutants` never mutates a `const`'s digits — pin the resolved VALUE against an independent source (contract text) |
| AP-28 | no test-count literals masquerading as bounds |
| EX-18, EX-20, EX-13 | one grace constant compile-time checked; constants checked against contract text; `take(N+1)` then refuse |
| A-3, A-4, A-9 | third door on grace; re-typed frame bound; silent clamp |
| D-07 | one topic, one home |

## 8 · Interfaces
Emitted into the K0 schema that bash (`hee4` wrapper) and Pi read (UM:80, UM:151); read by K6 socket admission (UM:99-100), K1 submit (UM:109), K0h curl client (UM:169).

## 9 · Done criteria
| # | Criterion | Evidence |
|---|---|---|
| 1 | Each bound defined once | one-door census `duplicate_sites=0` at every Tier-2 (AT:150, DEC:37) |
| 2 | Every constant's value asserted against the contract text (independent source), not its own name | `constants` step shape (EX-20); planted digit swap killed by a named test |
| 3 | Bash/Pi read bounds from the schema and **refuse** >60 s | `bash_wrapper.py` case with both numbers (AR:26) |
| 4 | Every derived set (roster obs, permits, fd strings) has its own bound | refusal named per bound (UM:42) |

## 10 · Open decisions and risks
- TimeoutStopSec derives from RC01 1,200 s + measured seal time (AT:85) — the 1,200 s lives here, the seal time is UNMEASURED until P4 (AT:239).
- THRESHOLD calibration is by measurement, never Jev (JM:23).
- Which constants are "contract" (RC01) vs "safety" caps is not classified yet.

## 11 · Pull commands
```bash
cd ~/herdr-engineering-engine-v4
sed -n 39p ~/hee4-evidence/design/ULTRAMAP.md; sed -n 42p ~/hee4-evidence/design/ULTRAMAP.md
sed -n 10p ~/hee4-evidence/reference/v3-evidence-b5367bc/architecture-review-b5367bc.md; sed -n 36p ~/hee4-evidence/reference/v3-evidence-b5367bc/architecture-review-b5367bc.md
rg -n 'THRESHOLD' ~/hee4-evidence/design/DECISION_POINTS-b5367bc.md
cat migrated/v3-b5367bc/src/contracts/rc01.rs
rg -n 'pub const' migrated/v3-b5367bc/src/contracts/control.rs | head -12
rg -n 'MAX|DEFAULT_MS' migrated/v3-b5367bc/integrations/bash/hee3 | head
rg -n '### (EX-18|EX-20|EX-13|A-3|A-4|A-9)' docs/EXEMPLARS.md
```
