# hee4-cohesion · skills
> **Status: PLANNING — HOLD** (V4-0, DEC:4). No code until Luke says "start coding". This card is the stub.

## 1 · Identity
| Field | Value | Source |
|---|---|---|
| Crate | `hee4-cohesion` (as a **Rust loader**) | UM:76 |
| Cluster | K3 | UM:76 |
| v3 origin | `skills` (`skills/load_skill.py`, `generate_skill_schema.py`, `skill-v1.schema.json`) | MA:15; AR:22; MIG:17 |
| Status | PLANNING — HOLD | DEC:4 |
| Deviation | v3 skills is Python outside `src/`; UM:76 places a Rust loader in K3 | UM:76; JM:38 |

**Design section:** [K3 hee4-cohesion › skills](obsidian://open?vault=herdr-engineering-engine-v4.vault&file=15%20Module%20Design/K3%20hee4-cohesion%23skills): logic flow, interfaces, S-n/actions, RL/E2E, refusals, Must not, size budget. Elaboration only; this card and the authorities win. Index: [Module Design Index](obsidian://open?vault=herdr-engineering-engine-v4.vault&file=15%20Module%20Design%2F00%20-%20Module%20Design%20Index)

## 2 · Purpose, owned state, allowed dependencies
| Item | Value | Source |
|---|---|---|
| Purpose | ~~Discover, load (scope + hash)~~ Parse and budget-include skills from bytes and digests that K6 acquires through K0h (bounded reads, the PH-6 hasher) and passes in as values. K3 stays pure, and "load" here means parse (*rev 2026-10-01 ratified V4-66, DC-43*); shares the Omission vocabulary with context and `_shape()` with workflows | AR:22; UM:76 |
| Owned state | none | UM:76 |
| Allowed deps | `hee4-contracts` | UM:63 |
| v3 edge | skills → actions (anchors.json) | E3 |

## 3 · Deployment
| Phase(s) | Entry gate | Exit evidence | Held-for-Luke |
|---|---|---|---|
| P9 milestone-2 | tag | per-slice gates (AT:88) | J10 advice needs H-8 (AT:192) |

## 4 · v3 basis
Flag **PARTIAL** — refusal sweep met; omission sites unswept, no revision store, not called from main (MA:15). Recommendation **HARDEN** — weaker shape check than workflows (two doors); wrong refusal names for bounds; schema re-read per call (AR:22).
| Finding | v3 file:line | Source | Errata |
|---|---|---|---|
| Second omission vocabulary (vs context) | — | AR:21 | — |
| Shape check weaker than workflows' `_shape()` | — | AR:22 | — |
| Schema re-read per call | — | AR:22 | — |

## 5 · Decision points
| Site · kind · note | Jev |
|---|---|
| discover/revise :155/:293 · load scope/hash :249 — EXACT (DP:126) | not-Jev |
| budget inclusion :230/:263 — JUDGMENT, first-fit (DP:127) | **J10: Jev (grant)** — Choice over skills + {none}; Noul per reference (JM:38) |

## 6 · Migrated inputs
| Path | Lines | Anchor lines |
|---|---|---|
| `migrated/v3-b5367bc/skills/load_skill.py` | 341 | 0 |
| `…/skills/generate_skill_schema.py` | 228 | 0 |
| `…/skills/skill-v1.schema.json`, `README.md`, `examples/receipt-reading/{skill.json,references/reading-a-receipt.md}` | — | 0 |
| `…/skills/README.stub.md` | — | 490 (strip, V4-10 DEC:27) |
| `…/tests/skill_schema.py` | 634 | 0 |
Hardening per MIG:17: share `_shape()` with workflows; one omission vocabulary; per-bound refusal codes; load the schema once. Depends on `schemas/actions` (migrated, MIG:17). Rust loader re-derives from these; Python stays only as reference (EXX:7 "imitate the shape, not the text"). Namespace `hee4` (V4-11).

## 7 · Quality guard
| id | why for THIS module |
|---|---|
| AP-01 | two shape checks (skills vs workflows), two omission vocabularies |
| AP-16 | first-fit budget inclusion is JUDGMENT (J10) |
| AP-19 | refusal names for bounds were wrong — assert the resolved value |
| AP-26 | parse the schema/manifest, never regex it |
| AP-45 | 490 anchor lines in README.stub.md |
| EX-01 | bounded read of a skill file |
| EX-05 | one Omission spelling |
| D-07 | one topic, one home: schema is the one definition |

## 8 · Interfaces
| Kind | Item | Source |
|---|---|---|
| Files | skill manifests + `skill-v1.schema.json` | MIG:17 |
| Jev (held) | J10 consumer | JM:81 |

## 9 · Done criteria
| # | Criterion | Evidence / read-back |
|---|---|---|
| 1 | One `_shape()` door shared with workflows | one-door census `duplicate_sites=0` |
| 2 | Per-bound refusal codes, each asserted by value | one test per bound over two fixtures |
| 3 | Schema loaded once per process | test counting loads |
| 4 | Omission sites swept (MA:15 gap) | enumerated from source; `sites=N/N` |
| 5 | Plants / mutation | plant per refusal killed by named test; scoped mutants on the Rust loader |

## 10 · Open decisions and risks
- Deviation: Python → Rust loader (UM:76) — record in DEC at the slice. "Revision store" absent (MA:15), owner UNMEASURED.

## 11 · Pull commands
```bash
E=~/hee4-evidence; R=~/herdr-engineering-engine-v4
rg -n 'skills' $E/design/ULTRAMAP.md $E/design/DECISION_POINTS-b5367bc.md
rg -n 'skills' $E/reference/v3-evidence-b5367bc/*.md $E/reference/ERRATA-v3-evidence-b5367bc.md
ls -R $R/migrated/v3-b5367bc/skills; wc -l $R/migrated/v3-b5367bc/skills/*.py $R/migrated/v3-b5367bc/tests/skill_schema.py
rg -n 'J10' $E/reference/v3-evidence-b5367bc/jev-decision-map-b5367bc.md
```
