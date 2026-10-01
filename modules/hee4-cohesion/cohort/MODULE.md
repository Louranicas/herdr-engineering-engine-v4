# hee4-cohesion · cohort
> **Status: PLANNING — HOLD** (V4-0, DEC:4). No code until Luke says "start coding". This card is the stub.

## 1 · Identity
| Field | Value | Source |
|---|---|---|
| Crate | `hee4-cohesion` | UM:76 |
| Cluster | K3 | UM:76 |
| v3 origin | `cohort` (`src/cohort.rs`) | MA:24; AR:12 |
| Status | PLANNING — HOLD | DEC:4 |

**Design section:** [K3 hee4-cohesion › cohort (v4.2)](obsidian://open?vault=herdr-engineering-engine-v4.vault&file=15%20Module%20Design/K3%20hee4-cohesion%23cohort%20%28v4.2%29): logic flow, interfaces, S-n/actions, RL/E2E, refusals, Must not, size budget. Elaboration only; this card and the authorities win. Index: [Module Design Index](obsidian://open?vault=herdr-engineering-engine-v4.vault&file=15%20Module%20Design%2F00%20-%20Module%20Design%20Index)

## 2 · Purpose, owned state, allowed dependencies
| Item | Value | Source |
|---|---|---|
| Purpose | Thread/cohort policy: assignment, conflicts, reports, rebrief, and **the one Join/Blocked vocabulary** (shared with workflows) | UM:76; UM:218 |
| Owned state | none; durable rows `cohort_threads`, `cohort_reports`, `dissent_log` (append-only) owned by K1 via app | UM:204; AR:12 |
| Allowed deps | `hee4-contracts` | UM:63 |

## 3 · Deployment
| Phase(s) | Entry gate | Exit evidence | Held-for-Luke |
|---|---|---|---|
| P9 milestone-2 (cohort flows; `thread.get/list` v4.2) | tag pushed | per-slice gates; l2 moves (AT:88) | J8 advice needs H-8 (AT:192) |
Deferred to v4.2 with trigger "MA NOT FINISHED rows 17-22" (UM:269); needs K1 durable owner (UM:162).

## 4 · v3 basis
Flag **NOT FINISHED** — in-memory refusals only; no allocation, lifecycle, persistence, paging or caller (MA:24). Recommendation **REFACTOR** — no durable owner; thread identity ≠ store attempt identity; dependencies recorded, not enforced; dissent reason lost on rebrief (AR:12).
| Finding | v3 file:line | Source | Errata |
|---|---|---|---|
| Re-key threads onto attempt identity | — | AR:12; UM:76 | — |
| `Report` enum with non-empty dissent; append-only dissent log | — | AR:12; UM:204 | — |
| Julia re-implements cohort::join | `Cohesion.jl:657` | DP:124; AR:15 | — |
| Two join vocabularies (cohort vs workflows) | — | AR:14; UM:218 | — |

## 5 · Decision points
| Site · kind · note | Jev |
|---|---|
| assign :742 · conflicts_with :544 · report :843 · rebrief guard :889 · join :954 — EXACT (DP:109) | not-Jev |
| bounds :742 · rebrief limit :882 — THRESHOLD 64 / 8 (DP:110) | not-Jev |
| reported outcome and dissent :843→:954 — JUDGMENT, self-label trusted, evidence unread (DP:111) | **J8: Jev (grant), advisory**; never closes an obligation (JM:36) |

## 6 · Migrated inputs
None — REFACTOR (MIG:22). No v3 test named in MIG for cohort.

## 7 · Quality guard
| id | why for THIS module |
|---|---|
| AP-01 | a second join engine in workflows and a third in Julia |
| AP-03 | thread lifecycle must be a phase enum, not flags |
| AP-13 | dissent written must have a reader (thread.get) |
| AP-16 | outcome/dissent is JUDGMENT — declare it; labelled fixtures before tuning |
| EX-05 | one spelling per Join/Blocked variant |
| EX-04 | join as pure policy with named rules |
| D-09 | no import of K1; rows via app |
| D-11 | milestone-2 stack must move l2 |

## 8 · Interfaces
| Kind | Item | Source |
|---|---|---|
| Actions | `thread.get`, `thread.list` — v4.2 (v3 refused C06) | UM:162; IM:21 (:421/:425→:420/:424, E10) |
| Jev (held) | J8 consumer via `Advised<T>` port | UM:133; JM:81 |

## 9 · Done criteria
| # | Criterion | Evidence / read-back |
|---|---|---|
| 1 | Threads keyed by attempt identity; restore from store | restart test: thread state rebuilt from K1 rows |
| 2 | Join/Blocked defined once, emitted as schema; workflows imports it | grep one definition; schema round-trip |
| 3 | Dissent non-empty and appended, never overwritten on rebrief | test rebriefs 3× and asserts all 3 dissent rows whole |
| 4 | Cancelled dependency blocks dependents | case from AR:14 finding |
| 5 | Plants / mutation | planted dependency-unblock killed by named test; scoped `cargo mutants` survivors named |

## 10 · Open decisions and risks
- Scope v4.2; migration numbering for new tables (UM:204) owned by store.
- J8 Jev advice held (H-8, H-10, H-11).
- Resolved, not open *(rev 2026-10-01 funnel audit)*: the register rows naming this module are all RESOLVED, ratified under delegation inside H-27's range: DC-14 (V4-56): Home of shared vocabularies (Join/Blocked, Omission): K0 (UM §5b) vs K3. The decision text is the V4 row in `plan/DECISIONS.md`; `hee4db highway --dc <DC-nn>` shows the row.

## 11 · Pull commands
```bash
E=~/hee4-evidence; R=~/herdr-engineering-engine-v4
rg -n 'cohort|thread|dissent|Join' $E/design/ULTRAMAP.md
rg -n 'cohort' $E/design/DECISION_POINTS-b5367bc.md $E/reference/v3-evidence-b5367bc/*.md
rg -n 'J8' $E/reference/v3-evidence-b5367bc/jev-decision-map-b5367bc.md
rg -n 'thread' $E/reference/ERRATA-v3-evidence-b5367bc.md
```
