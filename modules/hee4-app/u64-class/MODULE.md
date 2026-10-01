# hee4-app · u64-class
> **Status: PLANNING — HOLD** (V4-0, DEC:4). No code until Luke says "start coding". This card is the stub.

## 1 · Identity
| Field | Value | Source |
|---|---|---|
| Crate | `hee4-app` | UM:77, UM:79 |
| Cluster | K6 | UM:65 |
| v3 origin | `app` → `src/app/u64_receipt.rs` (verdict_of, decide_over, admit, diagnostics_of, identities); `check` class content | DP:86-87; AR:8 |
| Status | PLANNING — HOLD | DEC:4 |
| **Deviation from the brief** | not in the brief's K6 list; added because UM:77 says "U64 class content becomes a `Class` port implemented in K6" and AR:8 says "class content to an app-side class module". K4 keeps the generic `check::decide`; this card owns the U64-specific class implementation | UM:77; AR:8 |

**Design section:** [K6 hee4-app › u64-class](obsidian://open?vault=herdr-engineering-engine-v4.vault&file=15%20Module%20Design/K6%20hee4-app%23u64-class): logic flow, interfaces, S-n/actions, RL/E2E, refusals, Must not, size budget. Elaboration only; this card and the authorities win. Index: [Module Design Index](obsidian://open?vault=herdr-engineering-engine-v4.vault&file=15%20Module%20Design%2F00%20-%20Module%20Design%20Index)

## 2 · Purpose, owned state, allowed dependencies
| Aspect | Content | Source |
|---|---|---|
| Purpose | ~~Implement the `Class` port (defined in K0/K4)~~ Compute the **`ClassFacts` value** (a K0 type; no trait, *rev 2026-10-01 ratified V4-61, DC-39*) for the U64 rust-library-change class: admit the class receipt, supply class-specific identity facts and diagnostics to `decide` | UM:77; DP:86 |
| Not its job | producing the ledger verdict: that is `verdict_of(check::decide)` in K4 | UM:38; DEC:33 |
| Owned state | None | UM:79 |
| Allowed deps | K0 (receipt types), K4 (Class port, decide) via K6 | UM:65 |

## 3 · Deployment
| Phase(s) | Entry gate | Exit evidence | Held-for-Luke |
|---|---|---|---|
| **P3**: one verdict authority; the three identity sources wired so `decide` can PASS | P2 | gate; plants per rule | none |
| P5: R16-G3 identity facts filled for B16 | P4 | host record F07 | none |
Feeds D6 (AT:63). Sources: AT:82, AT:84; RT:47; DEC:33.

## 4 · v3 basis
- **Flag:** `app` PARTIAL (MA:19); `check` PARTIAL — "decide cannot PASS, no writer authentication" (MA:17).
- **Recommendation:** `check` REFACTOR — "U64 class content inside the generic checker" (AR:8); `app` REFACTOR (AR:7).

| Finding | v3 file:line | Source | Errata |
|---|---|---|---|
| `identities()` hard-wires Collector, Locks, Standards = `None` → `decide` can never PASS | `app/u64_receipt.rs:136-145` | UM:38 | **E15** (ER:25) corrects the earlier cite `check/decision.rs:46,97,120` |
| `verdict_of` exists but is **unwired** | `u64_receipt.rs:72` | DP:86; AP-01 | — |
| R16-G3 identity facts deferred | `u64_receipt.rs:122-124` | RT:47 | — |
| `diagnostics_of` = stderr empty or not | `u64_receipt.rs:91` | DP:87; JM:34 | — |

## 5 · Decision points
| site · kind · note | Jev |
|---|---|
| `verdict_of :72 (unwired) · decide_over :1576 · admit :1729` · EXACT (DP:86) | — |
| `diagnostics_of :91` · JUDGMENT · stderr empty or not (DP:87) | **J6: not-Jev** — use rustc `--error-format=json` exact counts (JM:34) |
| (K4-owned) `check/u64_oracle` evaluate :199 · consistency validate :222 · reviewed_case :519 · collector finalize_with :303 · EXACT (DP:89) | — (see `hee4-evidence/check` card) |

## 6 · Migrated inputs
None — `app` and `check` are REFACTOR (MIG:22). Receipt types it reads come from migrated `src/contracts/receipt/*` and `schemas/receipts/receipt-v1.schema.json` (owned by the `hee4-contracts` cards). `tests/t06_receipt_import.rs` (migrated) imports app, check, store — unresolved until they exist (MIG:29).

## 7 · Quality guard
| id | why for THIS module |
|---|---|
| AP-01 | v3 had two verdict doors; the class must not grow a third |
| AP-09 | `verdict_of` was public and unwired — every pub item gets its production caller |
| AP-16 | stderr-empty is a judgment; replace with rustc JSON counts (J6) |
| AP-19 | a class that can only fail pins nothing; require a PASS case and each missing identity → named Unavailable |
| AP-21 | identity facts / KATs from recorded receipts, not typed by the same head |
| AP-15 | a doc saying "decide is the verdict" must match the wiring (it did not in v3) |
| EX-17 | fail-closed severity lattice, sealed output (EXX:331) |
| EX-20 | constants checked against contract text |
| A-8 | a docstring claiming a property the code does not have |
| D-09 | class content in K6 behind a port; K4 stays generic |

## 8 · Interfaces
| Kind | Item | Source |
|---|---|---|
| value | ~~`Class` port (K4 defines / K0 types), implemented here~~ `ClassFacts` (K0 type), computed here and passed by the adapter to `decide(Observations, ClassFacts)` (*rev 2026-10-01 ratified V4-61*) | UM:77 |
| inputs | candidate receipt (from `plan`), rustc JSON diagnostics | RR:17; JM:34 |
| output | identity facts + diagnostics → `decide` → `verdict_of` | UM:38 |

## 9 · Done criteria
| # | criterion | evidence / read-back |
|---|---|---|
| 1 | `decide` reaches PASS for a real U64 receipt; each of Collector/Locks/Standards missing → named Unavailable | gate test over 4 fixtures from recorded receipts (E15) |
| 2 | Diagnostics from rustc `--error-format=json` counts, not stderr emptiness | test over recorded rustc JSON (J6) |
| 3 | No verdict constructed here | plant "class returns Pass" killed by a named test (AT:82 pattern) |
| 4 | Scoped `cargo mutants` over admit/identity mapping, `CARGO_TARGET_DIR` unset | `mutants caught= survivors=` named (AT:149) |

## 10 · Open decisions and risks
- Exact split of `u64_oracle` (K4 check) vs this card — AR:8 says "class content to an app-side class module"; DP:89 lists `u64_oracle` under check. Decide at P3 flow contract (INTERP).
- Writer authentication of the verification record (MA:17) — no v4 decision yet.

## 11 · Pull commands
```bash
V4=~/herdr-engineering-engine-v4; EV=~/hee4-evidence; R=$EV/reference/v3-evidence-b5367bc
sed -n '86,89p' $EV/design/DECISION_POINTS-b5367bc.md
sed -n '38p;77p' $EV/design/ULTRAMAP.md; sed -n 25p $EV/reference/ERRATA-v3-evidence-b5367bc.md
sed -n '17p;19p' $R/module-audit-b5367bc.md; sed -n '8p' $R/architecture-review-b5367bc.md; sed -n '47p' $R/route-to-v010-b5367bc.md; sed -n 34p $R/jev-decision-map-b5367bc.md
sed -n 33p $V4/plan/DECISIONS.md
sed -n '331,354p;592,602p' $V4/docs/EXEMPLARS.md
```
