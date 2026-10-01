# hee4-app · actions (registry + control)
> **Status: PLANNING — HOLD** (V4-0, DEC:4). No code until Luke says "start coding". This card is the stub.

## 1 · Identity
| Field | Value | Source |
|---|---|---|
| Crate | `hee4-app` (registry + control); catalogue **data** (`Action`/`Owner`/`Effect`) moves to K0 `hee4-contracts` | UM:71, UM:79 |
| Cluster | K6 | UM:79 |
| v3 origin | `actions` → `src/actions.rs`, `src/actions/control.rs` (+ `schemas/actions/`) | MA:20; MIG:16 |
| Status | PLANNING — HOLD | DEC:4 |

**Design section:** [K6 hee4-app › actions](obsidian://open?vault=herdr-engineering-engine-v4.vault&file=15%20Module%20Design/K6%20hee4-app%23actions): logic flow, interfaces, S-n/actions, RL/E2E, refusals, Must not, size budget. Elaboration only; this card and the authorities win. Index: [Module Design Index](obsidian://open?vault=herdr-engineering-engine-v4.vault&file=15%20Module%20Design%2F00%20-%20Module%20Design%20Index)

## 2 · Purpose, owned state, allowed dependencies
- **Purpose:** serve control-v1 requests: `serve_composed` → replay check → admit (catalogue lookup by content-pinned catalogue) → dispatch **by owner registry** `Registry::get(action.owner)` → `&dyn OwnerPort` (UM:102-106).
- **Owned state:** the owner registry (K6 owns it, UM:79). Replay/idempotency moves to the K1 primitive (UM:104, P9 UM:43). No durable state.
- **Allowed deps:** any (K6). `worker/tools.rs` projection moves to K0 catalogue, which removes the K2→K6 edge (UM:75; E6).
- **Dropped:** `UNSERVED` list + string `match` (unserved = no registry entry = refusal by name); `Action.cli` field (never read) (UM:252-253).

## 3 · Deployment
| Phase(s) | Entry gate | Exit evidence | Held-for-Luke |
|---|---|---|---|
| P1 (dispatch by `Owner` registry; health) | P0 | gate; binary test through main (AT:80) | — |
| P2 (task.* through socket, F15 shape) | P1 | task flows L2 +1-2 (AT:81) | — |
| P5 (`events.subscribe` + `Reply::Stream`; cancel/list/resolve through socket, F15) | P4 | gate; l2 +3 (AT:84) | — |
| P9 (roster.* v4.1; service/thread/analysis v4.2; judge.inspect held) | tag | per-slice gates (AT:88) | H-8 (judge.inspect), H-14 (analysis prod) |
Feeds D3, D6, D8 (AT:60, :58, :60).

## 4 · v3 basis
- **Flag:** PARTIAL — one 21-action catalogue, named unavailability; Pi doesn't read the tool column, 2 of the needed owner traits, no mutation gate (MA:20).
- **Recommendation:** HARDEN — dispatch on strings with 3 lists that must agree; `Dispatch` token dropped before use; `cli` never read (AR:25; SYS rec 1 AR:41).

| Finding | v3 file:line | Source | Errata |
|---|---|---|---|
| String dispatch `match action.id`; `UNSERVED` list | actions/control.rs:479, :418 | UM:44; migrated file | — |
| UNSERVED thread.get/list, events.subscribe | :420/:424, :456 | ER:20 | E10 |
| `Owner` exists on every Action | actions.rs:504 (enum :345) | UM:44; migrated file | — |
| Replay from `Effect::mutates()` | actions.rs:485 | UM:43 | — |
| `cli` field never read | actions.rs:508 | AR:25; migrated file | — |
| Roster revisions recorded under unserved `roster.update` by installer | native_provider.rs:273 | AR:35; UM:135 | — |
| Two catalogues (JSON schema vs CATALOGUE) pinned by count only | — | AR:36; UM:49 (P15) | — |
| actions → task undeclared edge | — | CMAP:10; E1 | E1 |

## 5 · Decision points
| site · kind · note | Jev |
|---|---|
| admit `actions/control.rs:343` · replay_expired `actions/control.rs:313` · EXACT (DP:100; locators corrected DP:20-21) | not-Jev |
| actions/tasks cursor lifetimes, MAX_QUERY_BYTES · THRESHOLD (DP:101) | not-Jev |

## 6 · Migrated inputs
| Path (under `migrated/v3-b5367bc/`) | Note |
|---|---|
| `src/actions.rs` (1,131 lines; 255 anchor lines), `src/actions/control.rs` (763 lines) | strip anchor block (V4-10, DEC:27); rename `hee3.control`/`habitat-engine` strings to `hee4` (V4-11) |
| `schemas/actions/*` (control-v1 request/result/error schemas, `generate_control_schema.py`, `README.stub.md` 999 anchor lines) | v4: schema emitted from K0 (P15, UM:49); pin by content digest |
| tests `t28_actions.rs` (→ worker, not migrated), `t28_control.rs` (→ store, not migrated), `control_schema.py` | MIG:28-33 test deps |
Changes when brought in: registry replaces `match`/`UNSERVED`; delete `cli`; `dispatch` takes `Dispatch` (AR:25); catalogue data to K0 (UM:71). 49 files in MIG table (MIG:16).

## 7 · Quality guard
| id | why for THIS module |
|---|---|
| AP-12 | string dispatch with 3 lists that must agree is this module's v3 defect |
| AP-07 | `Dispatch` token dropped before use — take it |
| AP-09 | `cli` field dead pub API |
| AP-01 | catalogue spelled twice (schema vs CATALOGUE) |
| AP-28 | catalogue pinned by count, not content |
| AP-10 / A-10 | `worker/tools.rs` projection compiled into lib, no production caller (E6) |
| EX-05 | one spelling per enum, `parse → None` |
| EX-19 | one door for a digest (catalogue content pin) |
| D-09 | compiler refuses K2→K6 once tools move to K0 |
| D-07 | one catalogue home (K0) |

## 8 · Interfaces
Serves 21 actions + `judge.inspect` (held): v4.0 health, tools.list/inspect, task.preview/submit/get/list/cancel/resolve, events.subscribe (P5); v4.1 roster.*; v4.2 service.*, thread.*, analysis.* (UM:156-164). Refusal by name for any action without a registry entry.

## 9 · Done criteria
| # | criterion | evidence |
|---|---|---|
| 1 | No string `match` on action id; no UNSERVED list | `rg -n 'match action.id\|UNSERVED' crates/hee4-app` → 0 |
| 2 | Unregistered action refused by name | socket test per unserved id |
| 3 | Catalogue pinned by content digest; schema emitted from K0 | gate test comparing digests (UM:49) |
| 4 | task.cancel/list/resolve through socket (F15) | gate L2 (RT:45; AT:84) |
| 5 | Mutation gate owed in v3 exists in v4 | scoped mutants on registry/admit, `CARGO_TARGET_DIR` unset (AT:132); plants `--cap-lints=warn` (MA:20) |

## 10 · Open decisions and risks
- 2 owner traits missing in v3 (MA:20): roster (`RosterActions`, DS5-open, RT:53), service, cohort, notify owners each land with their phase.
- Risk: t09/t28 test coupling to anchor text; check each migrated test for `HEE3-ANCHORS` before stripping (V4-10).
- Resolved, not open *(rev 2026-10-01 funnel audit)*: the register rows naming this module are all RESOLVED, ratified under delegation inside H-27's range: DC-24 (V4-56): `judge.inspect` owner; DC-28 (V4-56): `OwnerPort` home. The decision text is the V4 row in `plan/DECISIONS.md`; `hee4db highway --dc <DC-nn>` shows the row.

## 11 · Pull commands
```bash
sed -n 100,127p ~/hee4-evidence/design/ULTRAMAP.md; sed -n 151,162p ~/hee4-evidence/design/ULTRAMAP.md
rg -n 'actions' ~/hee4-evidence/design/DECISION_POINTS-b5367bc.md
sed -n 20p ~/hee4-evidence/reference/v3-evidence-b5367bc/module-audit-b5367bc.md; sed -n 25p ~/hee4-evidence/reference/v3-evidence-b5367bc/architecture-review-b5367bc.md
ls ~/herdr-engineering-engine-v4/migrated/v3-b5367bc/src/actions* ~/herdr-engineering-engine-v4/migrated/v3-b5367bc/schemas/actions
rg -n 'UNSERVED|match action.id|pub cli|enum Owner' ~/herdr-engineering-engine-v4/migrated/v3-b5367bc/src/actions.rs ~/herdr-engineering-engine-v4/migrated/v3-b5367bc/src/actions/control.rs
rg -n '^\| (E6|E10)' ~/hee4-evidence/reference/ERRATA-v3-evidence-b5367bc.md
```
