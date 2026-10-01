# hee4-host · spawn
> **Status: PLANNING — HOLD** (V4-0, DEC:4). No code until Luke says "start coding". This card is the stub.

## 1 · Identity
| Field | Value | Source |
|---|---|---|
| Crate | `hee4-host` | UM:72 |
| Cluster | K0h | UM:57, UM:72 |
| v3 origin | `worker::process` — the one spawn door `process.rs:547` (settle, limits, pidfd custody, TERM/KILL) | IM:62; UM:72 |
| Status | PLANNING — HOLD | DEC:4 |

**Design section:** [K0h hee4-host › spawn](obsidian://open?vault=herdr-engineering-engine-v4.vault&file=15%20Module%20Design/K0h%20hee4-host%23spawn): logic flow, interfaces, S-n/actions, RL/E2E, refusals, Must not, size budget. Elaboration only; this card and the authorities win. Index: [Module Design Index](obsidian://open?vault=herdr-engineering-engine-v4.vault&file=15%20Module%20Design%2F00%20-%20Module%20Design%20Index)

## 2 · Purpose, owned state, allowed dependencies
| Item | Value | Source |
|---|---|---|
| Purpose | **The only spawn** in the workspace (P13): every child process (bwrap + shim, systemd-run, busctl, curl, julia) goes through one door | UM:47, UM:72; IM:53-63 |
| Owned state | None durable; owns the process-lifecycle policy (settle step, escalation, census) | UM:72 |
| Allowed deps | K0 only | UM:62, UM:72 |
| Dependants | K2 worker, K4 evidence, K5 habitat, K6 app | UM:62, UM:65 |
| Absorbs | `ProcessReport::settled()` + one hasher (copies in numerical and service); K0h is their one home, so K4 numerical can reach them (*rev 2026-10-01 ratified V4-55, DC-16*) | UM:72; AR:23 |
| Removes | numerical→worker and service→worker edges (they existed only to reach this door) | UM:47, UM:57; CMAP:12-13 |

## 3 · Deployment
| Phase | Entry gate | Exit evidence | Held |
|---|---|---|---|
| P0 | H-5 | skeleton; planted `use hee4_worker` from K4 fails E0432/E0433 | H-5 — AT:79, UM:84 |
| P3 | P2 | "K0h: one spawn door"; SIGHUP handled like SIGTERM; plants per rule | none — AT:82, AT:135 |
| P4 | P3 + Tier-2 | first host record: spawn path runs under the real host (glibc/toolbox build) | H-9 TH-DEV (AT:193) — AT:83 |
Feeds D6 (candidate through `native::execute`, AT:63).

## 4 · v3 basis
- **Flag:** worker FINISHED — limits door, native execute, one settle, readback not redispatch, custody (MA:7). **Caveat:** "MET means code presence only… no mutants record" (ER §2) — re-run mutation in v4 (UM:279).
- **Recommendation:** worker REFACTOR (AR:10) — for native/tools/observer; the process lifecycle is REUSE (UM:279).

| Finding | v3 file:line | Source | Errata |
|---|---|---|---|
| One spawn door | `process.rs:547` | IM:62 | — |
| Run limit spelled 4× incl. `process.rs:541` 20 min | `process.rs:541` | AR:10 | — |
| Pure settle step | `process.rs:137-166` | EX-07 | — |
| pidfd verified before retained | `process.rs:228-270` | EX-08 | — |
| TERM/KILL once, revalidated | `process.rs:353-390`, `:675-691` | EX-09 | — |
| Second door on TERM_GRACE: `namespace.rs:1364` `from_secs(5)` | `resources.rs:46-48` vs `namespace.rs:1364` | DP:52; A-3 | — |
| plan.rs spawns (rustc, mkfifo) are test-only | `plan.rs:975` | IM:63 | — |
| Orphans adopted by conmon; leaked descendants fail a gate step | — | AT:134, AT:166 | — |

## 5 · Decision points
| Site · kind · note | Jev |
|---|---|
| worker/process `settle_step :154 · observe_limits :773 · drive scan :636 · drive exit :687 · Census::step :822 · parse_stat :1025` · EXACT · lifecycle (DP:48) | not-Jev |
| worker/process `ProcessSpec::validate :464` · THRESHOLD+EXACT · argv/env/stream caps (DP:49) | not-Jev |
| worker/process `drive escalation :675-681` · THRESHOLD · TERM_GRACE 5 s, CLEANUP 10 s (DP:50) | not-Jev |

## 6 · Migrated inputs
None — worker is REFACTOR, not migrated (MIG:22). Redesign from findings; imitate EX-07/08/09/18 shapes (EXX: "imitate the shape, not the text"). Test fixture note: migrated `src/numerical/process.rs` is a *consumer* whose spawn moves here (MIG:14).

## 7 · Quality guard
| id | why here |
|---|---|
| AP-01 | one spawn door; one grace constant |
| AP-06 | settle decision pure over one poll; clock injected (see `clock` card) |
| AP-31 | every loop and exit-path await has a budget; whole-process budget |
| AP-38 | stop children by pid/pgid, never by pattern |
| AP-49 | read back process state (pidfd) before acting |
| AP-10 | no test seam in release (`#[cfg(test)]` only) |
| EX-07, EX-08, EX-09, EX-18 | the four v3 lifecycle exemplars |
| A-3 | the literal 5 s second door |
| D-09 | compiler refuses K2/K4/K5 → each other; they reach spawn only via K0h |

## 8 · Interfaces
Programs launched (all via this door): `/usr/bin/bwrap` + `hee4-namespace-shim`, `/usr/bin/systemd-run`, `/usr/bin/busctl`, `/usr/bin/curl`, julia `bin/analysis.jl` (v4.2) — IM:53-63; UM:170-174. Children get allowlisted environments (IM:66).

## 9 · Done criteria
| # | Criterion | Evidence |
|---|---|---|
| 1 | Exactly one `Command::spawn`/equivalent site in the workspace | compiler-decided symbol check or census `duplicate_sites=0` (DEC:37; PL:95 K16) |
| 2 | TERM_GRACE / CLEANUP read from `bounds` | no literal duration in the crate (grep 0) |
| 3 | Tests that spawn are subreapers and reap with `wait` under a budget; no leaked descendants | gate leaked-descendant step PASS (AT:166) |
| 4 | Plants per rule (settle, escalation, pidfd verify) killed by named tests under `--cap-lints=warn` | `plants=k/k killers=named` (AT:149) |
| 5 | Scoped `cargo mutants` with CARGO_TARGET_DIR **unset**, precheck plant red | `mutants caught=a survivors=b` (AT:132; REQ:15) |
| 6 | SIGHUP treated as SIGTERM | named test (AT:135) |

## 10 · Open decisions and risks
- ER §2: "worker FINISHED" is unverified — treat as PARTIAL until v4 mutation runs.
- Budget semantics: "no created time limits" vs per-step budgets — owner decides; tests keep hang-guards (risk of conflating).
- Whether julia spawn (v4.2) needs a separate env allowlist type.
- Resolved, not open *(rev 2026-10-01 funnel audit)*: the register rows naming this module are all RESOLVED, ratified under delegation inside H-27's range: DC-38 (V4-60): P3 sandbox/cgroup seam (K0h / K2 / K6), five splits. The decision text is the V4 row in `plan/DECISIONS.md`; `hee4db highway --dc <DC-nn>` shows the row.

## 11 · Pull commands
```bash
cd ~/herdr-engineering-engine-v4
sed -n 47p ~/hee4-evidence/design/ULTRAMAP.md; sed -n 72p ~/hee4-evidence/design/ULTRAMAP.md; sed -n 277p ~/hee4-evidence/design/ULTRAMAP.md
sed -n 53,66p ~/hee4-evidence/reference/v3-evidence-b5367bc/interface-map-b5367bc.md
sed -n 48,52p ~/hee4-evidence/design/DECISION_POINTS-b5367bc.md
sed -n 7p ~/hee4-evidence/reference/v3-evidence-b5367bc/module-audit-b5367bc.md
rg -n 'worker FINISHED' ~/hee4-evidence/reference/ERRATA-v3-evidence-b5367bc.md
rg -n '### (EX-07|EX-08|EX-09|EX-18|A-3)' docs/EXEMPLARS.md
sed -n 129,130p ~/hee4-evidence/design/DEPLOYMENT_ATLAS.md; sed -n 161p ~/hee4-evidence/design/DEPLOYMENT_ATLAS.md
```
