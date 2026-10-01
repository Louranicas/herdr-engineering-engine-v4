# hee4-habitat · herdr
> **Status: PLANNING — HOLD** (V4-0, DEC:4). No code until Luke says "start coding". This card is the stub.

## 1 · Identity
| Field | Value | Source |
|---|---|---|
| Crate | `hee4-habitat` | UM:78 |
| Cluster | K5 | UM:78 |
| v3 origin | `herdr` (`src/herdr.rs`, 1,044 lines) | MA:18; AR:24; MIG:15 |
| Status | PLANNING — HOLD; HARDEN module staged verbatim, not wired | DEC:6 (V4-2); MIG:5 |

**Design section:** [K5 hee4-habitat › herdr](obsidian://open?vault=herdr-engineering-engine-v4.vault&file=15%20Module%20Design/K5%20hee4-habitat%23herdr): logic flow, interfaces, S-n/actions, RL/E2E, refusals, Must not, size budget. Elaboration only; this card and the authorities win. Index: [Module Design Index](obsidian://open?vault=herdr-engineering-engine-v4.vault&file=15%20Module%20Design%2F00%20-%20Module%20Design%20Index)

## 2 · Purpose, owned state, allowed dependencies
| Aspect | v4 shape | Source |
|---|---|---|
| Purpose | The herdr projection: a client value model presenting engine tasks in the herdr multiplexer; submit/observe/present/reconnect | UM:78; DP:118 |
| Owned state | none durable | UM:78 |
| Target shape | `TaskState` from K0 (not strings); bounded frame decoder with a version check; bound every `Snapshot` field; correct the authority doc | AR:24; UM:78 |
| Allowed deps | `hee4-contracts`, `hee4-host` | UM:62-66 |
| Advice display | shows Jev advice **labelled as advice** (if granted) | JM:83 |

## 3 · Deployment
| Phase(s) | Entry gate | Exit evidence | Held-for-Luke |
|---|---|---|---|
| P1 (TaskState in K0 unblocks the string match) | P0 | gate | none (AT:80) |
| P9 milestone-2 (herdr composed; not called from main in v3) | tag pushed | per-slice gate, l2 move | H-1 (AT:88, AT:185) |
D-rows fed: none in v4.0.

## 4 · v3 basis
Flag **PARTIAL** — "client value model built; no decoder, no cancel request, not called from main" (MA:18). Recommendation **HARDEN** — "task states as strings; no decoder or version check; module doc overstates authority" (AR:24).
| Finding | v3 file:line | Source | Errata |
|---|---|---|---|
| Task states matched as strings (one of three TaskState definitions) | `herdr.rs:617` (`from_engine(state: &str)` at `:615`) | UM:212; DP:118; migrated `src/herdr.rs:615` | — |
| Reads state `queued`, which nothing writes | `herdr.rs:617` | UM:22 (C1) | **E4** |
| Event text bound derived from the frame bound (good: one door) | `herdr.rs:319`, `:337` | migrated `src/herdr.rs:319,337` | — |
| Source imports contracts only; test `t16_herdr` imports **recovery** (not migrated) | `tests/t16_herdr.rs:1069` | MIG:15, MIG:31 | — |

## 5 · Decision points
| site · kind · note | Jev |
|---|---|
| from_engine `:615` · submit `:847` · observe `:904` · present/reconnect `:956` · EXACT (DP:118) | not-Jev (JM A6: herdr status is a typed enum, decide in code — JM:54) |

## 6 · Migrated inputs
| File | Lines | Anchor lines | Notes |
|---|---|---|---|
| `migrated/v3-b5367bc/src/herdr.rs` | 1,044 | 228 (block ends `:279`) | strip anchor block (V4-10, DEC:27) |
| `migrated/v3-b5367bc/tests/t16_herdr.rs` | 1,556 | 0 | imports `habitat_engine::recovery::TaskState` (`:1069`) → re-point to `hee4_contracts` TaskState |
| `migrated/v3-b5367bc/deploy/herdr/README.stub.md` | — | 378 | tooling stub, deploy card owns it |
Entry changes: hee4 namespace (V4-11, DEC:31); crate path; `queued` arm removed (TaskState 12 → 11, UM:212).

## 7 · Quality guard
| id | why for THIS module |
|---|---|
| AP-02 | task states as strings (AR:24) |
| AP-04 | unbounded `Snapshot` fields; decoder must bound at acquisition |
| AP-15 | module doc overstates authority (AR:24) |
| AP-13 | reads `queued`, never written (E4) |
| EX-01 | bounded frame reader with version refusal (EXX:15) |
| EX-05 | one spelling per enum, `parse → None` (EXX:19) |
| EX-19 | the frame bound read from its one door (`MAX_FRAME_BYTES`) |
| D-09 | K5 imports K0 only |

## 8 · Interfaces
Consumes engine task state (K0 `TaskState`) and frames of the control-v1 shape (≤ 1 MiB, version 1; UM:103). herdr plugin surface: `deploy/herdr/` stub (IM none; RT:14).

## 9 · Done criteria
| # | criterion | evidence / read-back |
|---|---|---|
| 1 | No `&str` state match: states decoded to K0 `TaskState` | grep 0 string compares; planted unknown state refused by name |
| 2 | Frame decoder bounds bytes before parse and refuses version ≠ 1 | fuzz/plant killed by named test under `--cap-lints=warn` |
| 3 | Every `Snapshot` field bounded | test per field with N+1 input, both numbers in the refusal |
| 4 | Anchor block stripped; `v3_refs=0` | independence check (DEC:28) |
| 5 | Scoped mutants over decoder | Tier-1 (AT:149) |

## 10 · Open decisions and risks
- Cancel request absent in v3 (MA:18): scope for v4 undecided.
- Risk: t16 relies on recovery's TaskState; recovery enums move to K0 (UM:278) — do both in one step.

## 11 · Pull commands
```bash
grep -n 'herdr' ~/hee4-evidence/design/ULTRAMAP.md ~/hee4-evidence/design/DECISION_POINTS-b5367bc.md
sed -n 18p ~/hee4-evidence/reference/v3-evidence-b5367bc/module-audit-b5367bc.md
sed -n 24p ~/hee4-evidence/reference/v3-evidence-b5367bc/architecture-review-b5367bc.md
grep -n 'herdr' ~/herdr-engineering-engine-v4/migrated/v3-b5367bc/MIGRATION.md
grep -n 'HEE3-ANCHORS-END\|fn from_engine\|MAX_FRAME_BYTES' ~/herdr-engineering-engine-v4/migrated/v3-b5367bc/src/herdr.rs
grep -n 'recovery' ~/herdr-engineering-engine-v4/migrated/v3-b5367bc/tests/t16_herdr.rs
```
