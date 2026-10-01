# hee4-contracts · state-enums
> **Status: PLANNING — HOLD** (V4-0, DEC:4). No code until Luke says "start coding". This card is the stub.

## 1 · Identity
| Field | Value | Source |
|---|---|---|
| Crate | `hee4-contracts` | UM:71 |
| Cluster | K0 | UM:71 |
| v3 origin | `recovery` state enums (`recovery.rs:308-370`), `Generation` (`contracts.rs:278-306`), verdict spellings (`receipt/records.rs:101`), native `ProviderState` (`native.rs:508`), roster `InstanceState` (`contracts/roster.rs:388`) | UM:71, UM:212-218; EX-14 |
| Status | PLANNING — HOLD | DEC:4 |

**Design section:** [K0 hee4-contracts › state-enums](obsidian://open?vault=herdr-engineering-engine-v4.vault&file=15%20Module%20Design/K0%20hee4-contracts%23state-enums): logic flow, interfaces, S-n/actions, RL/E2E, refusals, Must not, size budget. Elaboration only; this card and the authorities win. Index: [Module Design Index](obsidian://open?vault=herdr-engineering-engine-v4.vault&file=15%20Module%20Design%2F00%20-%20Module%20Design%20Index)

## 2 · Purpose, owned state, allowed dependencies
| Item | Value | Source |
|---|---|---|
| Purpose | **One definition** of every durable-state enum; the store writes only what K1 `transition()` returns over these types | UM:36 (P2), UM:209 |
| Owned state | None (types). The `transition(State, Event)` fn lives in K1 core, not here | UM:74 |
| Enums | TaskState (12 → 11, `queued` dropped); AttemptState/Effect/Cleanup; Verdict (one, was 3 spellings); Mode; ProviderState, InstanceState; Disposition, ObligationKind; Join/Blocked, Omission, Report (one each); DataClass is on judge-types | UM:212-219 |
| Cancellation | encoded in the waiting variant (`EffectUnknown{cancel: bool}`), one source | UM:37 (P3), UM:235; D-U5 (DEC:16) |
| Allowed deps | serde only | UM:71 |

## 3 · Deployment
| Phase | Entry gate | Exit evidence | Held |
|---|---|---|---|
| P0 | H-5 | skeleton compiles | H-5 (AT:189) — AT:79 |
| P1 | P0 green | typed `TaskState` + K1 pure `transition`; property test over all 11×\|Event\| pairs | none — AT:80, UM:241 |
Feeds D6 (`state=accepted` read back, AT:63) and D9 (`task.list state=accepted`, AT:66).

## 4 · v3 basis
- **Flag:** recovery PARTIAL — policy complete, restore marker always None, two cursor doors (MA:11). store PARTIAL (MA:10).
- **Recommendation:** recovery **KEEP** — "hand its state enums to contracts (with the store refactor)" (AR:28); store REFACTOR — state is a String, no transition fn (AR:6).

| Finding | v3 file:line | Source | Errata |
|---|---|---|---|
| Task state is a `String` at ≥9 write sites; no transition fn | `store.rs:751,796,828` (`pub state: String`) | AR:6; AP-02 | site list not persisted (ER §2) |
| `TaskState` enum exists but store does not use it | `recovery.rs:338` | AP-02; A-1 | — |
| Cancellation held twice (flag + state) | `store.rs:2239-2245` | UM:37 | — |
| `queued` never written; `repair_pending` IS written | `store.rs:1536`, `store/verification.rs:248` | UM:22 (C1) | **E4** |
| Verdict names spelled 3× | `recovery.rs` Verdict, VerificationVerdict, `receipt/records.rs:101` | UM:214 | — |
| herdr reads task states as strings | `herdr.rs:617` | AR:24; UM:212 | — |
| Generation written as literal `"1"` despite `Generation(NonZeroU64)` | `store.rs:1252-1253`; `contracts.rs:280` | A-2; EX-14 | — |

## 5 · Decision points
| Site · kind · note | Jev |
|---|---|
| recovery `reconcile R01-R14 :874-1297 · lease_refusal :1109 · permits_execution :839` · EXACT · pure policy (KEEP) (DP:69) — consumer of these enums | not-Jev |
| store `settle_attempt :1531 · begin :2612 · accept … · next_dispatchable :1439` · EXACT · "state is a String; no transition fn" (DP:62) | not-Jev |

## 6 · Migrated inputs
`recovery` is **not migrated** (KEEP; migrate on Luke's word, MIG:23). From the migrated set: `src/contracts.rs:278-306` (`Generation`, EX-14; strip anchor block `:1-166`, DEC:27) and `src/herdr.rs` (consumer; `TaskStateV1` to move here, AR:24). `src/task/control.rs:26` imports `recovery::TaskState` (MIG:10) — that import re-points to `hee4_contracts`.

## 7 · Quality guard
| id | why here |
|---|---|
| AP-02 | the whole reason this card exists |
| AP-03 | phase enums carry only phase-valid data |
| AP-12 | one spelling; `parse → None`, never a default |
| AP-13 | `queued`: no writer → not a variant |
| AP-19 | `Generation::FIRST` must be asserted off the origin (rows 2 and 3) |
| AP-21 | the transition table's KAT must come from v3 CHECK constraints + recorded event logs, not from the write-site list (UM:241) |
| EX-05, EX-14, EX-06 | one spelling per enum; NonZero generation; predicates read arm values |
| A-1, A-2 | text state inside the door; literal `"1"` |
| D-10 | types refuse instead of a census |

## 8 · Interfaces
Read by every cluster: `task.get/list` state (UM:122), dispatcher `Dispatchable` typed state (UM:111), herdr projection (UM:212), ledger CHECK constraints generated from the enum (UM:194-195).

## 9 · Done criteria
| # | Criterion | Evidence |
|---|---|---|
| 1 | One definition per enum in §5b | grep over the workspace: each enum name defined once (`duplicate_sites=0`, DEC:37) |
| 2 | `TaskState` has 11 variants; no `Queued` in Task or Attempt | compile; UM:212, UM:257 |
| 3 | Cancellation is a field of the waiting variant; no `cancellation` column | schema read-back (UM:194) |
| 4 | Property test enumerates all 11×\|Event\| pairs against UM §5c; expected table sourced from v3 CHECK constraints + recorded logs | test names its independent source (UM:241) |
| 5 | Scoped mutants on `parse`/`name` (CARGO_TARGET_DIR unset) | `survivors=0` or each equivalent with reason (AT:132) |

## 10 · Open decisions and risks
- D-U5 is "proposed" (DEC:16); UM:235 already assumes it.
- The UM §5c table is not independent of the code (F113, UM:241): risk of pinning v3 behaviour as spec.
- `recovery` migration needs Luke's word (MIG:23).
- Resolved, not open *(rev 2026-10-01 funnel audit)*: the register rows naming this module are all RESOLVED, ratified under delegation inside H-27's range: DC-06 (V4-57): UM §5c Stop row lists `queued` (removed) and `failed` (terminal) as sources; DC-07 (V4-57): Two readings of UM §5c rows: (a) "any → Resolve" would let a terminal task leave its terminal state; (b) "running (cancel requested) +…; DC-14 (V4-56): Home of shared vocabularies (Join/Blocked, Omission): K0 (UM §5b) vs K3; DC-34 (V4-57): Stop from `admitted`; DC-37 (V4-57): Transition legality, two homes. The decision text is the V4 row in `plan/DECISIONS.md`; `hee4db highway --dc <DC-nn>` shows the row.

## 11 · Pull commands
```bash
cd ~/herdr-engineering-engine-v4
sed -n 36,37p ~/hee4-evidence/design/ULTRAMAP.md; sed -n 207,239p ~/hee4-evidence/design/ULTRAMAP.md
sed -n 6p ~/hee4-evidence/reference/v3-evidence-b5367bc/architecture-review-b5367bc.md; sed -n 28p ~/hee4-evidence/reference/v3-evidence-b5367bc/architecture-review-b5367bc.md
rg -n '^\| E4 ' ~/hee4-evidence/reference/ERRATA-v3-evidence-b5367bc.md
sed -n 278,306p migrated/v3-b5367bc/src/contracts.rs
rg -n 'TaskState|recovery' migrated/v3-b5367bc/src/task/control.rs migrated/v3-b5367bc/src/herdr.rs | head
rg -n 'D-U5' plan/DECISIONS.md
rg -n '### (EX-05|EX-14|A-1|A-2)' docs/EXEMPLARS.md
```
