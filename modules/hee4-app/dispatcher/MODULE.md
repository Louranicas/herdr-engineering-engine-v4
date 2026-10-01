# hee4-app · dispatcher
> **Status: PLANNING — HOLD** (V4-0, DEC:4). No code until Luke says "start coding". This card is the stub.

## 1 · Identity
| Field | Value | Source |
|---|---|---|
| Crate | `hee4-app` | UM:79 |
| Cluster | K6 | UM:79 |
| v3 origin | `app` → `src/app/dispatcher.rs` | DP:79-80; V2:36 |
| Status | PLANNING — HOLD | DEC:4 |

**Design section:** [K6 hee4-app › dispatcher](obsidian://open?vault=herdr-engineering-engine-v4.vault&file=15%20Module%20Design/K6%20hee4-app%23dispatcher): logic flow, interfaces, S-n/actions, RL/E2E, refusals, Must not, size budget. Elaboration only; this card and the authorities win. Index: [Module Design Index](obsidian://open?vault=herdr-engineering-engine-v4.vault&file=15%20Module%20Design%2F00%20-%20Module%20Design%20Index)

## 2 · Purpose, owned state, allowed dependencies
- **Purpose:** wait for dispatchable tasks and admit dispatch: `classify` into 3 steps; pure decisions `backup_due`, `headroom`, `budget_decision`, `headroom_decision` (UM:41, UM:284); drain on SIGTERM waits for the in-flight attempt (AT:228).
- **Owned state:** none durable. Reads K1 `next_dispatchable` (Dispatchable carries a typed `TaskState`, UM:111). Backups run **inside `serve`** at dispatch admission, not a timer (V4-6, DEC:21; AT:40). *(rev 2026-10-01 ratified V4-58, DC-22)* The triggers are the one RC01 "Backup freshness" sentence (quoted in ATLAS D5(a)): freshness ≤ 15 min before any dispatch; after a task once freshness has expired; at every batch boundary (≤ 8 tasks); and before an upgrade. There is no daemon.
- **Allowed deps:** any crate (K6) (UM:65). Policy stays pure (P7, UM:41); I/O shell thin.
- **v4 addition:** wire `evaluate_fallback` (B15) — v4.1 (UM:75, UM:268; RT:53).

## 3 · Deployment
| Phase(s) | Entry gate | Exit evidence | Held-for-Luke |
|---|---|---|---|
| P2 (backup at start + before dispatch whose last backup > 15 min; 128 GiB budget fail-closed) | P1 | gate; restore drill in gate world (AT:81) | H-4 (real root custody, P6) |
| P3 (budget Ledger is the single charge door; SIGHUP) | P2 | plants per rule (AT:82) | — |
| P5 (dispatch → fail → repair → accept, F07) | P4 | host record F07 (AT:84) | — |
| P9 (B15 fallback in dispatch) | tag | l2 +2 with C01 (RT:53) | — |
Feeds D5(a) (AT:62), D6 (AT:63).

## 4 · v3 basis
- **Flag:** app PARTIAL (MA:19). **Recommendation:** REFACTOR (AR:7) — the Clock seam stops at the dispatcher; main holds a duplicated Dispatcher literal.

| Finding | v3 file:line | Source |
|---|---|---|
| Four pure decisions (P7 exemplars) | dispatcher.rs:245,289,299,316 | UM:41; V2:36 |
| `classify` into 3 steps — REUSE | dispatcher.rs:689 | UM:284 |
| Duplicated `Dispatcher` literal in root | main.rs:867/:880 | AP-08; AR:7 |
| Constants checked against contract text | dispatcher.rs:1506-1540 | EX-20 |
| dispatcher waits | dispatcher.rs:896 | UM:110 |
| `evaluate_fallback` test-only callers | route.rs:2074 | RT:13; UM:75 |

## 5 · Decision points
| site · kind · note | Jev |
|---|---|
| backup_due :245 · headroom :289/:316 · budget_decision :299 · THRESHOLD · 15 min/8 tasks; 96/256 GiB; 128 GiB (DP:79) | not-Jev (THRESHOLD, JM:23) |
| ensure_fresh :540 · classify :689 · open_refused :820 · EXACT (DP:80) | not-Jev |

## 6 · Migrated inputs
None — app is REFACTOR (MIG:22). Shape to imitate: EX-20 (dispatcher.rs:1506-1540).

## 7 · Quality guard
| id | why for THIS module |
|---|---|
| AP-06 | backup/headroom decisions must stay pure over values; Clock threaded, not read (AR:7) |
| AP-01 | thresholds (15 min, 96/256 GiB, 128 GiB) live once in K0 `rc01` (UM:39) |
| AP-19 | constants must be pinned against contract text, not their own names |
| AP-31 | drain wait must carry a budget (AT:228) |
| AP-08 | no second Dispatcher literal in `main` |
| EX-20 | constants checked against contract text — keep this test shape |
| EX-07 | pure decision per poll |
| EX-11 | device rule pure over values (backup target decision it consumes) |
| D-09 | dispatcher composes clusters only via ports |
| D-01 | each dispatch slice names its l2 delta |

## 8 · Interfaces
- Uses: K1 `next_dispatchable`, budget charge door (UM:74), backup-target decision (K6), route/roster via app routing (fallback v4.1).
- Serves: nothing on the socket; drives runtime lifecycle.
- Files: backups to `/var/mnt/STORAGE-10TB/hee4-backups/` on a separate disk (AT:85, AT:226).

## 9 · Done criteria
| # | criterion | evidence |
|---|---|---|
| 1 | Backup at start + before any dispatch > 15 min since last | `ls` backup dir: manifest-last, `objects=<n>` with bound (AT:62 D5a) |
| 2 | 128 GiB budget refused fail-closed at admission | gate test with both numbers (AT:226) |
| 3 | Decisions pure, known answers from contract text | EX-20-style test; mutants on the four fns killed |
| 4 | Drain bounded; restart → `health recovery=complete` | read-back after restart (AT:228) |
| 5 | Mutation | scoped `cargo mutants`, `CARGO_TARGET_DIR` unset (AT:132); plants `--cap-lints=warn` with named killer (AT:149) |

## 10 · Open decisions and risks
- Engine-state backup timer would deviate from RC01 — H-12 (AT:197); v4 keeps in-serve (V4-6).
- Backup object ceiling (backup/2) wall time UNMEASURED → P2 (AT:242).

## 11 · Pull commands
```bash
rg -n 'dispatcher' ~/hee4-evidence/design/ULTRAMAP.md ~/hee4-evidence/design/DEPLOYMENT_ATLAS.md
rg -n 'app/dispatcher' ~/hee4-evidence/design/DECISION_POINTS-b5367bc.md
sed -n '/### EX-20/,/### /p' ~/herdr-engineering-engine-v4/docs/EXEMPLARS.md
rg -n 'V4-6' ~/herdr-engineering-engine-v4/plan/DECISIONS.md
rg -n 'S11' ~/hee4-evidence/reference/v3-evidence-b5367bc/route-to-v010-b5367bc.md
```
