# hee4-cohesion · workflows
> **Status: PLANNING — HOLD** (V4-0, DEC:4). No code until Luke says "start coding". This card is the stub.

## 1 · Identity
| Field | Value | Source |
|---|---|---|
| Crate | `hee4-cohesion` | UM:76 |
| Cluster | K3 | UM:76 |
| v3 origin | `workflows` (v3 top-level `workflows/`, incl. `validate_procedure.py`) — not under `src/` | MA:16; AR:14; JM:41 |
| Status | PLANNING — HOLD | DEC:4 |

**Design section:** [K3 hee4-cohesion › workflows (v4.2)](obsidian://open?vault=herdr-engineering-engine-v4.vault&file=15%20Module%20Design/K3%20hee4-cohesion%23workflows%20%28v4.2%29): logic flow, interfaces, S-n/actions, RL/E2E, refusals, Must not, size budget. Elaboration only; this card and the authorities win. Index: [Module Design Index](obsidian://open?vault=herdr-engineering-engine-v4.vault&file=15%20Module%20Design%2F00%20-%20Module%20Design%20Index)

## 2 · Purpose, owned state, allowed dependencies
| Item | Value | Source |
|---|---|---|
| Purpose | Procedure validation and step dispatch/reconcile, using **cohort's one Join/Blocked vocabulary** (no second join engine) | UM:76; UM:256 |
| Owned state | none; `workflow_steps` table owned by K1 (v4.2) | UM:205 |
| Allowed deps | `hee4-contracts` | UM:63 |
| v3 edge | workflows → actions (anchors.json) | E3 |

## 3 · Deployment
| Phase(s) | Entry gate | Exit evidence | Held-for-Luke |
|---|---|---|---|
| P9 milestone-2 (workflow steps v4.2) | tag | per-slice gates (AT:88) | J13 advice needs H-8 (AT:192) |
Deferred to v4.2 (UM:269).

## 4 · v3 basis
Flag **PARTIAL** — refusal sweep and dispatch-by-name met, F19 subset through the engine; `verified` still a bool (MA:16). Recommendation **REFACTOR** — second join engine; no durable procedure state; retry declared, never applied; cancelled dependency unblocks dependents (AR:14).
| Finding | v3 file:line | Source | Errata |
|---|---|---|---|
| Second join vocabulary | — | AR:14; UM:256 | — |
| `retry` declared, never applied → **dropped** until a flow needs it | — | UM:261; DP:129 | — |
| Cancelled dependency unblocks dependents | — | AR:14 | — |
| `verified` a bool from outside | `validate_procedure.py:269` | JM:41; MA:16 | — |

## 5 · Decision points
| Site · kind · note | Jev |
|---|---|
| validate :116 · dispatch/reconcile :335/:449 — EXACT (DP:128) | not-Jev |
| resume ready set :241 · observe error map :377 — THRESHOLD, retry_on ignored (DP:129) | not-Jev |
| join verified :269 — JUDGMENT, free-text criteria judged outside (DP:130) | **J13: Jev (grant), advisory per criterion**; acceptance authority stays with the verifier (JM:41) |

## 6 · Migrated inputs
None — REFACTOR (MIG:22). `skills` (migrated) owns the `_shape()` it must share (MIG:17).

## 7 · Quality guard
| id | why for THIS module |
|---|---|
| AP-01 | second join engine; second shape check |
| AP-13 | declared `retry` field with no reader — dropped |
| AP-02 | `verified: bool` must become a typed outcome |
| AP-16 | free-text criteria are JUDGMENT (J13) |
| EX-04 | pure step policy with named rules |
| EX-05 | Join/Blocked one spelling |
| D-08 | no reader, no row: every step record read by thread/workflow views |
| D-09 | imports cohort's vocabulary within K3 only |

## 8 · Interfaces
| Kind | Item | Source |
|---|---|---|
| Consumes | Join/Blocked from cohort (same crate); step rows via K6 | UM:76; UM:205 |
| Jev (held) | J13 consumer | JM:81 |

## 9 · Done criteria
| # | Criterion | Evidence / read-back |
|---|---|---|
| 1 | No join vocabulary defined in workflows | grep: 0 definitions; imports cohort's |
| 2 | Cancelled dependency blocks dependents | named test (AR:14 case) |
| 3 | Step records persisted (`workflow_steps`) and resumed after restart | restart test |
| 4 | `retry` absent from schema | schema diff (UM:261) |
| 5 | Plants / mutation | planted unblock-on-cancel killed by named test; scoped mutants named |

## 10 · Open decisions and risks
- Python vs Rust home for the validator not fixed by UM (only "join vocabulary" in K3, UM:76) — decide at slice, record in DEC.
- J13 held (H-8).

## 11 · Pull commands
```bash
E=~/hee4-evidence; R=~/herdr-engineering-engine-v4
rg -n 'workflow' $E/design/ULTRAMAP.md $E/design/DECISION_POINTS-b5367bc.md
rg -n 'workflows' $E/reference/v3-evidence-b5367bc/*.md $E/reference/ERRATA-v3-evidence-b5367bc.md
rg -n 'J13' $E/reference/v3-evidence-b5367bc/jev-decision-map-b5367bc.md
rg -n '_shape' $R/migrated/v3-b5367bc/skills/*.py
```
