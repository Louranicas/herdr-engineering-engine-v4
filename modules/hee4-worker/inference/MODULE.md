# hee4-worker · inference
> **Status: PLANNING — HOLD** (V4-0, DEC:4). No code until Luke says "start coding". This card is the stub.

**Thin evidence.** The v4 sources name this member only in UM:75 — "worker (native, namespace, aggregate, inference, pi)". No reference copy (MA, AR, DP, IM) names a `worker/inference` file, site or finding. Everything below that is not cited to UM:75 is marked INTERP. Confirm the v3 file and its scope at the P3 skeleton before writing more.

## 1 · Identity
| Field | Value | Source |
|---|---|---|
| Crate | `hee4-worker` | UM:75 |
| Cluster | K2 | UM:75 |
| v3 origin | `worker` (sub-member "inference"; file not named in any v4-owned source) | UM:75; MA:7 |
| Status | PLANNING — HOLD; REFACTOR, not migrated | MIG:22 |

**Design section:** [K2 hee4-worker › inference](obsidian://open?vault=herdr-engineering-engine-v4.vault&file=15%20Module%20Design/K2%20hee4-worker%23inference): logic flow, interfaces, S-n/actions, RL/E2E, refusals, Must not, size budget. Elaboration only; this card and the authorities win. Index: [Module Design Index](obsidian://open?vault=herdr-engineering-engine-v4.vault&file=15%20Module%20Design%2F00%20-%20Module%20Design%20Index)

## 2 · Purpose, owned state, allowed dependencies
| Aspect | v4 shape | Source |
|---|---|---|
| Purpose | INTERP: provider-neutral candidate execution (the "candidate execution" UM assigns K2) that native implements | UM:75 "Owns eligibility (one door), routing policy, candidate execution" |
| Owned state | none durable | UM:75 |
| Allowed deps | `hee4-contracts`, `hee4-host` | UM:62-66 |

## 3 · Deployment
| Phase(s) | Entry gate | Exit evidence | Held-for-Luke |
|---|---|---|---|
| P3 (with native) | P2 | gate | none (AT:82) |
D-rows fed: D6 via native (AT:63).

## 4 · v3 basis
Worker flag **FINISHED** (assertion; MA:7, ER §2 row 1); recommendation **REFACTOR** (AR:10). No inference-specific finding exists in the reference copies (grep `inference` over `~/hee4-evidence/reference/`: 0 hits, measured 2026-10-01).

## 5 · Decision points
None in DP (grep `inference` over DP: 0 hits). The candidate-loop JUDGMENT sites J1-J3 sit in K6 `candidates` (JM:29-31).

## 6 · Migrated inputs
None (MIG:22).

## 7 · Quality guard
| id | why for THIS module |
|---|---|
| AP-09 | a member named in a plan but with no finding risks becoming dead pub API |
| AP-01 | do not re-spell the run limit here (AR:10 — spelled 4×) |
| D-05 | do not create the module unless a flow needs it (apparatus ratio) |
| D-03 | skeleton first: decide the file by compiling it, not by prose |

## 8 · Interfaces
None of its own; reached through native (UM:114).

## 9 · Done criteria
| # | criterion | evidence / read-back |
|---|---|---|
| 1 | Its v3 file and scope confirmed, or the member retired from UM:75 with a decision row | `plan/DECISIONS.md` row |
| 2 | If kept: has a production caller | `habitat-unused-pub` clean |

## 10 · Open decisions and risks
- OPEN: does "inference" denote a v3 file (`worker/inference.rs`?) or a planned seam? UNMEASURED from v4-owned sources.

## 11 · Pull commands
```bash
grep -n 'inference' ~/hee4-evidence/design/*.md
grep -rn 'inference' ~/hee4-evidence/reference/ ~/herdr-engineering-engine-v4/migrated/v3-b5367bc/MIGRATION.md
sed -n 75p ~/hee4-evidence/design/ULTRAMAP.md
```
