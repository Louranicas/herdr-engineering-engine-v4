# hee4-evidence · check
> **Status: PLANNING — HOLD** (V4-0, DEC:4). No code until Luke says "start coding". This card is the stub.

## 1 · Identity
| Field | Value | Source |
|---|---|---|
| Crate | `hee4-evidence` | UM:77 |
| Cluster | K4 (verification and numerical evidence) | UM:77; CMAP:3 |
| v3 origin | `check` (`check/decision.rs`, `check/u64_oracle`, `check/consistency`, `check/collector`); the verdict half of `app/u64_receipt.rs` (`verdict_of`) | MA:17; DP:88-89, DP:86 |
| Role | **The one verdict authority**: `decide` → `verdict_of` is the only ledger verdict | UM:38 (P4); DEC:33 (V4-13) |
| Status | PLANNING — HOLD | DEC:4 |

**Design section:** [K4 hee4-evidence › check](obsidian://open?vault=herdr-engineering-engine-v4.vault&file=15%20Module%20Design/K4%20hee4-evidence%23check): logic flow, interfaces, S-n/actions, RL/E2E, refusals, Must not, size budget. Elaboration only; this card and the authorities win. Index: [Module Design Index](obsidian://open?vault=herdr-engineering-engine-v4.vault&file=15%20Module%20Design%2F00%20-%20Module%20Design%20Index)

## 2 · Purpose, owned state, allowed dependencies
| Item | Value | Source |
|---|---|---|
| Purpose | Decide a candidate's verdict from observations; nothing else produces a verdict | UM:38, UM:77 |
| Owned state | None durable. The verdict value is written by K1 (`verifications` row = `verdict_of(decide)`) | UM:77, UM:201 |
| Owns | the fail-closed severity lattice, the three identity sources (Collector, Locks, Standards) wired so `decide` can PASS, Decision+Observed sealed together | UM:77; AR:8 |
| Allowed deps | `hee4-contracts` (K0), `hee4-host` (K0h). Never K1/K2/K6 | UM:62-66, UM:77 |
| Not here | `live_verifier` stays a **K6 adapter** that runs the stages (needs worker + store) and hands observations to `decide`; U64 class content becomes a `Class` port implemented in K6 | UM:77; DEC:33 |

## 3 · Deployment
| Phase | Entry gate | Exit evidence | Held-for-Luke |
|---|---|---|---|
| P3 Worker + native + verdict (moved from P5 by V4-13) | P2 | gate; plants per rule, **including one that makes the adapter emit a verdict itself**; verdict authority single before P4's first host record | none (AT:82) |
| P5 System scenario | P4 | B16 fail → repair → verify → accept through main, host record F07 | none (AT:84) |
Feeds D-rows: D6 (accepted end to end; check in the path) AT:63; D8 (scoreboard) AT:65.

## 4 · v3 basis
Flag **PARTIAL** (MA:17): "decide+finalize run through main as evidence; the ledger verdict is still live_verifier's, decide cannot PASS, no writer authentication". Recommendation **REFACTOR** (AR:8): two verdict authorities → one verdict, ledger = `verdict_of(decide)`, seal Decision+Observed, class content to an app-side class module.

| Finding | v3 file:line | Source | Errata |
|---|---|---|---|
| Ledger takes live_verifier's verdict (second authority) | `app/live_verifier.rs:294` `checked` | DP:83; AP-01 row | — |
| `verdict_of` exists but is unwired | `app/u64_receipt.rs:72` | DP:86 | — |
| `decide` cannot PASS: identities (Collector, Locks, Standards) hard-wired `None`/Unavailable | `app/u64_receipt.rs:136-145` `identities()` | UM:38 | **E15**: the earlier cite `check/decision.rs:46,97,120` names the `Unavailable` variants, not the wiring |
| Severity lattice, fail-closed, sealed output — sound (exemplar) | `check/decision.rs:296-413` | EX-17 | — |
| decide over causes/identities/cases/process/oracle/evidence/timing | `check/decision.rs:370-895` | DP:88 | — |
| live_verifier imports worker + store, so it cannot move into K4 | `app/live_verifier.rs:25-28` | UM:77; DEC:33 | — |
| runtime ↔ live_verifier cycle breaks once live_verifier only produces observations | `app/runtime.rs` | UM:79; A-5 | — |

## 5 · Decision points
| site · kind · note | Jev |
|---|---|
| `check/decision` decide/causes/identities/cases/process/oracle/evidence/timing :370-895 · EXACT · severity lattice, fail-closed (DP:88) | not-Jev (EXACT, JM:22) |
| `check/u64_oracle · consistency · collector` evaluate :199 · validate :222 · reviewed_case :519 · finalize_with :303 · EXACT (DP:89) | not-Jev |
| `app/u64_receipt` verdict_of :72 (unwired) · decide_over :1576 · admit :1729 · EXACT (DP:86) | not-Jev |
| `app/u64_receipt` diagnostics_of :91 · JUDGMENT · stderr empty or not (DP:87) | **J6 not-Jev**: rustc `--error-format=json` gives exact counts (JM:34) |
| `app/live_verifier` checked :294 · EXACT · the second authority, removed as a verdict source (DP:83; UM:259) | — |

## 6 · Migrated inputs
None. `check` is REFACTOR, not carried verbatim (MIG:22). `tests/t06_receipt_import.rs` (migrated with contracts) imports app, check, store, which were not migrated (MIG:27-29); rewrite it against the v4 `decide` when K4 lands. v4 names follow the `hee4` namespace (DEC:31, V4-11).

## 7 · Quality guard
| id | why for THIS module |
|---|---|
| AP-01 | Two verdict authorities is the canonical v3 instance (live_verifier.rs:294 vs u64_receipt.rs:72) |
| AP-22 | `decide` refuses an absent identity source or a stage observation that did not happen; neither may become PASS or a "caught" plant, and the §9 #6 battery requires the killing test's own name in each failure *(rev 2026-10-01 open-tasks CN-19: V5 generic row made module-specific)* |
| AP-29 | Exit code or caption as verdict: `decide` must print what it looked at |
| AP-18 | The adapter's doubles must record the observations handed to `decide`, or the verdict is unobservable |
| AP-21 | Known-answer cases must come from an independent source (recordings), not from the lattice's author |
| AP-13 | `verdict_of` was written and unwired; every output must have a production reader |
| EX-17 | Imitate: fail-closed severity lattice with sealed output |
| EX-06 | Imitate: predicates that read their arms' values |
| A-5 | Do not rebuild the runtime ↔ verifier cycle |
| D-09 | K4 may not import K1/K2/K6; the compiler enforces it |
| D-10 | If verdict-rule plants keep surviving per round, move the rule into types |

## 8 · Interfaces
| Kind | Item | Source |
|---|---|---|
| Called by | K6 live-verifier adapter (observations → `decide`); K6 runtime `settle` writes `verdict_of(decide)` via K1 | UM:79, UM:114 |
| Uses | `bwrap` check run via the K0h spawn door, **called by the K6 live-verifier adapter, not K4** (DEC V4-13; design register DC-01; UM §4c wording pending ratification) *(rev 2026-10-01 design-set integrator)* | UM:173 |
| State | `verifications` table value = `verdict_of(decide)` (K1 writes) | UM:201 |
| Enum | `Verdict`: one enum in K0 (v3 had three spellings) | UM:214 |

## 9 · Done criteria
| # | criterion | evidence / read-back |
|---|---|---|
| 1 | `decide` can return PASS: all three identity sources wired | gate test through main reaching PASS; plant that re-hardwires one identity to `None` is killed by a named test (AT:82; UM:77) |
| 2 | Exactly one verdict producer | planted verdict emitted by the K6 adapter is killed by a named test (AT:82); one-door census `duplicate_sites=0` for verdict (UM:39) |
| 3 | Ledger verdict = `verdict_of(decide)` before P4's first host record | P4 host record carries a verdict produced by K4 (DEC:33; AT:90) |
| 4 | Decision and Observed sealed together | unit test pinning both halves whole over two fixtures differing in every field |
| 5 | Lattice pinned | scoped `cargo mutants` on `check` with `CARGO_TARGET_DIR` unset; survivors named or equivalent-with-reason (AT:132; PL:82 K3) |
| 6 | Plants killed for the right reason | plant battery under `--cap-lints=warn`, killing test named in each failure (AT:163; AP-23) |
| 7 | F07 accept path verified by this verdict | host record F07 (AT:84) |

## 10 · Open decisions and risks
- Writer authentication of verdict evidence absent in v3 (MA:17); no v4 decision names it yet — raise at the P3 flow contract.
- Where exactly U64 class content lives (K6 `Class` port) must not leak back into K4 (UM:77).
- `u64_oracle`/`collector` placement: K4 keeps the generic checker; class-specific oracle content moves to K6 (AR:8) — split line UNMEASURED until P3 skeleton.
- Risk: the identity sources depend on facts R16-G3 deferred at `u64_receipt.rs:122-124` (RT S5 row).
- Resolved, not open *(rev 2026-10-01 funnel audit)*: the register rows naming this module are all RESOLVED, ratified under delegation inside H-27's range: DC-15 (V4-64): Verdict seal: `Verdict` is a K0 enum, so no K4-private constructor exists; DC-23 (V4-64): Verdict hand-off; DC-39 (V4-61): `Class` port: home and seam shape. The decision text is the V4 row in `plan/DECISIONS.md`; `hee4db highway --dc <DC-nn>` shows the row.

## 11 · Pull commands
```bash
E=~/hee4-evidence; R=$E/reference/v3-evidence-b5367bc; V=~/herdr-engineering-engine-v4
sed -n '38p;77p;79p;114p;199p;212p;257p' $E/design/ULTRAMAP.md
sed -n '77p;79p;85p' $E/design/DEPLOYMENT_ATLAS.md
sed -n '33p' $V/plan/DECISIONS.md
sed -n '83,89p' $E/design/DECISION_POINTS-b5367bc.md
sed -n '8p' $R/architecture-review-b5367bc.md; sed -n '17p' $R/module-audit-b5367bc.md
grep -n 'E15' $E/reference/ERRATA-v3-evidence-b5367bc.md
grep -n 'EX-17\|EX-06\|A-5 ' $V/docs/EXEMPLARS.md | head
grep -n 'AP-01\|AP-22\|AP-29' $V/docs/ANTIPATTERNS.md | head
grep -n 'J6' $R/jev-decision-map-b5367bc.md
```
