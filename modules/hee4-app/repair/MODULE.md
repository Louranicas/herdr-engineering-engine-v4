# hee4-app · repair
> **Status: PLANNING — HOLD** (V4-0, DEC:4). No code until Luke says "start coding". This card is the stub.

## 1 · Identity
| Field | Value | Source |
|---|---|---|
| Crate | `hee4-app` | UM:79 |
| Cluster | K6 | UM:65 |
| v3 origin | `app` → `src/app/repair.rs` (`apply`, `apply_candidate`, class bounds) | DP:90-91 |
| Status | PLANNING — HOLD | DEC:4 |

**Design section:** [K6 hee4-app › repair](obsidian://open?vault=herdr-engineering-engine-v4.vault&file=15%20Module%20Design/K6%20hee4-app%23repair): logic flow, interfaces, S-n/actions, RL/E2E, refusals, Must not, size budget. Elaboration only; this card and the authorities win. Index: [Module Design Index](obsidian://open?vault=herdr-engineering-engine-v4.vault&file=15%20Module%20Design%2F00%20-%20Module%20Design%20Index)

## 2 · Purpose, owned state, allowed dependencies
| Aspect | Content | Source |
|---|---|---|
| Purpose | Apply a repair candidate after a failed verdict: inventory the change, enforce the class's byte/changed-line bounds, hand the result back to verification | DP:90-91 |
| State path | `verifying --Verdict{Failed}--> repair_pending --Begin--> running` (written by K1 `transition`) | UM:225, UM:230 |
| Owned state | None; the repair_pending state is K1's (and IS written in v3) | UM:22 (C1); ER:14 (E4) |
| Allowed deps | K0 (bounds, TaskState), K1 (via transition), K2 (candidate execution) via K6 | UM:65 |

## 3 · Deployment
| Phase(s) | Entry gate | Exit evidence | Held-for-Luke |
|---|---|---|---|
| **P5 System scenario**: B16 shape admit → dispatch → fail → repair → verify → accept through `main`, as host record F07 | P4 | host record F07 committed; gate; l2 +1 for F07 | none |
| P7 real GPU task incl. one fail → repair → accept (D6) | P6 | D6 read-back, `task.get` → accepted | H-7 if journal read |
Sources: AT:63, AT:84, AT:86; RT:47.

## 4 · v3 basis
- **Flag:** `app` PARTIAL — "B16 … absent" (MA:19); task PARTIAL — "B16 (lost reply → repair → verified accept) not composed" (MA:9).
- **Recommendation:** `app` REFACTOR (AR:7).

| Finding | v3 file:line | Source | Errata |
|---|---|---|---|
| B16 repairs through `repair::apply_candidate`, not route fallback (so B15 is not a tag gate) | — | RT:27 (correction 6, INTERP) | — |
| apply / apply_candidate inventory | `repair.rs:176`, `:290` | DP:90 | — |
| class bounds (bytes, changed lines) | `repair.rs:~310` (approximate in source) | DP:91 | — |
| `repair_pending` is written (settle + verdict Failed); only `queued` is never written | `store.rs:1536`, `store/verification.rs:248` | UM:22 | **E4** (ER:14) |

## 5 · Decision points
| site · kind · note | Jev |
|---|---|
| `apply/apply_candidate inventory :176/:290` · EXACT (DP:90) | — |
| `class bounds :~310` · THRESHOLD · bytes, changed lines (DP:91) | not-Jev (JM:23) |
| related: repair feedback template in candidates `render history` (DP:35) | J3 grant — owned by `candidates` card |

## 6 · Migrated inputs
None — `app` is REFACTOR (MIG:22).

## 7 · Quality guard
| id | why for THIS module |
|---|---|
| AP-02 | repair_pending must be a `TaskState` variant reached only via `transition`, never a string |
| AP-01 | class bounds have one definition site (K0 / class profile), read here |
| AP-04 | the diff/inventory is a derived set: bound it by bytes and changed lines before building it |
| AP-19 | test over ≥ 2 repair rounds, second round asserted whole (not only attempt 1) |
| AP-14 | an over-bound repair is refused by name with both numbers, never truncated |
| EX-02 | the settle/accept goes through the one write door, deadline before COMMIT |
| EX-13 | `take(N+1)` then refuse for the inventory bound |
| D-01 | this module's value is F07 moving l2; land it with the scenario |

## 8 · Interfaces
| Kind | Item | Source |
|---|---|---|
| store (via K1) | `transition(Verdict{Failed})` → repair_pending; `transition(Begin)` | UM:225, UM:230 |
| candidates | next candidate rendered with failure history | DP:35 |
| actions | none directly; observable via `task.get` | UM:120-123 |

## 9 · Done criteria
| # | criterion | evidence / read-back |
|---|---|---|
| 1 | B16 end to end through `main`: fail → repair → verify → accept | host record F07, recomputed by the scoreboard (AT:84) |
| 2 | Over-bound repair refused by name with both numbers | test over two fixtures (bytes, lines) |
| 3 | No state write outside `transition` | property test over the §5c table (UM:241) |
| 4 | Plants on bound comparison and inventory killed by named tests under `--cap-lints=warn`; scoped mutants, `CARGO_TARGET_DIR` unset | `plants=k/k killers=named`; `mutants caught= survivors=` (AT:149) |

## 10 · Open decisions and risks
- The exact v3 bounds line is approximate (`:~310`, DP:91) — re-derive the bounds from the class profile, do not copy.
- B16's L2 kind is a host record (RT:35, INTERP ruling) — record it in DEC at P5's flow contract.
- The TaskState transition table's source is not independent of v3 code (F113, UM:241).
- Resolved, not open *(rev 2026-10-01 funnel audit)*: the register rows naming this module are all RESOLVED, ratified under delegation inside H-27's range: DC-45 (V4-66): Who consumes Jev advice for J1/J2/J3/J7. The decision text is the V4 row in `plan/DECISIONS.md`; `hee4db highway --dc <DC-nn>` shows the row.

## 11 · Pull commands
```bash
V4=~/herdr-engineering-engine-v4; EV=~/hee4-evidence; R=$EV/reference/v3-evidence-b5367bc
rg -n 'repair' $EV/design/*.md $R/*.md $EV/reference/ERRATA-v3-evidence-b5367bc.md
sed -n '22p;219,239p' $EV/design/ULTRAMAP.md
sed -n '58p;79p' $EV/design/DEPLOYMENT_ATLAS.md
sed -n '9p;19p' $R/module-audit-b5367bc.md; sed -n '27p;35p;47p' $R/route-to-v010-b5367bc.md
rg -n '^\| \*\*AP-(01|02|04|14|19)\*\*' $V4/docs/ANTIPATTERNS.md
```
