# hee4-core · budget
> **Status: PLANNING — HOLD** (V4-0, DEC:4). No code until Luke says "start coding". This card is the stub.

## 1 · Identity
| Field | Value | Source |
|---|---|---|
| Crate | `hee4-core` | UM:74 |
| Cluster | K1 | UM:74 |
| v3 origin | `budget` (`src/budget.rs`) | MA:23; AR:9 |
| Status | PLANNING — HOLD | DEC:4 |

**Design section:** [K1 hee4-core › budget](obsidian://open?vault=herdr-engineering-engine-v4.vault&file=15%20Module%20Design/K1%20hee4-core%23budget): logic flow, interfaces, S-n/actions, RL/E2E, refusals, Must not, size budget. Elaboration only; this card and the authorities win. Index: [Module Design Index](obsidian://open?vault=herdr-engineering-engine-v4.vault&file=15%20Module%20Design%2F00%20-%20Module%20Design%20Index)

## 2 · Purpose, owned state, allowed dependencies
| Item | Value | Source |
|---|---|---|
| Purpose | **The one charge door**: store settles through `budget::Ledger` (the 3 SQL sites become 1). Also the reservation/report path for Jev calls if granted | UM:74; UM:263; JM:76 |
| Owned state | none durable; counters persisted by store | UM:74 |
| Value types | Amount/Provenance/Unit/Usage move to **K0** as cost types (context may not import K1) | UM:76; CMAP:11 |
| Allowed deps | `hee4-contracts` | UM:63 |

## 3 · Deployment
| Phase(s) | Entry gate | Exit evidence | Held-for-Luke |
|---|---|---|---|
| P3 worker + native + verdict — "budget Ledger is the single charge door (AR row 4)" | P2 | gate; plants per rule (AT:82) | — |
| P9 Jev charge through the door (shadow, if granted) | tag | RC01 `external request cap 0` refuses until changed | H-11 (AT:196), H-8 |
Feeds D6 (usage in one settle, AT:63).

## 4 · v3 basis
Flag **NOT FINISHED** — sound library, nothing composes it; three overrun rules (MA:23). Recommendation **REFACTOR** — the conserving Ledger has 0 production callers; real accounting is store SQL at 3 sites (AR:9).
| Finding | v3 file:line | Source | Errata |
|---|---|---|---|
| `Ledger` 0 production callers | `budget.rs:1142` | AP-09 | — |
| Docstring "Ledger is the only thing … that adds" while store adds | `budget.rs:301-304`; `store.rs:2019` | A-8; AP-15 | — |
| Charge ≤ reservation enforced at 3 store SQL sites | — | AR:9; AR:6 | — |
| v4 decision: wire as the one door, not delete (AR:4 option 1) | — | UM:263 | — |

## 5 · Decision points
| Site · kind · note | Jev |
|---|---|
| Ceiling::admits :713 · Privacy::admits :769 · reserve :1250 · ceiling check in `report` :1344 · release :1424 · select_fallback :1506 — EXACT (+caps), library only (DP:61) | not-Jev |
| Locator fixes: release :999→:1424; "apply :1357" does not exist, the check is in `report` :1344 (DP:23-24) | E16 |

## 6 · Migrated inputs
None — REFACTOR (MIG:22). Migrated `context.rs:325` imports `crate::budget::{Amount, Provenance, Unit, Usage}` (grep of MIG copy; MIG:13): those types go to `hee4-contracts` so K3 does not depend on K1 (UM:76). Accounting suite `accounting.rs` stays in v3, named only (MIG:25).

## 7 · Quality guard
| id | why for THIS module |
|---|---|
| AP-01 | charge rule at 3 SQL sites + an uncalled library = 4 doors |
| AP-09 | a pub library with 0 production callers |
| AP-15 | its own docstring claimed sole ownership |
| AP-13 | every counter row needs a reader (P11) |
| EX-19 | a doc that names both callers of the one door, checkable by grep |
| EX-02 | the charge happens inside the store's one write door |
| A-8 | the anti-exemplar docstring |
| D-01 | wiring must name the flow it moves (settle usage) |
| D-10 | if a charge census grows rounds, let types refuse instead |

## 8 · Interfaces
| Kind | Item | Source |
|---|---|---|
| Called by | store settle/accept (K1 intra); Jev judgment flow charge (held) | UM:74; UM:133 |
| External | none | — |

## 9 · Done criteria
| # | Criterion | Evidence / read-back |
|---|---|---|
| 1 | Exactly one charge site (store calls Ledger) | one-door census `duplicate_sites=0` (V4-17, DEC:37); `habitat-unused-pub` shows Ledger reached |
| 2 | Charge ≤ reservation conserved across settle | property test over sequences, independent known answers |
| 3 | Cost value types live in K0 | `cargo metadata`: hee4-cohesion does not depend on hee4-core (D-09) |
| 4 | Mutation | scoped `cargo mutants` on Ledger with `CARGO_TARGET_DIR` unset; survivors named (PL:82 K3) |
| 5 | Plants | planted overrun (charge > reservation) killed by a named test under `--cap-lints=warn` |

## 10 · Open decisions and risks
- "three overrun rules" (MA:23) must collapse to one stated rule — which one is UNMEASURED.
- Jev charge path depends on H-8/H-11 (AT:192, :187).
- Resolved, not open *(rev 2026-10-01 funnel audit)*: the register rows naming this module are all RESOLVED, ratified under delegation inside H-27's range: DC-12 (V4-66): Where the RC01 request cap refuses (K0e vs K1 budget). The decision text is the V4 row in `plan/DECISIONS.md`; `hee4db highway --dc <DC-nn>` shows the row.

## 11 · Pull commands
```bash
E=~/hee4-evidence; R=~/herdr-engineering-engine-v4
rg -n 'budget|Ledger|charge' $E/design/ULTRAMAP.md $E/design/DEPLOYMENT_ATLAS.md
rg -n 'budget' $E/design/DECISION_POINTS-b5367bc.md $E/reference/ERRATA-v3-evidence-b5367bc.md
rg -n 'budget' $E/reference/v3-evidence-b5367bc/*.md
rg -n 'budget' $R/migrated/v3-b5367bc/src/context.rs
rg -n 'AP-09|AP-15|A-8|EX-19' $R/docs/ANTIPATTERNS.md $R/docs/EXEMPLARS.md
```
