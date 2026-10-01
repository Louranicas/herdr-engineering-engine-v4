# tooling · bash
> **Status: PLANNING — HOLD** (V4-0, DEC:4). No code until Luke says "start coding". This card is the stub.

## 1 · Identity
| Field | Value | Source |
|---|---|---|
| Crate | none — **outside the crate DAG** (tooling) | UM:80; MIG:18 |
| Cluster | outside (v3 design placed bash in K6; V1 corrected MIG to "outside") | CMAP:3; V1:37 |
| v3 origin | `bash` (`integrations/bash/hee3`, README.md, README.stub.md; `tests/bash_wrapper.py`) | MA:21; MIG:18 |
| v4 name | ~~the `hee4 …` bash wrapper (v3 `hee3`)~~ **`hee4-sh`** (*rev 2026-10-01 ratified V4-56, DC-19*). The name is distinct from the binary `hee4`, so `hee4` on `PATH` has one meaning | UM:151; DEC:31 (V4-11) |
| Status | PLANNING — HOLD | DEC:4 |

**Design section:** [T tooling › bash](obsidian://open?vault=herdr-engineering-engine-v4.vault&file=15%20Module%20Design/T%20tooling%23bash): logic flow, interfaces, S-n/actions, RL/E2E, refusals, Must not, size budget. Elaboration only; this card and the authorities win. Index: [Module Design Index](obsidian://open?vault=herdr-engineering-engine-v4.vault&file=15%20Module%20Design%2F00%20-%20Module%20Design%20Index)

## 2 · Purpose, owned state, allowed dependencies
| Item | Value | Source |
|---|---|---|
| Purpose | Shell front end: builds a control-v1 request, runs the `hee4` binary as producer, prints the reply; `chain` runs bounded step sequences | IM:33; UM:127 |
| Owned state | None; generated or pinned artifact only | UM:80 |
| Reads | **K0's emitted schema** for every bound (no retyped literals). *(rev 2026-10-01 ratified V4-56, DC-18)* The wrapper's constants are **generated from** the schema at build time, so nothing reads the schema at run time. UM wins | UM:80, UM:151 |
| Must | refuse a deadline over 60 s by name (no clamp, no silent default); move the Python out of heredocs into `.py` files | UM:80; AR:26 |

## 3 · Deployment
| Phase | Entry gate | Exit evidence | Held-for-Luke |
|---|---|---|---|
| P1 Contract spine (INTERP: the schema it reads is emitted by K0 here) | P0 green | shellcheck control + `bash_wrapper.py` through a real `hee4 health` (AT:80) | none |
| P5 System scenario (INTERP: F15 consumers — cancel/list/resolve through the socket) | P4 | gate (AT:84) | none |
Acceptance line: "The wrapper clamps deadlines that the engine refuses" is a v3 debt v4 must not inherit (AT:95). Feeds D8 (AT:65).

## 4 · v3 basis
Flag **PARTIAL** (MA:21): "All wrapper semantics, two compositions, shellcheck control; the site sweep excludes bash `fail`/builder sites". Recommendation **HARDEN** (AR:26): bounds retyped; **clamps the deadline where the engine refuses**; two Python programs in heredocs → read bounds from the schema; refuse over 60 s; move the Python to .py files.

| Finding | v3 file:line | Source | Errata |
|---|---|---|---|
| Non-digit or ≤ 0 budget → `DEFAULT_MS` (30 s) silently | `hee3:264` | AP-14; A-9; DP:132 | — |
| `min(ahead, MAX_AHEAD_MS)` clamp to 60 s | `hee3:275` | AP-14; A-9 | — |
| Bounds retyped (`MAX_ARGS=64`, `MAX_OUTPUT_BYTES=1048576`, …) | `hee3:35-49` (migrated copy) | AR:26 | — |
| Python in heredocs | `hee3:102`, `:187`, `:426` (migrated copy) | AR:26 | — |
| Wrapper semantics + shellcheck control are reusable | — | UM:287 | — |

## 5 · Decision points
| site · kind · note | Jev |
|---|---|
| bash deadline default :264 · THRESHOLD · **silent clamp and fallback** (DP:132) | not-Jev (THRESHOLD) |
| bash chain step verdict :790 · EXACT (DP:133) | not-Jev |

## 6 · Migrated inputs
| File | Lines | Anchor lines | Change when brought in |
|---|---|---|---|
| `migrated/v3-b5367bc/integrations/bash/hee3` | 906 | 0 | rename to the `hee4` wrapper; env `HEE3_TIMEOUT_MS`, `HEE3_CHAIN_STEP` → `HEE4_*` (`hee3:263`, `:371`); refuse >60 s; bounds from schema |
| `…/integrations/bash/README.md` | 251 | 0 | rewrite for `hee4` |
| `…/integrations/bash/README.stub.md` | 423 | 356 | strip the anchor block (DEC:27, V4-10) |
| `migrated/v3-b5367bc/tests/bash_wrapper.py` | 1,734 | 0 | keep; add a >60 s refusal case and a non-digit refusal case |
| `migrated/v3-b5367bc/schemas/actions/*` | — | — | input; regenerated from K0 (UM:80) |
MIG:18 counts 4 files / 3,314 lines. The migrated copy calls the v3 binary name (`habitat-engine`); v4 calls `hee4` (DEC:31).

## 7 · Quality guard
| id | why for THIS module |
|---|---|
| AP-14 | the v3 wrapper is the canonical silent clamp (A-9) |
| AP-01 | bounds retyped in the wrapper are a second door on K0's constants |
| AP-41 | shell scaffolding re-derives primitives (exit codes through pipes) |
| AP-29 | the chain verdict must come from each step's exit code, not the last line |
| AP-39 | `chain` must not continue past a failed step |
| A-9 | do not re-create the deadline clamp |
| EX-01 | imitate: refuse by name with both numbers |
| D-07 | one topic, one home: bounds live in K0 only |

## 8 · Interfaces
| Kind | Item | Source |
|---|---|---|
| CLI | ~~`hee4 <action> …`, `hee4 --check`, `hee4 chain …`~~ `hee4-sh <action> …`, `hee4-sh --check`, `hee4-sh chain …` (*rev 2026-10-01 ratified V4-56, DC-19*) (v3 `hee3:873-900`) | IM:33; UM:151 |
| Socket | none directly — reaches the socket only through the `hee4` binary | IM:11 |
| Env | allowlisted child env (`hee3:366-371` in v3) | IM:66 |

## 9 · Done criteria
| # | criterion | evidence / read-back |
|---|---|---|
| 1 | Deadline > 60 s refused by name with both numbers; non-digit refused | `bash_wrapper.py` cases; plant re-adding the clamp is killed by a named case |
| 2 | Zero retyped bounds | every bound read from the emitted schema; one-door census `duplicate_sites=0` (UM:39) |
| 3 | No heredoc Python | `grep -c "<<'PY'"` = 0 on the wrapper; `.py` files pass `ast.parse`/ruff |
| 4 | shellcheck control | the control plants a violation and asserts shellcheck's own diagnostic (AP-22) |
| 5 | Wrapper and engine agree | a request the engine refuses is refused by the wrapper with the same reason name |

## 10 · Open decisions and risks
- Phase placement is INTERP (the ATLAS names no phase for the wrapper).
- The fail/builder site sweep excluded in v3 (MA:21) is owed.

## 11 · Pull commands
```bash
E=~/hee4-evidence; R=$E/reference/v3-evidence-b5367bc; M=~/herdr-engineering-engine-v4/migrated/v3-b5367bc
sed -n '39p;80p;127p;149p;258p;285p' $E/design/ULTRAMAP.md
sed -n '75p;79p;90p' $E/design/DEPLOYMENT_ATLAS.md
sed -n '132,133p' $E/design/DECISION_POINTS-b5367bc.md
sed -n '26p' $R/architecture-review-b5367bc.md; sed -n '21p' $R/module-audit-b5367bc.md; sed -n '11p;33p;66p' $R/interface-map-b5367bc.md
sed -n '18p' $M/MIGRATION.md
sed -n '260,276p' $M/integrations/bash/hee3
grep -n 'readonly MAX\|<<.PY' $M/integrations/bash/hee3
grep -n 'A-9' ~/herdr-engineering-engine-v4/docs/EXEMPLARS.md
```
