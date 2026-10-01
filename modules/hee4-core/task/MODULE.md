# hee4-core · task
> **Status: PLANNING — HOLD** (V4-0, DEC:4). No code until Luke says "start coding". This card is the stub.

## 1 · Identity
| Field | Value | Source |
|---|---|---|
| Crate | `hee4-core` | UM:74 |
| Cluster | K1 | UM:74 |
| v3 origin | `task` (`src/task.rs`, `task/control.rs`, `task/driver.rs`) | MIG:10; UM:107, UM:112 |
| Status | PLANNING — HOLD | DEC:4 |

**Design section:** [K1 hee4-core › task](obsidian://open?vault=herdr-engineering-engine-v4.vault&file=15%20Module%20Design/K1%20hee4-core%23task): logic flow, interfaces, S-n/actions, RL/E2E, refusals, Must not, size budget. Elaboration only; this card and the authorities win. Index: [Module Design Index](obsidian://open?vault=herdr-engineering-engine-v4.vault&file=15%20Module%20Design%2F00%20-%20Module%20Design%20Index)

## 2 · Purpose, owned state, allowed dependencies
| Item | Value | Source |
|---|---|---|
| Purpose | Task lifecycle: submission parse, `LoopGuard` (attempt/no-progress/clock), the driver that runs one attempt (work → check → stop). Parse derived from the one field list | UM:107; UM:112; AR:20 |
| Owned state | none durable (store owns rows); owns the guard's in-memory accounting | UM:74 |
| Allowed deps | `hee4-contracts` (TaskState, Generation, rc01 limits); intra-crate `recovery` | UM:63; CMAP:14 (task→recovery, intra-K1) |
| v3 edge | `task/control.rs:26` imports `recovery::TaskState` → becomes K0 enum | MIG:10; UM:212 |

## 3 · Deployment
| Phase(s) | Entry gate | Exit evidence | Held-for-Luke |
|---|---|---|---|
| P1 contract spine | P0 green | typed TaskState; submission parse; gate + test through `main` (AT:80) | — |
| P2 task.* flows | P1 | task.submit/get/list/cancel/resolve L2 +1-2 (AT:81) | — |
| P5 B16 fail→repair→verify→accept | P4 | F07 host record; F15 socket case (AT:84; RT:45,47) | — |
Feeds D6 (AT:63) and D8 (AT:65).

## 4 · v3 basis
Flag **PARTIAL** — 6 lifecycle actions + one composer through main; B16 not composed (MA:9). Recommendation **HARDEN** — driver repeats the mask and deadline the guard owns (AR:20).
| Finding | v3 file:line | Source | Errata |
|---|---|---|---|
| Driver re-checks criteria mask + post-pass deadline (second deadline door) | `task/driver.rs:150` | AR:20; DP:58; AP-07 | — |
| Guard should expose `declared()` / `may_accept()`; `Generation::FIRST` | — | AR:20; UM:74 | — |
| rc01 limits re-exported (`TASK_LIMIT, CLEANUP_RESERVE, MAX_ATTEMPTS, MAX_NO_PROGRESS`) | `task.rs:337` (migrated copy) | grep of MIG copy | — |
| Submission parse hand-written; derive from the one field list | `task/control.rs:100` | UM:107; AR:19 | — |
| t28_socket lacks task.cancel/list/resolve case (F15) | — | RT:25 | — |

## 5 · Decision points
| Site · kind · note | Jev |
|---|---|
| LoopGuard::new :420 · record_reconciled_attempt :502 · observe_clock :529 — EXACT (DP:55) | not-Jev |
| begin_attempt :449-468 — THRESHOLD (EXACT order): TASK_LIMIT, CLEANUP_RESERVE, MAX_ATTEMPTS, MAX_NO_PROGRESS (DP:56) | not-Jev (JM:23) |
| task/driver work map :133 · check map :141 · stop :174 — EXACT (DP:57) | not-Jev |
| task/driver post-pass deadline :150 — THRESHOLD, a second deadline door (DP:58) | not-Jev |
| task/control spec :205 · budget :255 — EXACT, local_only, tokens/currency zero (DP:59); criteria/list/note bounds THRESHOLD (DP:60) | not-Jev |

## 6 · Migrated inputs
| Path | Lines | Anchor lines | Note |
|---|---|---|---|
| `migrated/v3-b5367bc/src/task.rs` | 540 | 330 | strip anchor block (V4-10, DEC:27); the `HEE3-ANCHORS` block spans lines 1–330 (measured 2026-10-01, V13 B5; was 279) |
| `…/src/task/control.rs` | 564 | 0 | `use crate::recovery::TaskState` → `hee4_contracts` |
| `…/src/task/driver.rs` | 180 | 0 | drop the repeated mask/deadline |
| `…/tests/t01_task.rs` | 406 | — | primary test (MIG:25) |
| `…/tests/t06_driver.rs` | 657 | — | primary test |
Depends on **recovery** (KEEP, not migrated — MIG:10, :23). Rename `hee3`→`hee4` paths/protocol per V4-11 (DEC:31). No `t09_route.rs:2352` coupling here.

## 7 · Quality guard
| id | why for THIS module |
|---|---|
| AP-07 | driver re-acquires what the guard owns — pass the proof value |
| AP-01 | deadline decided twice (guard + driver :150) |
| AP-02 | TaskState must be the K0 enum, never text |
| AP-19 | `Generation` of a first attempt is the identity element — assert attempts 2 and 3 |
| AP-31 | every loop/await in the driver names its budget |
| AP-45 | 330 anchor lines to strip from `task.rs` (lines 1–330) |
| EX-06 | guard predicates read their arms' values |
| EX-14 | `Generation(NonZeroU64)`, `Generation::FIRST` |
| EX-15 | proof-carrying value instead of re-check |
| D-03 | skeleton compiles before any B16 design review |
| D-11 | B16/F15 stack must move l2 |

## 8 · Interfaces
| Kind | Item | Source |
|---|---|---|
| Actions (parse) | task.preview/submit/get/list/cancel/resolve (v4.0) | UM:157 |
| Flow hops | 14 submission parse; 19 driver run; 3b.1 get/list parse | UM:107, UM:112, UM:120 |
| External | none | — |

## 9 · Done criteria
| # | Criterion | Evidence / read-back |
|---|---|---|
| 1 | Driver calls `guard.declared()` / `guard.may_accept()`; no second mask or deadline check | one-door census `duplicate_sites=0` (V4-17) |
| 2 | Parse derived from the one K0 field list | round-trip test over all fields vs schema |
| 3 | F15: cancel before dispatch, list with cursor, resolve through the socket | socket test through `main` (RT:45) |
| 4 | B16 repair loop composed | F07 host record committed and recomputed (AT:84) |
| 5 | Plants | a planted second deadline check or a mask drop killed by a named test under `--cap-lints=warn` (AT:149) |
| 6 | Mutation | scoped `cargo mutants` on guard with `CARGO_TARGET_DIR` unset; survivors named (REQ rank 7) |

## 10 · Open decisions and risks
- B16 L2 kind decided as host record in v3 route (RT:35); re-record in v4 register at P5.
- Risk: t28-style tests near a 180 s partition (AT:98; RT:70).
- Recovery migration is "on Luke's word" per MIG:23 — v4 plans it as a re-derivation in this crate (see recovery card).
- Resolved, not open *(rev 2026-10-01 funnel audit)*: the register rows naming this module are all RESOLVED, ratified under delegation inside H-27's range: DC-06 (V4-57): UM §5c Stop row lists `queued` (removed) and `failed` (terminal) as sources; DC-07 (V4-57): Two readings of UM §5c rows: (a) "any → Resolve" would let a terminal task leave its terminal state; (b) "running (cancel requested) +…; DC-34 (V4-57): Stop from `admitted`; DC-37 (V4-57): Transition legality, two homes. The decision text is the V4 row in `plan/DECISIONS.md`; `hee4db highway --dc <DC-nn>` shows the row.

## 11 · Pull commands
```bash
E=~/hee4-evidence; R=~/herdr-engineering-engine-v4
rg -n 'task' $E/design/ULTRAMAP.md | head -40
rg -n '^\| task' $E/design/DECISION_POINTS-b5367bc.md
rg -n 'task' $E/reference/v3-evidence-b5367bc/{module-audit,architecture-review,route-to-v010}-b5367bc.md
ls -la $R/migrated/v3-b5367bc/src/task* $R/migrated/v3-b5367bc/src/task/ $R/migrated/v3-b5367bc/tests/t0{1_task,6_driver}.rs
rg -n 'use crate|HEE3-ANCHORS' $R/migrated/v3-b5367bc/src/task.rs $R/migrated/v3-b5367bc/src/task/*.rs | head
rg -n 'AP-0[127]|AP-19|AP-31|EX-06|EX-1[45]' $R/docs/ANTIPATTERNS.md $R/docs/EXEMPLARS.md
```
