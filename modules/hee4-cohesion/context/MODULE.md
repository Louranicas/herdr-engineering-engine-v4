# hee4-cohesion · context
> **Status: PLANNING — HOLD** (V4-0, DEC:4). No code until Luke says "start coding". This card is the stub.

## 1 · Identity
| Field | Value | Source |
|---|---|---|
| Crate | `hee4-cohesion` | UM:76 |
| Cluster | K3 | UM:76 |
| v3 origin | `context` (`src/context.rs`) | MA:14; AR:21; MIG:13 |
| Status | PLANNING — HOLD | DEC:4 |

**Design section:** [K3 hee4-cohesion › context](obsidian://open?vault=herdr-engineering-engine-v4.vault&file=15%20Module%20Design/K3%20hee4-cohesion%23context): logic flow, interfaces, S-n/actions, RL/E2E, refusals, Must not, size budget. Elaboration only; this card and the authorities win. Index: [Module Design Index](obsidian://open?vault=herdr-engineering-engine-v4.vault&file=15%20Module%20Design%2F00%20-%20Module%20Design%20Index)

## 2 · Purpose, owned state, allowed dependencies
| Item | Value | Source |
|---|---|---|
| Purpose | Assemble bounded context packets for a brief; owns **the one Omission vocabulary** (shared with skills) | UM:76; UM:218 |
| Owned state | none | UM:76 |
| Allowed deps | `hee4-contracts` (cost types moved there from budget) | UM:63; UM:76 |
| v3 edge removed | `context → budget` (`context.rs:325` imports `budget::{Amount, Provenance, Unit, Usage}`) | CMAP:11; grep of MIG copy |

## 3 · Deployment
| Phase(s) | Entry gate | Exit evidence | Held-for-Luke |
|---|---|---|---|
| P9 milestone-2 | tag | per-slice gates (AT:88) | J9 advice needs H-8 (AT:192) |

## 4 · v3 basis
Flag **PARTIAL** — bounds, coverage and compare met; no repeat cost, not called from main (MA:14). Recommendation **HARDEN** — second omission vocabulary in skills; O(n²) walk; unbounded Permit; depth checked before permit (AR:21).
| Finding | v3 file:line | Source | Errata |
|---|---|---|---|
| Caps are the `MAX_*` consts | `context.rs:329-361` (was cited :968) | DP:113 | E16 |
| `MAX_SOURCES == 16 * MAX_SELECTED` compile-time assert | `context.rs:355` | grep of MIG copy | — |
| Permit unbounded (derived set) | — | AR:21; AP-04 | — |
| Fix: BTreeSets, `MAX_PERMIT`, permit before depth | — | AR:21; MIG:13 | — |

## 5 · Decision points
| Site · kind · note | Jev |
|---|---|
| omission chain :1082 · is_gap/compare/consumers — EXACT (DP:112) | not-Jev |
| assemble :1032 · depth :1082 · caps `MAX_*` :329-361 — THRESHOLD (DP:113) | not-Jev |
| budget selection :1118 — JUDGMENT, first-fit, no relevance (DP:114) | **J9: Jev (grant)** — relevance per source; order and budget stay in code (JM:37) |

## 6 · Migrated inputs
| Path | Lines | Anchor lines | Note |
|---|---|---|---|
| `migrated/v3-b5367bc/src/context.rs` | 1,334 | 246 | strip anchor block (V4-10, DEC:27); replace `crate::budget` with K0 cost types |
| `…/tests/t11_context.rs` | 1,673 | — | primary test |
Hardening owed per MIG:13: one Omission vocabulary; BTreeSet `seen`/Permit + MAX_PERMIT; permit before depth; first-fit stated or stopped. Namespace `hee4` (V4-11, DEC:31).

## 7 · Quality guard
| id | why for THIS module |
|---|---|
| AP-04 | Permit is a derived set with no bound |
| AP-01 | two omission vocabularies (context, skills) |
| AP-16 | first-fit selection is a JUDGMENT (J9) — state it or stop it |
| AP-11 | the context→budget edge must not reappear |
| AP-45 | 246 anchor lines to strip |
| EX-13 | `take(N+1)` then refuse |
| EX-18 | compile-time-checked constant relation (like `:355`) |
| EX-05 | one Omission spelling, `parse → None` |
| D-09 | K3 cannot see K1; compiler-enforced |

## 8 · Interfaces
| Kind | Item | Source |
|---|---|---|
| Consumers | cohort briefs; Jev J9 consumer (held) | JM:81 |
| External | none | — |

## 9 · Done criteria
| # | Criterion | Evidence / read-back |
|---|---|---|
| 1 | `MAX_PERMIT` bounds Permit; refusal names both numbers | test at N and N+1 |
| 2 | One Omission vocabulary, used by skills | grep one definition |
| 3 | Permit checked before depth | ordering test |
| 4 | Walk not O(n²) (BTreeSet) | test at MAX_SOURCES with recorded wall time |
| 5 | No dependency on hee4-core | `cargo metadata` edge list (D-09) |
| 6 | Plants / mutation | plant removing MAX_PERMIT killed by named test under `--cap-lints=warn`; scoped mutants named |

## 10 · Open decisions and risks
- J9 held (H-8). "repeat cost" not met in v3 (MA:14) — owner UNMEASURED.
- Resolved, not open *(rev 2026-10-01 funnel audit)*: the register rows naming this module are all RESOLVED, ratified under delegation inside H-27's range: DC-14 (V4-56): Home of shared vocabularies (Join/Blocked, Omission): K0 (UM §5b) vs K3. The decision text is the V4 row in `plan/DECISIONS.md`; `hee4db highway --dc <DC-nn>` shows the row.

## 11 · Pull commands
```bash
E=~/hee4-evidence; R=~/herdr-engineering-engine-v4
rg -n 'context' $E/design/ULTRAMAP.md $E/design/DECISION_POINTS-b5367bc.md
rg -n 'context' $E/reference/v3-evidence-b5367bc/*.md $E/reference/ERRATA-v3-evidence-b5367bc.md
rg -n 'MAX_|use crate|HEE3-ANCHORS' $R/migrated/v3-b5367bc/src/context.rs | head -30
wc -l $R/migrated/v3-b5367bc/src/context.rs $R/migrated/v3-b5367bc/tests/t11_context.rs
```
