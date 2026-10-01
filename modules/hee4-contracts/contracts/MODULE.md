# hee4-contracts · contracts
> **Status: PLANNING — HOLD** (V4-0, DEC:4). No code until Luke says "start coding". This card is the stub.

## 1 · Identity
| Field | Value | Source |
|---|---|---|
| Crate | `hee4-contracts` (leaf; serde only) | UM:71 |
| Cluster | K0 | UM:56, UM:71 |
| v3 origin | `contracts` (`src/contracts.rs`, `contracts/{control,events,principal,rc01,roster,receipt}.rs`, `contracts/receipt/{codec,invariants,primitives,records}.rs`) | MIG:9; `ls migrated/v3-b5367bc/src/contracts` |
| Scope of this card | wire (control-v1 frame, `admit_object`, `receive`), envelope (Request/Reply incl. `Reply::Stream` shape), receipt records (RC04), `Principal`, `EventCursorV1`, roster wire types. Bounds, state enums, catalogue data and judge types have their own cards | brief; UM:71 |
| Status | PLANNING — HOLD | DEC:4 |

**Design section:** [K0 hee4-contracts › contracts](obsidian://open?vault=herdr-engineering-engine-v4.vault&file=15%20Module%20Design/K0%20hee4-contracts%23contracts): logic flow, interfaces, S-n/actions, RL/E2E, refusals, Must not, size budget. Elaboration only; this card and the authorities win. Index: [Module Design Index](obsidian://open?vault=herdr-engineering-engine-v4.vault&file=15%20Module%20Design%2F00%20-%20Module%20Design%20Index)

## 2 · Purpose, owned state, allowed dependencies
| Item | Value | Source |
|---|---|---|
| Purpose | Every wire type and record every cluster may import; the only thing all clusters share | UM:56, UM:71 |
| Owned state | **None.** Owns types only | UM:71 |
| Allowed deps | serde only. Depended on by every crate | UM:62-66, UM:71 |
| Shape changes | Generate `Deserialize`+`Validate` from one field list (AR:19 → UM:71); bound `EventCursorV1` (MA:8); `Reply::Stream` variant for `events.subscribe` (AR SYS; UM:101) | AR:19, MA:8, UM:101, DEC:34 |
| Ports | P6: a cluster exposes its trait here; app wires the impl | UM:40 |

## 3 · Deployment
| Phase | Entry gate | Exit evidence | Held |
|---|---|---|---|
| P0 skeleton | "start coding" (H-5) | crate compiles in the 9-crate workspace; planted cross-cluster `use` fails with E0432/E0433 (asserted on the diagnostic) | H-5 (AT:189) — AT:79, UM:84 |
| P1 contract spine | P0 green | control-v1 framing ≤ 1 MiB + version refusal; **RA10 fuzz test from day one** (seeded, std-only, budgeted), plant killed under `--cap-lints=warn` | none — AT:80 |
| P5 | P4 | `Reply::Stream` carried by `events.subscribe` | none — AT:84, DEC:34 |
| P6 | P5 | RA10 security lens over contracts; 0 HIGH open | H-6 (lens upgrade, not block) — AT:85, AT:190 |
Feeds D3 (health through the socket, AT:60) and D8 (scoreboard, AT:65).

## 4 · v3 basis
- **Flag:** PARTIAL — frame bound, version refusal, socket/in-process parity; EventCursorV1 unbounded, schema pinned one way, no old/new fixture pair (MA:8).
- **Recommendation:** HARDEN — 62 RC04 records spell fields 3×; the `validate()` copy is not compiler-checked; attempt limit decided twice (AR:19).

| Finding | v3 file:line | Source | Errata |
|---|---|---|---|
| Frame bound `MAX_FRAME_BYTES` re-typed as `1_048_576` in store and coordinator | `contracts/control.rs:23`; `store.rs:1236`, `coordinator.rs:53` | AR:36; UM:39; A-4 | — |
| Byte scanner before serde | `contracts/control.rs:213` `admit_object` | DP:67; RT:12 (S4 context) | — |
| `wire::receive` ≤1 MiB, version 1 | `contracts/control.rs:1166` | UM:103 | — |
| `EventCursorV1` unbounded | `contracts/events.rs:7` (migrated copy) | MA:8; UM:42 | — |
| Receipt cross-field requires | `contracts/receipt/invariants.rs:63-214` | DP:68 | — |
| Declared deps "contracts only" is not what anchors.json says | — | CMAP:3 | E3 |
| `Reply` is `{Close, Frame}`; streaming inexpressible | — | AR:34 | — |
| No fuzz test (0 files) | — | RT:12 | — |

## 5 · Decision points
| Site · kind · note | Jev |
|---|---|
| contracts/control `admit_object :213` · EXACT · byte scanner before serde (DP:67) | not-Jev (EXACT, JM:22) |
| contracts/receipt `invariants::check :63-214` · EXACT · cross-field requires (DP:68) | not-Jev |
| contracts/roster `freshness :272 · Selection::permits :343` · EXACT (+TTL) (DP:66) | not-Jev; TTL is THRESHOLD (JM:23) |

## 6 · Migrated inputs
| Path (under `migrated/v3-b5367bc/`) | Lines | Note |
|---|---|---|
| `src/contracts.rs` | 388 | anchor block `:1-166` (128 anchor lines) — strip on import (DEC:27, V4-10); `Generation` at `:280` (EX-14) |
| `src/contracts/{control,events,principal,rc01,roster,receipt}.rs` | 1491/96/59/22/497/68 | 0 anchor lines each |
| `src/contracts/receipt/{codec,invariants,primitives,records}.rs` | 98/276/390/4212 | `records.rs` 4,212 lines: split target for AP-08 |
| `schemas/receipts/{receipt-v1.schema.json,generate_receipt_schema.py}` | — | one schema generating both sides (P15, UM:49) |
| tests: `t01_contracts.rs`, `t03_contract.rs`, `t05_codec.rs`, `t06_receipts.rs`, `t06_receipt_import.rs`, `t28_control.rs`, `receipt_schema.py` | 308/1013/303/1073/512/1151/496 | `t01_contracts.rs:13` does `include_str!("../docs/contract-decisions.md")`, a v3 doc that was not copied; t03 → worker; t06_receipt_import → app, check, store; t28_control → store (MIG:28-33) |
**On import:** rename `hee3.control` → the `hee4` namespace (`contracts/control.rs:29`; V4-11, DEC:31); strip anchor block; drop the v3-doc `include_str!` or supply a v4 source; the dependent tests wait for their crates.

## 7 · Quality guard
| id | why here |
|---|---|
| AP-01 | frame bound re-typed at 2+ sites (A-4) |
| AP-04 / AP-05 | `MAX_FRAME_BYTES` is checked inside the frame reader before any `split` or parse, and the `EventCursorV1` bound when the cursor is decoded; the caller that receives the bounded value verifies it at its flow, not here (§9 #2, #4) *(rev 2026-10-01 open-tasks CN-19: V5 generic row made module-specific)* |
| AP-14 | never clamp a caller bound; refuse by name |
| AP-21 | fuzz "Scanner-accept ⇒ serde-accept" is an independent oracle only because serde is another implementation |
| AP-26 | generate Validate from one field list; never regex declarations |
| AP-45 | anchor block in `contracts.rs` must not cross |
| EX-01, EX-19, EX-14, EX-15 | bounded frame reader; one digest door; `Generation(NonZeroU64)`; `Principal` private fields |
| A-4 | the re-typed frame bound |
| D-09, D-10 | leaf crate is the compiler door; prefer types over censuses |

## 8 · Interfaces
Served to every crate: control-v1 frame (≤1 MiB, v1; UM:103), `Request/Reply{Close,Frame,Stream}` (UM:126), `EventCursorV1` (UM:125), receipt records (`hee4 task.get` evidence view, UM:123). The socket itself is K6 (UM:146). Schema emitted for bash/Pi (UM:80, UM:151).

## 9 · Done criteria
| # | Criterion | Evidence |
|---|---|---|
| 1 | Every RC04 record's Deserialize+Validate generated from one field list | a planted field-rename fails compile, not a test (AR:19) |
| 2 | `MAX_FRAME_BYTES` has one definition site | one-door census `duplicate_sites=0` (DEC:37) |
| 3 | RA10 fuzz: seeded, std-only, iteration budget printed with both numbers; plant (Scanner/serde divergence) killed by the named test under `--cap-lints=warn` | `plants=k/k killers=named` (AT:149; RT:46) |
| 4 | `EventCursorV1` bounded, refusal by name with both numbers | test off the origin (AP-19) |
| 5 | Old/new schema fixture pair, fixture from a recording (F113) | MA:8 gap closed |
| 6 | Scoped `cargo mutants` (CARGO_TARGET_DIR unset) on codec/admit | `mutants caught=a survivors=b` each named (AT:149, AT:132) |

## 10 · Open decisions and risks
- `Reply::Stream` wire shape undecided in detail (V4-14 decides *that*, not *how*; DEC:34).
- `records.rs` 4,212 lines: split plan owed (AP-08).
- Independence of receipt KATs: must come from recorded v3 receipts, not the same head (AP-21).
- Resolved, not open *(rev 2026-10-01 funnel audit)*: the register rows naming this module are all RESOLVED, ratified under delegation inside H-27's range: DC-08 (V4-55): K0 dependencies: UM says "serde only"; the hardened code needs `serde_json` and `sha2`; DC-28 (V4-56): `OwnerPort` home. The decision text is the V4 row in `plan/DECISIONS.md`; `hee4db highway --dc <DC-nn>` shows the row.

## 11 · Pull commands
```bash
cd ~/herdr-engineering-engine-v4
sed -n 71p ~/hee4-evidence/design/ULTRAMAP.md; sed -n 103p ~/hee4-evidence/design/ULTRAMAP.md
sed -n 8p ~/hee4-evidence/reference/v3-evidence-b5367bc/module-audit-b5367bc.md
sed -n 19p ~/hee4-evidence/reference/v3-evidence-b5367bc/architecture-review-b5367bc.md
sed -n 66,68p ~/hee4-evidence/design/DECISION_POINTS-b5367bc.md
sed -n 74,75p ~/hee4-evidence/design/DEPLOYMENT_ATLAS.md
rg -n 'contracts' migrated/v3-b5367bc/MIGRATION.md
ls -l migrated/v3-b5367bc/src/contracts migrated/v3-b5367bc/src/contracts/receipt
rg -n 'pub const|pub fn (admit_object|receive)' migrated/v3-b5367bc/src/contracts/control.rs
rg -n 'EX-0?1 |EX-19|EX-14|EX-15|A-4 ' docs/EXEMPLARS.md
rg -n 'AP-01|AP-04|AP-05|AP-14|AP-21|AP-26' docs/ANTIPATTERNS.md | cut -c1-160
```
