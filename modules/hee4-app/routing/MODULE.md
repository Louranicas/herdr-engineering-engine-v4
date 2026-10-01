# hee4-app · routing
> **Status: PLANNING — HOLD** (V4-0, DEC:4). No code until Luke says "start coding". This card is the stub.

## 1 · Identity
| Field | Value | Source |
|---|---|---|
| Crate | `hee4-app` | UM:79 |
| Cluster | K6 (composition of K2 `route` policy) | UM:65, UM:75 |
| v3 origin | `app` → `src/app/routing.rs` (join/code, preview); consumer of `route` (not a dependency of it) | DP:77; MIG:12 |
| Status | PLANNING — HOLD | DEC:4 |
| Config | `~/.config/hee4/routing/` | UM:182; IM:72 |

**Design section:** [K6 hee4-app › routing](obsidian://open?vault=herdr-engineering-engine-v4.vault&file=15%20Module%20Design/K6%20hee4-app%23routing): logic flow, interfaces, S-n/actions, RL/E2E, refusals, Must not, size budget. Elaboration only; this card and the authorities win. Index: [Module Design Index](obsidian://open?vault=herdr-engineering-engine-v4.vault&file=15%20Module%20Design%2F00%20-%20Module%20Design%20Index)

## 2 · Purpose, owned state, allowed dependencies
| Aspect | Content | Source |
|---|---|---|
| Purpose | Load routing config, call the pure K2 route policy, serve `task.preview`; in v4 also wire `evaluate_fallback` into dispatch (B15) | DP:77; UM:75 |
| Owned state | None | UM:79 |
| Allowed deps | K0, K2 (`route`, `roster` eligibility) via K6 | UM:65 |

## 3 · Deployment
| Phase(s) | Entry gate | Exit evidence | Held-for-Luke |
|---|---|---|---|
| P1/P2: `task.preview` stays served (v4.0 action) | P0 | gate: task.* flows L2 | none |
| P9 (post-tag, v4.1): B15 fallback + roster actions (RT S11: l2 +2) | tag | per-slice gate | none |
Sources: UM:157; UM:268; AT:88; RT:53.

## 4 · v3 basis
- **Flag:** `app` PARTIAL (MA:19); `route` PARTIAL — "production use is preview only, fallback uncalled" (MA:12).
- **Recommendation:** `app` REFACTOR (AR:7); `route` HARDEN (AR:18).

| Finding | v3 file:line | Source | Errata |
|---|---|---|---|
| preview is the only live caller of route | `routing.rs:252` | DP:77 | — |
| `evaluate_fallback` defined, only test callers (`t09_route.rs` :1962, :2277, :2292, :2629) | `route.rs:2074` | RT:13; UM:75 | — |
| task.preview path: app::tasks → app::routing → route | `tasks.rs:914` | IM:19 | — |
| R11 ranking uses hand-typed cost/quality/latency figures | `route.rs:1925/2110` | DP:76; JM:33 | — |

## 5 · Decision points
| site · kind · note | Jev |
|---|---|
| `join/code :177 · preview :252` · EXACT (DP:77) | — |
| acquisition bounds "routing" · THRESHOLD (DP:104) | not-Jev |
| (in route) R11 ranking · JUDGMENT (DP:76) | **J5: not-Jev** — replace figures with measured ones in code (JM:33) |

## 6 · Migrated inputs
None for `app/routing.rs` (REFACTOR, MIG:22). It consumes migrated `src/route.rs` + `tests/t09_route.rs` (owned by the `hee4-worker/route` card). Coupling to note: `t09_route.rs:2352` splits `route.rs` on `HEE3-ANCHORS-END`, so stripping route's anchor block must change that test in the same step (DEC:27, V4-10).

## 7 · Quality guard
| id | why for THIS module |
|---|---|
| AP-09 | `evaluate_fallback` public with no production caller — give it one (B15) or delete |
| AP-06 | observation age and routing inputs are values passed in, not read inside the policy |
| AP-01 | route config validated twice in v3 (AR:18) — one validation door |
| AP-12 | no string match on step kinds; typed rule per step kind (AR:18) |
| EX-10 | pure, explained, order-independent routing (EXX:210) |
| EX-20 | constants checked against contract text |
| D-01 | B15 lands with C01 so the stack moves l2 by 2 (RT:53) |

## 8 · Interfaces
| Kind | Item | Source |
|---|---|---|
| action served | `task.preview` (v4.0) | UM:157; IM:19 |
| actions later | roster.list/inspect (v4.1) via registry | UM:159 |
| config | `~/.config/hee4/routing/` | UM:182 |

## 9 · Done criteria
| # | criterion | evidence / read-back |
|---|---|---|
| 1 | `task.preview` served through `main` and the owner registry | gate socket test (t28-style) |
| 2 | Fallback called from dispatch (v4.1) with a production caller | `habitat-unused-pub` shows no route pub without caller |
| 3 | Known answers from an independent source (keep v3's independent KATs) | test provenance line (MA:12) |
| 4 | Plants on config validation killed by named tests; scoped mutants over the app-side composition, `CARGO_TARGET_DIR` unset | `plants=k/k`; `mutants caught= survivors=` (AT:149) |

## 10 · Open decisions and risks
- `roster: Option<&dyn RosterActions>` field (DS5-open in v3) — decide at B15 (RT:53).
- Measured cost/quality/latency figures do not exist yet (J5, JM:33) — UNMEASURED.

## 11 · Pull commands
```bash
V4=~/herdr-engineering-engine-v4; EV=~/hee4-evidence; R=$EV/reference/v3-evidence-b5367bc
rg -n 'routing|evaluate_fallback|task.preview|B15' $EV/design/*.md $R/*.md
sed -n '75p;155p;157p;180p;266p' $EV/design/ULTRAMAP.md
sed -n '12p;19p' $R/module-audit-b5367bc.md; sed -n '18p' $R/architecture-review-b5367bc.md; sed -n '13p;53p' $R/route-to-v010-b5367bc.md
sed -n 2350,2354p $V4/migrated/v3-b5367bc/tests/t09_route.rs
sed -n '210,236p' $V4/docs/EXEMPLARS.md
```
