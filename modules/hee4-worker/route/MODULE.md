# hee4-worker · route
> **Status: PLANNING — HOLD** (V4-0, DEC:4). No code until Luke says "start coding". This card is the stub.

## 1 · Identity
| Field | Value | Source |
|---|---|---|
| Crate | `hee4-worker` | UM:75 |
| Cluster | K2 | UM:75; CMAP:3 |
| v3 origin | `route` (`src/route.rs`, 2,165 lines; `route/config.rs`) | MA:12; AR:18; MIG:12 |
| Status | PLANNING — HOLD; HARDEN module staged verbatim, not wired | DEC:6 (V4-2); MIG:5 |
| Binary | none | UM:82 |

**Design section:** [K2 hee4-worker › route](obsidian://open?vault=herdr-engineering-engine-v4.vault&file=15%20Module%20Design/K2%20hee4-worker%23route): logic flow, interfaces, S-n/actions, RL/E2E, refusals, Must not, size budget. Elaboration only; this card and the authorities win. Index: [Module Design Index](obsidian://open?vault=herdr-engineering-engine-v4.vault&file=15%20Module%20Design%2F00%20-%20Module%20Design%20Index)

## 2 · Purpose, owned state, allowed dependencies
| Aspect | v4 shape | Source |
|---|---|---|
| Purpose | Routing **policy**: pure, explained, order-independent rule pipeline (R02-R13) that picks a candidate provider; `evaluate_fallback` wired into dispatch | UM:75; AR:18; EX-10 |
| Owned state | none durable | UM:75 |
| Target shape | typed rule per step kind (Step accepts any Rule today); keep the admitted policy; split `route/config.rs` | AR:18; UM:75 |
| Allowed deps | `hee4-contracts`, `hee4-host` | UM:62-66 |
| Consumer | K6 `app/routing` (preview is the only live caller today) | DP:77; MIG:12 |

## 3 · Deployment
| Phase(s) | Entry gate | Exit evidence | Held-for-Luke |
|---|---|---|---|
| P1 (preview flow: `task.preview` through `main`) | P0 green | gate; route KATs from an independent source | none (AT:80) |
| P9 / v4.1 (B15 fallback wired in dispatch, paired with C01) | tag pushed | per-slice gate; l2 +2 with roster (RT:53) | H-1 (AT:185) |
D-rows fed: D8 (task.preview flow names its L2 test, AT:65).

## 4 · v3 basis
Flag **PARTIAL** — "router and independent known answers done; production use is preview only, fallback uncalled" (MA:12). Recommendation **HARDEN** — "pure and sound; Step accepts any Rule; config validated twice; 2,165-line file" (AR:18).
| Finding | v3 file:line | Source | Errata |
|---|---|---|---|
| `evaluate_fallback` has only test callers (`tests/t09_route.rs:1962,2277,2292,2629`) | `route.rs:2074` | RT:13; UM:75; V2:39 | — |
| Production use is `task.preview` only | `app/routing.rs:252`, `tasks.rs:914` | DP:77; IM:19 | — |
| Config validated twice; Step accepts any Rule | `route.rs:1443` (config admitted) | AR:18; DP:74 | — |
| R11 ranks on hand-typed cost/quality/latency figures | `route.rs:1925`, `:2110` | DP:76; JM:33 (J5) | — |
| Imports contracts only | `route.rs:312-313` | MIG:12; migrated `src/route.rs:312-313` | — |
| Route policy reused verbatim as a pattern (pure, independent known answers) | — | UM:283 | — |

## 5 · Decision points
| site · kind · note | Jev |
|---|---|
| `apply_filter` R02/R03/R04 `:1758-1774` · R06 `:1800` · screen R13 `:1857` · `guard_baseline :1939` · decide R09/R10/R12 `:2096` · config admitted `:1443` · EXACT · pure (DP:74) | not-Jev |
| R05 availability `:1779` · R07 deadline `:1812` · R08 quality `:1824` · THRESHOLD · hand-declared figures (DP:75) | not-Jev (numbers, JM:23) |
| R11 ranking `:1925/:2110` · JUDGMENT · ranks on hand-typed figures (DP:76) | **J5: not-Jev** — replace figures with measured ones in code (JM:33) |

## 6 · Migrated inputs
| File | Lines | Anchor lines | Notes |
|---|---|---|---|
| `migrated/v3-b5367bc/src/route.rs` | 2,165 | 248 (block ends `:299`) | strip the anchor block (V4-10, DEC:27) |
| `migrated/v3-b5367bc/tests/t09_route.rs` | 4,187 | 1 | **coupled**: see below |
**The `t09_route.rs:2352` coupling (V4-10, DEC:27).** `policy_source_names_no_effectful_item()` does `include_str!("../src/route.rs")` then `source.split("HEE3-ANCHORS-END").nth(1).unwrap_or_default()` (migrated `tests/t09_route.rs:2350-2354`). Stripping the anchor block makes `nth(1)` return `None` → `unwrap_or_default()` → an **empty body**, so the "no effectful item" check would silently pass over nothing (AP-29). The strip and the test change land in **one step**: parse the whole source (or the module's items via `syn`) instead of splitting on a marker, and add a control that plants an effectful path and requires the named failure (AP-22).
Other entry changes: hee4 namespace (V4-11, DEC:31); crate path; typed rule per step kind; split `route/config.rs` (AR:18). No v3 dependency is missing (MIG:12).

## 7 · Quality guard
| id | why for THIS module |
|---|---|
| AP-26 | the t09 marker-split is a text census over a declaration; parse instead |
| AP-29 | an empty split body passes the policy test — a check that looked at nothing |
| AP-22 | the replacement control must fail on its own diagnostic (the planted effectful path) |
| AP-08 | 2,165-line file; split config (AR:18) |
| AP-21 | route KATs must keep their independent source (UM:283 "independent known answers") |
| AP-16 | R11 ranking is JUDGMENT over numbers — measure figures, don't tune by hand (J5) |
| AP-09 | `evaluate_fallback` pub with no production caller until wired |
| EX-10 | the pattern this module is: pure, explained, order-independent routing (EXX:24, EXX:210) |
| EX-06 | rule predicates read their arms' values |
| D-10 | if route checks start receding, prefer typed rules (compiler) over more tests |
| D-01 | B15 lands only with its l2 delta (RT:53) |

## 8 · Interfaces
| Kind | Item | Source |
|---|---|---|
| Action (via K6) | `task.preview` → app::tasks → app::routing → route | IM:19; UM:157 |
| Config read (via K6 loader) | `~/.config/hee4/` routing/ | UM:182 |
| Called by | K6 dispatcher (`evaluate_fallback`, v4.1) | UM:75; RT:53 |

## 9 · Done criteria
| # | criterion | evidence / read-back |
|---|---|---|
| 1 | Typed rule per step kind: a Step with the wrong Rule does not compile | planted wrong pairing fails `cargo check` with the named type error (D-10) |
| 2 | Config validated once | one-door census `duplicate_sites=0` (DEC:37) |
| 3 | t09 policy check parses the whole source; its control plants an effectful item and names the failing test | `cases=k/k` + named killer (AP-22) |
| 4 | `rg -c 'HEE3-ANCHORS' ` over route.rs and t09 = 0 (anchor strip complete) | v4 independence check `v3_refs=0` (DEC:28) |
| 5 | Route KATs keep an independent source, cited in each test | review on committed objects |
| 6 | v4.1: `evaluate_fallback` has a production caller in dispatch | `habitat-unused-pub` clean; F03/F04 flows l2 +2 (RT:53) |
| 7 | Mutation: scoped `cargo mutants` over route, survivors named/equivalent | Tier-1 line (AT:149) |

## 10 · Open decisions and risks
- R11 figures: measured source not decided (J5, JM:33); until then ranking stays on declared figures.
- B15 is post-tag (RT:27, RT:53); fallback stays dark in v4.0.
- Risk: the marker-split test silently passing on stripped source (above) — the top risk when the file is brought in.

## 11 · Pull commands
```bash
grep -n 'route' ~/hee4-evidence/design/ULTRAMAP.md ~/hee4-evidence/design/DECISION_POINTS-b5367bc.md
sed -n 12p ~/hee4-evidence/reference/v3-evidence-b5367bc/module-audit-b5367bc.md
sed -n 18p ~/hee4-evidence/reference/v3-evidence-b5367bc/architecture-review-b5367bc.md
grep -n 'evaluate_fallback\|S11' ~/hee4-evidence/reference/v3-evidence-b5367bc/route-to-v010-b5367bc.md
sed -n 2350,2354p ~/herdr-engineering-engine-v4/migrated/v3-b5367bc/tests/t09_route.rs
grep -n 'HEE3-ANCHORS-END\|fn evaluate_fallback\|fn apply_filter' ~/herdr-engineering-engine-v4/migrated/v3-b5367bc/src/route.rs
grep -n 'V4-10' ~/herdr-engineering-engine-v4/plan/DECISIONS.md
sed -n '/### EX-10/,/### EX-11/p' ~/herdr-engineering-engine-v4/docs/EXEMPLARS.md
```
