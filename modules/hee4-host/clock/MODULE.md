# hee4-host · clock
> **Status: PLANNING — HOLD** (V4-0, DEC:4). No code until Luke says "start coding". This card is the stub.

## 1 · Identity
| Field | Value | Source |
|---|---|---|
| Crate | `hee4-host` (the `Clock` impl; the trait is a port in K0 per P6) | UM:72; UM:40 |
| Cluster | K0h | UM:72 |
| v3 origin | the dispatcher's Clock seam (`app`), `task::LoopGuard::observe_clock` monotonic check | AR:7; DP:55 |
| Status | PLANNING — HOLD | DEC:4 |

**Design section:** [K0h hee4-host › clock](obsidian://open?vault=herdr-engineering-engine-v4.vault&file=15%20Module%20Design/K0h%20hee4-host%23clock): logic flow, interfaces, S-n/actions, RL/E2E, refusals, Must not, size budget. Elaboration only; this card and the authorities win. Index: [Module Design Index](obsidian://open?vault=herdr-engineering-engine-v4.vault&file=15%20Module%20Design%2F00%20-%20Module%20Design%20Index)

## 2 · Purpose, owned state, allowed dependencies
| Item | Value | Source |
|---|---|---|
| Purpose | One source of wall and monotonic time for the engine; every decision takes time as a value (F95), so policy is provable by argument and tests use logical clocks | UM:41 (P7); REQ:14 (rank 6) |
| Owned state | None | UM:72 |
| Allowed deps | K0 (trait) | UM:62 |
| Threading | "Clock threaded through" the app (was: seam stopped at the dispatcher) | UM:79; AR:7 |

## 3 · Deployment
| Phase | Entry gate | Exit evidence | Held |
|---|---|---|---|
| P1 | P0 | `serve` + dispatcher take the Clock port from K6 wiring | none — AT:80 |
| P2 | P1 | backup-due and restore RTO decisions take time as values (15 min rule) | none — AT:81, AT:62 |
| P3 | P2 | task deadlines/no-progress guards read the clock port only | none — AT:82 |
Feeds D5 (backup age >15 min, AT:62) and D7 (RTO measured, AT:64).

## 4 · v3 basis
- **Flag / recommendation:** n/a as a module; app REFACTOR — "the Clock seam stops at the dispatcher… thread Clock through" (AR:7).

| Finding | v3 file:line | Source | Errata |
|---|---|---|---|
| Clock seam stops at the dispatcher | — | AR:7; AP-06 | — |
| Pure decisions over time already proven: `backup_due :245`, `headroom :289/:316`, `budget_decision :299` | `dispatcher.rs:245-316` | UM:41; DP:79 | — |
| Monotonic guard | `task observe_clock :529` | DP:55 | — |
| A test took 1.000605 s against a 1 s budget (load-sensitive wall clock) | `tests/t28_runtime.rs:1032` | AP-06; PL:43 (L7) | — |

## 5 · Decision points
| Site · kind · note | Jev |
|---|---|
| task `LoopGuard::new :420 · record_reconciled_attempt :502 · observe_clock :529` · EXACT · mask, no-progress, monotonic (DP:55) | not-Jev |
| app/dispatcher `backup_due :245 · headroom :289/:316 · budget_decision :299` · THRESHOLD · 15 min/8 tasks (DP:79) | not-Jev |

## 6 · Migrated inputs
None directly (app is REFACTOR, MIG:22). Consumers in the migrated set: `src/task.rs`, `src/task/driver.rs` (deadline checks the guard owns; AR:20) — they must read the port, not `Instant::now()`.

## 7 · Quality guard
| id | why here |
|---|---|
| AP-06 | no `Instant::now()`/`SystemTime` inside a decision |
| AP-07 | a function given a deadline must not re-acquire the time to re-decide it (task driver) |
| AP-31 | tests use logical clocks; loops budgeted |
| EX-07, EX-11, EX-20 | pure step over one poll; device rule pure over values; constants vs contract text |
| D-09 | trait in K0, impl here; K1/K3 cannot depend on K0h — they receive time values |

## 8 · Interfaces
Internal only: port consumed by K6 runtime/dispatcher, K1 (via values), K2 deadlines. No external call.

## 9 · Done criteria
| # | Criterion | Evidence |
|---|---|---|
| 1 | Exactly one crate reads the system clock | grep `Instant::now\|SystemTime::now` outside `hee4-host` = 0 (compiler-decided alternative: `disallowed-methods` with a planted call asserting the lint's own diagnostic, per F130) |
| 2 | Every time-dependent policy has a test driven by a logical clock at ≥ 3 distinct instants (off the origin) | AP-19 pattern |
| 3 | Gate step wall times print `elapsed/budget` | AT:165; REQ:14 |

## 10 · Open decisions and risks
- Trait home: UM:72 names the impl in K0h but no row names the trait; P6 implies K0 (UM:40) — record at P1.
- K1 core cannot depend on K0h (UM:63): time must enter K1 as values, not a trait object.

## 11 · Pull commands
```bash
cd ~/herdr-engineering-engine-v4
sed -n 40,41p ~/hee4-evidence/design/ULTRAMAP.md; sed -n 72p ~/hee4-evidence/design/ULTRAMAP.md; sed -n 79p ~/hee4-evidence/design/ULTRAMAP.md
sed -n 7p ~/hee4-evidence/reference/v3-evidence-b5367bc/architecture-review-b5367bc.md
sed -n 55p ~/hee4-evidence/design/DECISION_POINTS-b5367bc.md; sed -n 79p ~/hee4-evidence/design/DECISION_POINTS-b5367bc.md
sed -n 14p gates/REQUIREMENTS.md
rg -n 'Instant::now|SystemTime' migrated/v3-b5367bc/src | head
rg -n '^\| \*\*AP-06' docs/ANTIPATTERNS.md | cut -c1-200
```
