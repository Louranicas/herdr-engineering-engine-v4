# hee4-app · workload
> **Status: PLANNING — HOLD** (V4-0, DEC:4). No code until Luke says "start coding". This card is the stub.

## 1 · Identity
| Field | Value | Source |
|---|---|---|
| Crate | `hee4-app` | UM:79 |
| Cluster | K6 | UM:65 |
| v3 origin | `app` → `src/app/workload.rs` (evaluate / classify the class workload's producer run) | DP:84-85 |
| Status | PLANNING — HOLD | DEC:4 |

**Design section:** [K6 hee4-app › workload](obsidian://open?vault=herdr-engineering-engine-v4.vault&file=15%20Module%20Design/K6%20hee4-app%23workload): logic flow, interfaces, S-n/actions, RL/E2E, refusals, Must not, size budget. Elaboration only; this card and the authorities win. Index: [Module Design Index](obsidian://open?vault=herdr-engineering-engine-v4.vault&file=15%20Module%20Design%2F00%20-%20Module%20Design%20Index)

## 2 · Purpose, owned state, allowed dependencies
| Aspect | Content | Source |
|---|---|---|
| Purpose | Evaluate the candidate's workload run and classify its outcome (exit status, stderr) into an observation for the check | DP:84-85 |
| Owned state | None | UM:79 |
| Verdict | none: its classification is an **observation** for K4 `decide` (P4) | UM:38; DEC:33 |
| Allowed deps | K0 (observation types), K0h (spawn door output), K4 (decide inputs) via K6 | UM:65 |

## 3 · Deployment
| Phase(s) | Entry gate | Exit evidence | Held-for-Luke |
|---|---|---|---|
| P3 (one verdict authority; workload feeds it) | P2 | gate; plants | none |
| P5 fail → repair → accept scenario uses the classification | P4 | host record F07 | none |
| P9 J7 residual advisory, only if granted | tag | shadow mode | H-8, H-10, H-11 |
Feeds D6 (AT:63). Sources: AT:82, AT:84, AT:88.

## 4 · v3 basis
- **Flag:** `app` PARTIAL (MA:19). **Recommendation:** `app` REFACTOR (AR:7).

| Finding | v3 file:line | Source | Errata |
|---|---|---|---|
| any stderr byte fails the producer (JUDGMENT as a byte rule) | `workload.rs:556` | DP:85; AP-16; JM:35 | — |
| evaluate / classify | `:395`, `:556` | DP:84 | — |

## 5 · Decision points
| site · kind · note | Jev |
|---|---|
| `evaluate/classify :395/:556` · EXACT (DP:84) | — |
| `classify stderr arm :556` · JUDGMENT · any stderr byte fails (DP:85) | **J7: code first** (panic marker, exit status); **grant** residual, advisory only; the oracle keeps the verdict (JM:35) |

## 6 · Migrated inputs
None — `app` is REFACTOR (MIG:22).

## 7 · Quality guard
| id | why for THIS module |
|---|---|
| AP-16 | stderr-any-byte is a judgment hard-coded as exact |
| AP-01 | classification must not become a second verdict beside `decide` |
| AP-04 | stderr capture bounded at acquisition (K0h process caps) |
| AP-20 | test classification whole, over two fixtures differing in every field — not `contains` |
| AP-21 | fixtures from recorded runs (panic, warning, env failure), not typed |
| EX-05 | closed outcome enum, `parse → None` |
| EX-07 | pure classification over one settled `ProcessReport` |
| D-10 | if a stderr heuristic grows round by round, stop and ask what exit status/markers can refuse instead |

## 8 · Interfaces
| Kind | Item | Source |
|---|---|---|
| input | settled process report from the K0h spawn door | UM:72 |
| output | observation → K4 `decide` | UM:77 |
| held | `Advised<Choice{benign warning, panic, test failure, environment, other}>` | JM:35 |

## 9 · Done criteria
| # | criterion | evidence / read-back |
|---|---|---|
| 1 | Classification is code-first (exit status, panic marker); stderr-only outcome is a named class, not a silent fail | test table over recorded fixtures |
| 2 | Workload never constructs a verdict | plant "workload returns Pass" killed by a named test (as AT:82's adapter plant) |
| 3 | Scoped mutants on classify with `CARGO_TARGET_DIR` unset | `mutants caught= survivors=` (AT:149) |

## 10 · Open decisions and risks
- Which stderr classes count as benign is UNMEASURED; decide from labelled runs, not by reading (JM:74).
- J7 needs H-8 grant (AT:192).
- Resolved, not open *(rev 2026-10-01 funnel audit)*: the register rows naming this module are all RESOLVED, ratified under delegation inside H-27's range: DC-39 (V4-61): `Class` port: home and seam shape; DC-45 (V4-66): Who consumes Jev advice for J1/J2/J3/J7. The decision text is the V4 row in `plan/DECISIONS.md`; `hee4db highway --dc <DC-nn>` shows the row.

## 11 · Pull commands
```bash
V4=~/herdr-engineering-engine-v4; EV=~/hee4-evidence; R=$EV/reference/v3-evidence-b5367bc
sed -n '84,85p' $EV/design/DECISION_POINTS-b5367bc.md
sed -n '35p;72,75p' $R/jev-decision-map-b5367bc.md
sed -n '38p;77p' $EV/design/ULTRAMAP.md; sed -n '77p;79p' $EV/design/DEPLOYMENT_ATLAS.md
rg -n '^\| \*\*AP-(01|04|16|20|21)\*\*' $V4/docs/ANTIPATTERNS.md
```
