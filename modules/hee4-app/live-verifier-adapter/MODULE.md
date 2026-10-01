# hee4-app · live-verifier-adapter
> **Status: PLANNING — HOLD** (V4-0, DEC:4). No code until Luke says "start coding". This card is the stub.

## 1 · Identity
| Field | Value | Source |
|---|---|---|
| Crate | `hee4-app` | UM:79; DEC:33 (V4-13) |
| Cluster | K6 | UM:65 |
| v3 origin | `app` → `src/app/live_verifier.rs` (stage runner **and** the ledger's verdict source); `check` for the verdict it must hand to | DP:83; AR:7-8; MA:17 |
| Status | PLANNING — HOLD | DEC:4 |
| Why K6, not K4 | it needs worker and store (`app/live_verifier.rs:25-28` imports), so it stays an app adapter | UM:77; DEC:33 |

**Design section:** [K6 hee4-app › live-verifier-adapter](obsidian://open?vault=herdr-engineering-engine-v4.vault&file=15%20Module%20Design/K6%20hee4-app%23live-verifier-adapter): logic flow, interfaces, S-n/actions, RL/E2E, refusals, Must not, size budget. Elaboration only; this card and the authorities win. Index: [Module Design Index](obsidian://open?vault=herdr-engineering-engine-v4.vault&file=15%20Module%20Design%2F00%20-%20Module%20Design%20Index)

## 2 · Purpose, owned state, allowed dependencies
| Aspect | Content | Source |
|---|---|---|
| Purpose | Run the verification stages (candidate sandbox via bwrap, limit read-backs) and hand **observations** to K4 `check::decide`; it produces **no verdict** | UM:77, UM:114, UM:259; DEC:33 |
| Owned state | None; the verification row is written by K1 with value `verdict_of(decide)` | UM:201 |
| Allowed deps | K0, K0h (spawn door, cgroup-io aggregate/namespace I/O), K1 (store), K2 (worker), K4 (decide) | UM:65, UM:72 |
| Breaks | the runtime ↔ live_verifier/candidates/plan cycle, because the adapter only produces observations | UM:79; AR:7 |

## 3 · Deployment
| Phase(s) | Entry gate | Exit evidence | Held-for-Luke |
|---|---|---|---|
| **P3** (moved from P5 by V4-13): one check verdict authority; adapter runs stages, `decide` decides | P2 | gate; **plant making the adapter emit a verdict itself**, killed by a named test | none |
| must precede P4's first host record | P3 | P4 host record uses the single authority | H-9 |
Feeds D6 (verify step of the accepted task) — AT:63. Sources: AT:82, AT:90; DEC:33.

## 4 · v3 basis
- **Flag:** `app` PARTIAL (MA:19); `check` PARTIAL — "the ledger verdict is still live_verifier's, decide cannot PASS, no writer authentication" (MA:17).
- **Recommendation:** `app` REFACTOR (AR:7); `check` REFACTOR — "two verdict authorities" (AR:8).

| Finding | v3 file:line | Source | Errata |
|---|---|---|---|
| `checked` is the ledger verdict — a second authority | `live_verifier.rs:294` | DP:83; AP-01 | — |
| `decide` cannot PASS: 3 identities (Collector, Locks, Standards) hard-wired `None` | `app/u64_receipt.rs:136-145` `identities()` | UM:38 | **E15** (ER:25): not `check/decision.rs:46,97,120` |
| Imports worker and store | `app/live_verifier.rs:25-28` | UM:77; DEC:33 | — |
| Intra-app cycle runtime ↔ live_verifier (`live_verifier.rs:22` → runtime) | `runtime.rs:16`, `live_verifier.rs:22` | A-5 (EXX:543-554); AR:7 | — |
| Starts `/usr/bin/bwrap` (candidate sandbox) | `live_verifier.rs:35` | IM:56 | — |
| systemd get-property limit read-back | `aggregate.rs:337-349,550,1077-1089` | IM:50 | — |

## 5 · Decision points
| site · kind · note | Jev |
|---|---|
| `checked :294` · EXACT · the ledger verdict (a second authority) — deleted as a verdict in v4 | — (EXACT, JM:22) |

## 6 · Migrated inputs
None — `app` and `check` are REFACTOR (MIG:22). `t06_receipt_import` (migrated) imports app/check/store, which were not migrated (MIG:29).

## 7 · Quality guard
| id | why for THIS module |
|---|---|
| AP-01 | two verdict authorities is the v3 defect this module exists to end |
| AP-08 | runtime god-file cycle ran through this module (A-5) |
| AP-11 | adapter must not become a path for K2↔K1 cross-imports outside K6 |
| AP-22 | the "adapter emits a verdict" plant must fail on the named test, not on a compile error |
| AP-29 | a stage that looked at nothing must be `Unavailable`, not PASS |
| AP-49 | limit set on a scope is read back (get-property) before it counts |
| A-5 | god-file with intra-app cycle — do not reproduce |
| EX-17 | fail-closed severity lattice, sealed output — lives in K4; the adapter feeds it (EXX:331) |
| EX-08 / EX-09 | pidfd verified before retaining; TERM/KILL once — for the sandbox children |
| D-09 | compile-enforced: only K6 sees K1+K2+K4 together |

## 8 · Interfaces
| Kind | Item | Source |
|---|---|---|
| process (via K0h spawn door) | `/usr/bin/bwrap` + `hee4-namespace-shim` | UM:173; IM:56,60 |
| D-Bus (via K0h busctl) | get-property ControlGroup, MainPID, CPUQuota, MemoryMax, TasksMax… | IM:50; UM:171 |
| port | observations → K4 `decide` → `verdict_of` → K1 `verifications` row | UM:77, UM:201 |

## 9 · Done criteria
| # | criterion | evidence / read-back |
|---|---|---|
| 1 | The adapter has no code path that constructs a `Verdict`; the ledger value is `verdict_of(decide)` | compile-level: `Verdict` constructor not reachable from K6 adapter (design register DC-15: `decide` returns a K4 `Decided` newtype and K6 `settle` takes `&Decided`; a private constructor on the K0 enum `Verdict` cannot exist) *(rev 2026-10-01 design-set integrator)*. *(rev 2026-10-01 ratified V4-64, DC-15, DC-23)* ~~private ctor in K4~~ The seal is K4's `Decided` (private fields). The adapter returns `Decided`, and K6 runtime `settle` takes `&Decided` and calls `verdict_of`. The census prints `Verdict::` constructions outside K4/K0 parsing as 0 + plant "adapter emits verdict" killed by a named test under `--cap-lints=warn` (AT:82) |
| 2 | `decide` can PASS: Collector, Locks, Standards identities wired | a gate test reaching PASS with all three present, and each one missing → named Unavailable (UM:38; E15) |
| 3 | No module cycle inside app involving the adapter | `cargo` module graph / review; `largest_file=` printed at cut (D-05) |
| 4 | Stage observations carry the read-back values, not the requested ones | test asserting read-back fields over two fixtures differing in every field (AP-19/AP-20) |
| 5 | Scoped `cargo mutants` over the adapter's observation mapping, `CARGO_TARGET_DIR` unset | `mutants caught= survivors=` named (AT:132, AT:149) |

## 10 · Open decisions and risks
- "no writer authentication" on the verification record (MA:17) — no v4 decision yet; raise at P3 flow contract.
- R16-G3 identity facts deferred in v3 (`u64_receipt.rs:122-124`, RT:47) must be filled for PASS to be reachable — shared with `u64-class`.
- H-9 (TH-DEV, no seccomp until T15) governs what the sandbox may run (AT:193); H-14 O-15 for seccomp (AT:199).
- Resolved, not open *(rev 2026-10-01 funnel audit)*: the register rows naming this module are all RESOLVED, ratified under delegation inside H-27's range: DC-38 (V4-60): P3 sandbox/cgroup seam (K0h / K2 / K6), five splits. The decision text is the V4 row in `plan/DECISIONS.md`; `hee4db highway --dc <DC-nn>` shows the row.

## 11 · Pull commands
```bash
V4=~/herdr-engineering-engine-v4; EV=~/hee4-evidence; R=$EV/reference/v3-evidence-b5367bc
rg -n 'live_verifier|verdict_of|decide|V4-13' $EV/design/*.md $R/*.md $V4/plan/DECISIONS.md
sed -n '38p;77p;79p;114p;199p;257p' $EV/design/ULTRAMAP.md
sed -n '77p;85p' $EV/design/DEPLOYMENT_ATLAS.md
sed -n '17p;19p' $R/module-audit-b5367bc.md; sed -n '7,8p' $R/architecture-review-b5367bc.md
sed -n 25p $EV/reference/ERRATA-v3-evidence-b5367bc.md
sed -n '331,354p;543,555p' $V4/docs/EXEMPLARS.md
```
