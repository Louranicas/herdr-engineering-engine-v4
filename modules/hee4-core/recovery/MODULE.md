# hee4-core · recovery
> **Status: PLANNING — HOLD** (V4-0, DEC:4). No code until Luke says "start coding". This card is the stub.

## 1 · Identity
| Field | Value | Source |
|---|---|---|
| Crate | `hee4-core` (policy); its enums → `hee4-contracts` | UM:71, UM:74 |
| Cluster | K1 | UM:74 |
| v3 origin | `recovery` (`src/recovery.rs`; enums `:308-370`, `TaskState :338`) | UM:71; UM:212-215 |
| Status | PLANNING — HOLD | DEC:4 |

**Design section:** [K1 hee4-core › recovery](obsidian://open?vault=herdr-engineering-engine-v4.vault&file=15%20Module%20Design/K1%20hee4-core%23recovery): logic flow, interfaces, S-n/actions, RL/E2E, refusals, Must not, size budget. Elaboration only; this card and the authorities win. Index: [Module Design Index](obsidian://open?vault=herdr-engineering-engine-v4.vault&file=15%20Module%20Design%2F00%20-%20Module%20Design%20Index)

## 2 · Purpose, owned state, allowed dependencies
| Item | Value | Source |
|---|---|---|
| Purpose | Pure startup reconcile policy R01-R14 (durable state vs observations) that recovery runs to `complete` before any dispatch; also after `restore` | UM:239; UM:278; AT:226 |
| Owned state | none (pure) | AR:28 |
| Allowed deps | `hee4-contracts` (enums moved there) | UM:74; UM:278 |
| v3 edges | 0 crate edges (not even contracts) | E2 |

## 3 · Deployment
| Phase(s) | Entry gate | Exit evidence | Held-for-Luke |
|---|---|---|---|
| P1 enums to K0 (with `transition`) | P0 | gate (AT:80) | — |
| P2 restore → recovery to complete | P1 | restore drill in gate world (AT:81) | — |
| P7 G11 rehearsal of `07-recover` | P6 | rehearsal record per-step `rc=` (AT:86) | H-7 journal (AT:191) |
Feeds **D3** `recovery=complete` (AT:60), **D5**(b) (AT:62), **D7** (AT:64).

## 4 · v3 basis
Flag **PARTIAL** — policy complete, startup through main; restore marker always None, two cursor doors (MA:11). Recommendation **KEEP** — a sound pure policy module; hand its enums to contracts (AR:28).
| Finding | v3 file:line | Source | Errata |
|---|---|---|---|
| Pure policy with named rules (exemplar) | `recovery.rs:296-300`, `:866-911` | EX-04 | — |
| One spelling per enum, `parse → None` | `recovery.rs:305-406` | EX-05 | — |
| Predicate reads its arms' values | `recovery.rs:835-845` | EX-06 | — |
| Restore marker always None (`restored_from` never persisted) | — | MA:10-11 | — |
| Anchor block at `recovery.rs:1-295` | — | AP-45 | — |
| Not migrated: KEEP; "migrate on Luke's word" | — | MIG:23 | — |

## 5 · Decision points
| Site · kind · note | Jev |
|---|---|
| reconcile R01-R14 :874-1297 · lease_refusal :1109 · permits_execution :839 — EXACT, pure policy (DP:69) | not-Jev |

## 6 · Migrated inputs
None — KEEP, but **not copied** (MIG:23). Migrated `task/control.rs:26` and `tests/t16_herdr.rs` depend on it (MIG:10, :31). ~~**Migrate vs re-derive is OPEN: ATLAS H-26, Luke's word at P2 (design register DC-27).** If re-derived, it is built from its exemplars (EX-04/05/06) (EXX:7) *(rev 2026-10-01 design-set integrator)*.~~ **Migrate** (*rev 2026-10-01 ratified V4-59, DC-27, H-26 closed under delegation; reversible*). At "start coding", v3 `src/recovery.rs` from `b5367bc` is staged verbatim into `migrated/v3-b5367bc/` (MANIFEST + `sha256sum -c`, as V4-2 did). They are then brought into K1 with only these deltas, each named in the slice record: the anchor strip (AP-45, V4-10), enums imported from K0, `queued` arms deleted, the `cancellation` flag reads replaced by the state variant (D-U5), and `restored_from` persisted (MA:10-11). EX-04/05/06 stay the shapes to keep. Enum text spellings: `queued` dropped (UM:212-213; E4).

## 7 · Quality guard
| id | why for THIS module |
|---|---|
| AP-06 | keep every rule a pure fn over values (no clock, no /proc) |
| AP-02 | enums are the one state vocabulary everyone else imports |
| AP-18 | reconcile tests must assert computed values, not branches |
| AP-45 | v3's 295-line anchor block |
| EX-04, EX-05, EX-06 | the three exemplars live here |
| D-09 | enums in K0 so K5 herdr need not import K1 |
| D-10 | policy proven by types + KATs, not a census |

## 8 · Interfaces
| Kind | Item | Source |
|---|---|---|
| Called by | serve startup (K6), `hee4 restore` | UM:239; UM:149 |
| Health | `health ready=true recovery=complete` | AT:60 |

## 9 · Done criteria
| # | Criterion | Evidence / read-back |
|---|---|---|
| 1 | All state enums defined once in K0, recovery imports them | grep: 0 enum definitions outside `hee4-contracts` (UM:209-219) |
| 2 | R01-R14 each has a case asserting the resolved value; KAT answers from v3 ledger CHECK constraints / recorded logs, not this code | test list `rules=14/14` (UM:241; F113) |
| 3 | `restored_from` persisted and reported after restore | restore drill line (AT:62) |
| 4 | Mutation | scoped `cargo mutants`, runner-owned target dir, survivors named or equivalent-with-reason (AT:149) |
| 5 | Plants | neuter each rule (`if true`) in turn → named test fails (AP-24) |

## 10 · Open decisions and risks
- v3 "migrate on Luke's word" (MIG:23) vs v4 re-derivation: ~~record which in DEC at P1.~~ ~~OPEN: ATLAS H-26, Luke's word at P2 (DC-27) *(rev 2026-10-01 open-tasks CN-27)*.~~ Decided: **migrate** (*rev 2026-10-01 ratified V4-59*; Luke may revoke).
- Risk: reconcile after `restore` into a fresh generation untested in v3 (no restore verb, AT:62).

## 11 · Pull commands
```bash
E=~/hee4-evidence; R=~/herdr-engineering-engine-v4
rg -n 'recovery' $E/design/ULTRAMAP.md $E/design/DEPLOYMENT_ATLAS.md
rg -n 'recovery' $E/design/DECISION_POINTS-b5367bc.md $E/reference/ERRATA-v3-evidence-b5367bc.md
rg -n 'recovery' $E/reference/v3-evidence-b5367bc/*.md $R/migrated/v3-b5367bc/MIGRATION.md
sed -n '/### EX-04/,/### EX-07/p' $R/docs/EXEMPLARS.md
```
