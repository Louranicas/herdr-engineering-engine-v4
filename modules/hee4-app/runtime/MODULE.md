# hee4-app · runtime
> **Status: PLANNING — HOLD** (V4-0, DEC:4). No code until Luke says "start coding". This card is the stub.

## 1 · Identity
| Field | Value | Source |
|---|---|---|
| Crate | `hee4-app` | UM:79 |
| Cluster | K6 (composition; the only crate that sees more than one cluster, and the only dependant of K0e) | UM:65-66, UM:79 |
| v3 origin | `app` → `src/app/runtime.rs` (3,814 lines) | AR:7; UM:79; V2:40 |
| v4 shape | `runtime::{ports, lifecycle, settle, publish}` | AR:7; UM:79 |
| Also owns | the `DataClass` check before K0e (held; judgment flow) | UM:79, UM:133 |
| Status | PLANNING — HOLD | DEC:4 |

**Design section:** [K6 hee4-app › runtime](obsidian://open?vault=herdr-engineering-engine-v4.vault&file=15%20Module%20Design/K6%20hee4-app%23runtime): logic flow, interfaces, S-n/actions, RL/E2E, refusals, Must not, size budget. Elaboration only; this card and the authorities win. Index: [Module Design Index](obsidian://open?vault=herdr-engineering-engine-v4.vault&file=15%20Module%20Design%2F00%20-%20Module%20Design%20Index)

## 2 · Purpose, owned state, allowed dependencies
- **Purpose:** composition of one task's lifecycle: drive (`runtime::drive` → `task::driver::run`), begin attempt, execute via K2+K0h, verify via K4 `decide` only (through the live-verifier adapter), accept via `transition(Accept)` (UM:112-114).
- **Owned state:** none durable. Composition only: wiring, ports, lifecycle; all durable state is K1's (UM:74, UM:79).
- **Allowed deps:** all crates (K0, K0h, K0e, K1-K5) — K6 only (UM:65). **Rule:** no cluster crate may import it; it must not re-implement a cluster policy (P6 ports live in contracts, UM:40).
- **Split:** `ports` (trait impls K6 wires, P6), `lifecycle` (drive/begin), `settle` (one settle per attempt), `publish` (accept + ~~outbox insert~~ a call to the store's one outbox fn; runtime inserts no row itself, *rev 2026-10-01 ratified V4-56, DC-21*; read by `events.subscribe`) (AR:7; UM:114, UM:125).

## 3 · Deployment
| Phase(s) | Entry gate | Exit evidence | Held-for-Luke |
|---|---|---|---|
| P1 (skeleton + `serve`/`health` through `main`) | P0 green | gate; t28-style binary test through `main` (AT:80) | H-5 (P0) |
| P3 (execute/verify wiring; SIGHUP handled like SIGTERM) | P2 | plants per rule incl. one making the adapter emit a verdict itself (AT:82) none at P3; H-9 is needed by P4 (its ATLAS §5 row reads "P4 R-exec row") |
| P5 (B16 fail → repair → verify → accept through `main`; publish → outbox reader) | P4 | host record F07 committed; gate; l2 +3 (AT:84) | none |
Feeds D6 (task accepted end to end, AT:63), D9 (`used`, AT:66), D3 (health line, AT:60).

## 4 · v3 basis
- **Flag:** app PARTIAL — serve, dispatcher, commission, backup gate, drain through main; B16, ACT-G12 table, D03 fault matrix, A18 `#[expect]` cleanup absent (MA:19).
- **Recommendation:** REFACTOR — runtime.rs 3,814 lines, hub of cycles runtime ↔ live_verifier/candidates/plan; Clock seam stops at the dispatcher (AR:7).

| Finding | v3 file:line | Source | Errata |
|---|---|---|---|
| God-file + intra-app cycle `runtime.rs:16` → live_verifier → `live_verifier.rs:22` → runtime; also `candidates.rs:15` | runtime.rs:16 | AP-08 (ANTIPATTERNS row), A-5 | — |
| Cycles break because live_verifier becomes an observation-only adapter; verdict from K4 `decide` | app/live_verifier.rs:25-28 | UM:79; DEC:33 (V4-13) | E15 |
| Roster observation written per attempt, global 4096 cap, never pruned | runtime.rs:1327 | AR:17 (row 12); AP-04 | — |
| Hops: drive `runtime.rs:937`; begin `:2024`; execute/verify/accept `:2134,:2226,:2336` | runtime.rs | UM:112-114 | — |
| Outbox written on accept (`store.rs:2024`) and on stop (`store/terminal.rs:219`), never delivered | store.rs:2024 | AR:33; UM:23 | E5 |

## 5 · Decision points
| site · kind · note | Jev |
|---|---|
| app/runtime prepare :1103 · capture_share :914 · after_ready :2480 · execute/verify :2134/:2226 · accept :2336 · EXACT (DP:81) | not-Jev (EXACT) |
| app/runtime CHECK_TEARDOWN :268 · THRESHOLD · grace (DP:82) | not-Jev (THRESHOLD, JM:23) |

## 6 · Migrated inputs
None — app is REFACTOR; redesign from findings, not carried verbatim (MIG:22). Namespace: binary `hee4`, runtime dir `$XDG_RUNTIME_DIR/hee4/` (V4-11, DEC:31).

## 7 · Quality guard
| id | why for THIS module |
|---|---|
| AP-08 | runtime.rs is the v3 god-file and cycle hub; print `largest_file=` at each cut (D-05) |
| AP-06 | Clock seam stopped at the dispatcher; thread Clock through runtime (AR:7) |
| AP-01 | verdict must come only from K4 `decide`; the adapter emitting a verdict = second door (UM:38) |
| AP-13 | publish writes outbox rows; each needs its reader (`events.subscribe`, P5) |
| AP-04 | per-attempt roster observation is a derived set; bound it (runtime.rs:1327) |
| AP-31 | every exit-path await (drain, seal) needs a budget |
| EX-02 | one write door, deadline before COMMIT — the accept path imitates it |
| EX-07 | pure settle step — `runtime::settle` shape |
| A-5 | God-file with an intra-app cycle: do not rebuild it |
| D-09 | K6 alone sees many clusters; compiler enforces the rest |
| D-05 | `largest_file=` printed at every cut |
| D-11 | runtime slices must move l2 (F06/F07) or excuse zero delta |

## 8 · Interfaces
- Uses: K1 `transition`/store ports, K2 execute, K0h spawn, K4 `decide` via the live-verifier adapter (UM:114); K0e egress only after the `DataClass` check (held, UM:133).
- Serves: task lifecycle behind `task.submit` (dispatch), outbox rows for `events.subscribe` (UM:158, V4-14).
- Files: state under `~/.local/state/hee4/` through K1 only (UM:181).

## 9 · Done criteria
| # | criterion | evidence / read-back |
|---|---|---|
| 1 | No file in `runtime::` > 1,500 lines (AP-08 TRIGGER column, APX:52; its symptom column says > ~2,000); no intra-app module cycle | cut prints `largest_file=` (D-05); a `use`-graph census over `crates/hee4-app/src` prints `cycles=0` (**to build at "start coding"**; `cargo metadata` sees crates, not `mod`s, so it cannot show this) |
| 2 | Ledger verdict = `verdict_of(decide)` only | plant: adapter emits its own verdict → named test fails (AT:82) |
| 3 | B16 scenario accepted through `main` on host | F07 host record committed (AT:84); `hee4 task.get <id>` → `state=accepted` (AT:63) |
| 4 | Outbox rows have a production reader | `acknowledge_delivery` has a caller; `events.subscribe` gate test (AT:84) |
| 5 | SIGHUP handled like SIGTERM; drain bounded | test of drain budget with both numbers (AT:135, AT:228) |
| 6 | Mutation | scoped `cargo mutants` on settle/publish with `CARGO_TARGET_DIR` unset (AT:132); plants under `--cap-lints=warn` naming the killing test (AT:149) |

## 10 · Open decisions and risks
- `DataClass` grant record and egress: H-8, H-10, H-11, H-12 (AT:192-197) — ports ship dark.
- Risk: A18 `#[expect]` cleanup and D03 fault matrix owed from v3 (MA:19) — no v4 home yet besides this card.
- Seal time for TimeoutStopSec measured in P4 (AT:239).
- Resolved, not open *(rev 2026-10-01 funnel audit)*: the register rows naming this module are all RESOLVED, ratified under delegation inside H-27's range: DC-15 (V4-64): Verdict seal: `Verdict` is a K0 enum, so no K4-private constructor exists; DC-23 (V4-64): Verdict hand-off. The decision text is the V4 row in `plan/DECISIONS.md`; `hee4db highway --dc <DC-nn>` shows the row.

## 11 · Pull commands
```bash
rg -n 'hee4-app|runtime' ~/hee4-evidence/design/ULTRAMAP.md
sed -n 90,135p ~/hee4-evidence/design/ULTRAMAP.md            # flows through runtime
sed -n 74,83p ~/hee4-evidence/design/DEPLOYMENT_ATLAS.md       # phases
rg -n 'app/runtime' ~/hee4-evidence/design/DECISION_POINTS-b5367bc.md
sed -n 7p ~/hee4-evidence/reference/v3-evidence-b5367bc/architecture-review-b5367bc.md
sed -n 19p ~/hee4-evidence/reference/v3-evidence-b5367bc/module-audit-b5367bc.md
rg -n 'AP-08|AP-06|A-5' ~/herdr-engineering-engine-v4/docs/ANTIPATTERNS.md ~/herdr-engineering-engine-v4/docs/EXEMPLARS.md
rg -n 'V4-13|V4-14' ~/herdr-engineering-engine-v4/plan/DECISIONS.md
```
