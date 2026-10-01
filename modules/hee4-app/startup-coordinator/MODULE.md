# hee4-app · startup-coordinator
> **Status: PLANNING — HOLD** (V4-0, DEC:4). No code until Luke says "start coding". This card is the stub.

## 1 · Identity
| Field | Value | Source |
|---|---|---|
| Crate | `hee4-app` (owns "the startup/commission sequence") | UM:79 |
| Cluster | K6 | UM:79 |
| v3 origin | `app` → `src/app/startup.rs`, `src/app/coordinator.rs` | DP:92-94; IM:71 |
| Status | PLANNING — HOLD | DEC:4 |

**Design section:** [K6 hee4-app › startup-coordinator](obsidian://open?vault=herdr-engineering-engine-v4.vault&file=15%20Module%20Design/K6%20hee4-app%23startup-coordinator): logic flow, interfaces, S-n/actions, RL/E2E, refusals, Must not, size budget. Elaboration only; this card and the authorities win. Index: [Module Design Index](obsidian://open?vault=herdr-engineering-engine-v4.vault&file=15%20Module%20Design%2F00%20-%20Module%20Design%20Index)

## 2 · Purpose, owned state, allowed dependencies
- **Purpose:** commission a state root (`hee4 commission`, 0700/0600, refuses an existing root), open the active generation (`Manifest::active`), run startup reconcile (recovery to `complete` before any dispatch), `health_of`, and the new `hee4 restore --into <dir> <backup-id>` into a fresh generation (AT:81; UM:149; DP:94).
- **Owned state:** sequencing only; the state root layout `~/.local/state/hee4/{active.json,generations/<g>/ledger.sqlite3,objects,attempts}` is K1's (UM:181). v4 state is fresh; no v3 ledger migration (D-U2, DEC:13).
- **Allowed deps:** any (K6); recovery policy stays pure in K1 (AR:28 KEEP).

## 3 · Deployment
| Phase(s) | Entry gate | Exit evidence | Held-for-Luke |
|---|---|---|---|
| P2 (commission; restore verb + recovery-to-complete) | P1 | restore drill in gate world on a disposable root (AT:81) | — |
| P6 (first real `commission` on Luke's root) | P5 + P2 | D4 read-back (AT:85) | **H-4** (AT:188) |
| P7 (G11 restore drill, RTO) | P6 | restore line with `rto_s` (AT:86) | H-7 (journal) |
Feeds D4 (AT:61), D5(b) (AT:62), D7 (AT:64).

## 4 · v3 basis
- **Flag:** app PARTIAL — commission through main (MA:19); store/recovery: no restore, `restored_from` never persisted, restore marker always None (MA:10-11).
- **Recommendation:** app REFACTOR (AR:7); recovery KEEP (AR:28).

| Finding | v3 file:line | Source | Errata |
|---|---|---|---|
| state root files | coordinator.rs:33,45,338,342 | IM:71 | — |
| frame limit literal re-typed | coordinator.rs:53 | AP-01; UM:39 | — |
| presence (locator corrected) | startup.rs:207 (was :173) | DP:26 | E16 |
| no restore verb, no retention, no prune | — | AT:39 | — |
| no real state root in v3 (T3 used disposable HOMEs) | — | AT:61 | — |

## 5 · Decision points
| site · kind · note | Jev |
|---|---|
| startup classify :113 · presence :207 · task_history :1049 · EXACT (DP:92) | not-Jev |
| OPEN_ATTEMPT_LIMIT / CLEANUP_BATCH / WORKSPACE_REMOVAL_BUDGET · THRESHOLD (DP:93) | not-Jev |
| coordinator health_of :199 · Manifest::active :98 · EXACT (DP:94) | not-Jev |

## 6 · Migrated inputs
None — app is REFACTOR (MIG:22). `recovery` (KEEP) is not migrated either; "migrate it on Luke's word" (MIG:23). *(rev 2026-10-01 ratified V4-59)* recovery is migrated at "start coding" (DC-27). *(rev 2026-10-01 ratified V4-58, DC-42)* `serve_cgroup` is read **once at serve start** and passed to admission as a value.

## 7 · Quality guard
| id | why for THIS module |
|---|---|
| AP-01 | frame limit `1_048_576` at coordinator.rs:53 — read K0 (A-4) |
| AP-49 | commission/restore must read state back (modes, `user_version`, object parity) |
| AP-31 | open-attempt cleanup loops carry budgets |
| AP-06 | presence/classify pure over listings, I/O thin |
| EX-04 | pure recovery policy with named rules — call it, don't re-derive |
| EX-16 | partial → fsync → rehash → rename for restore landing |
| A-4 | frame bound retyped outside its door |
| D-16 | "deployed" needs real commission (D4) |

## 8 · Interfaces
CLI `hee4 commission <s>`, `hee4 serve`, `hee4 restore --into <dir> <backup-id>` (UM:148-149). `health` reports `recovery=complete database=ready` (AT:60).

## 9 · Done criteria
| # | criterion | evidence |
|---|---|---|
| 1 | commission exit 0 once, refuses second time by name; modes 0700/0600; `user_version` = CURRENT | `sqlite3 mode=ro PRAGMA user_version`; `stat` (AT:61) |
| 2 | restore → `health recovery=complete` | `restore backup=<id> ledger=<d> objects=<n>/<n> rto_s=<t> verdict=PASS` (AT:62) |
| 3 | RTO ≤ 10 min | measured P2 gate world + P7 host (AT:241) |
| 4 | `restored_from` persisted | ledger row read back (MA:10) |
| 5 | Mutation | scoped mutants on startup classify, `CARGO_TARGET_DIR` unset (AT:132) |

## 10 · Open decisions and risks
- H-4 re-grant needed for v4 custody (AT:188). Upgrade verb + migration ownership (AT:229).

## 11 · Pull commands
```bash
rg -n 'commission|restore' ~/hee4-evidence/design/ULTRAMAP.md ~/hee4-evidence/design/DEPLOYMENT_ATLAS.md
rg -n 'app/startup|app/coordinator|startup.rs' ~/hee4-evidence/design/DECISION_POINTS-b5367bc.md
sed -n 10,11p ~/hee4-evidence/reference/v3-evidence-b5367bc/module-audit-b5367bc.md
rg -n 'D-U2|V4-8|V4-15' ~/herdr-engineering-engine-v4/plan/DECISIONS.md
```
