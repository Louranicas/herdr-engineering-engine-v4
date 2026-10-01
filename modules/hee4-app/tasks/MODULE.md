# hee4-app · tasks
> **Status: PLANNING — HOLD** (V4-0, DEC:4). No code until Luke says "start coding". This card is the stub.

## 1 · Identity
| Field | Value | Source |
|---|---|---|
| Crate | `hee4-app` | UM:79 |
| Cluster | K6 (port impl, thin) | UM:108 |
| v3 origin | `app` → `src/app/tasks.rs` (`StoreTasks`) | IM:20; UM:108, UM:121 |
| Status | PLANNING — HOLD | DEC:4 |

**Design section:** [K6 hee4-app › tasks](obsidian://open?vault=herdr-engineering-engine-v4.vault&file=15%20Module%20Design/K6%20hee4-app%23tasks): logic flow, interfaces, S-n/actions, RL/E2E, refusals, Must not, size budget. Elaboration only; this card and the authorities win. Index: [Module Design Index](obsidian://open?vault=herdr-engineering-engine-v4.vault&file=15%20Module%20Design%2F00%20-%20Module%20Design%20Index)

## 2 · Purpose, owned state, allowed dependencies
- **Purpose:** K6 implementation of the task owner port: `StoreTasks::submit/get/list/cancel/resolve`, evidence view, `delivery_of`, and `task.preview` → app routing → route (UM:108, UM:121-124; IM:19-20).
- **Owned state:** none; writes go through K1 (`Store::submit` via the idempotent-op primitive, state from `transition(∅, Admit)`) (UM:109).
- **Allowed deps:** any (K6). Must stay thin: parsing is K1 `task::control` (UM:107).
- **v4 change:** `delivery_of` reads a **real** delivered count so "pending forever" ends (UM:124); `task.get` reads `TaskState` enum, not String (UM:122); admission writes `serve_cgroup` for D9 (UM:194; V4-15).

## 3 · Deployment
| Phase(s) | Entry gate | Exit evidence | Held-for-Luke |
|---|---|---|---|
| P2 (task.submit/get/list/cancel/resolve, F15 shape) | P1 | L2 +1-2 (AT:81) | — |
| P5 (cancel/list/resolve through socket; delivered count real) | P4 | gate F15 (AT:84) | — |
| P7 (first real task; `used` count) | P6 | D6/D9 read-backs (AT:86) | H-4 (real root) |
Feeds D6 (AT:63), D9 (AT:66).

## 4 · v3 basis
- **Flag:** app PARTIAL (MA:19). **Recommendation:** REFACTOR (AR:7).

| Finding | v3 file:line | Source |
|---|---|---|
| handlers exist; only socket proof missing (F15) | tasks.rs:730/795/850 | RT:34, RT:45 |
| submit / get / list | tasks.rs:571 / :642,:730 | UM:108, UM:121 |
| evidence view; `delivery_of` | tasks.rs:415,555; :701,971 | UM:123-124 |
| task.get `delivery` reads "pending" forever (outbox undelivered) | — | AR:33 |
| t28_socket has no cancel/list/resolve case | — | RT:25 |

## 5 · Decision points
| site · kind · note | Jev |
|---|---|
| resume_key :197 · resolve_fault :266 · EXACT (DP:102) | not-Jev |

## 6 · Migrated inputs
None — app is REFACTOR (MIG:22). Its K1 counterpart (`task/control.rs` parse) is migrated with `task` (MIG:10).

## 7 · Quality guard
| id | why for THIS module |
|---|---|
| AP-02 | read state as `TaskState`, never compare strings (A-1) |
| AP-13 | `delivery` must reflect a real reader, not "pending" forever |
| AP-19 | test several submits, rows 2+ asserted whole (generation "1" trap, A-2) |
| AP-20 | assert whole `task.get` lines, not `contains` |
| EX-05 | enum parse at the boundary |
| EX-14 | `Generation::FIRST`, asserted off the origin |
| A-1, A-2 | state as text; generation literal "1" |
| D-16 | D9 `used` counts only tasks through `hee4.service` |

## 8 · Interfaces
Serves task.preview, submit, get, list, cancel, resolve (UM:157). Uses K1 store ports, app routing (preview), outbox delivered count (UM:125).

## 9 · Done criteria
| # | criterion | evidence |
|---|---|---|
| 1 | cancel before dispatch, list with cursor, resolve by operator through socket | F15 gate L2 (RT:45) |
| 2 | Accepted task read back | `hee4 task.get <id>` → `state=accepted` (AT:63) |
| 3 | `serve_cgroup` written at admission, D9 filter works | `task.list state=accepted` filtered to `/hee4.service` (AT:66) |
| 4 | `delivery` not pending after ack | test with events.subscribe ack (UM:124) |
| 5 | Mutation | scoped mutants on delivery_of/resolve_fault, `CARGO_TARGET_DIR` unset (AT:132) |

## 10 · Open decisions and risks
- `default-release-tests-main` ran 149-155 s against 180 s in v3 — socket tests must record wall time (AT:98; RT:70).

## 11 · Pull commands
```bash
rg -n 'app/tasks|tasks.rs' ~/hee4-evidence/design/ULTRAMAP.md ~/hee4-evidence/design/DECISION_POINTS-b5367bc.md
rg -n 'F15|S3' ~/hee4-evidence/reference/v3-evidence-b5367bc/route-to-v010-b5367bc.md
sed -n 58,61p ~/hee4-evidence/design/DEPLOYMENT_ATLAS.md
rg -n 'serve_cgroup|V4-15' ~/hee4-evidence/design/ULTRAMAP.md ~/herdr-engineering-engine-v4/plan/DECISIONS.md
```
