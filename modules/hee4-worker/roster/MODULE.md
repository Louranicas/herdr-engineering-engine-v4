# hee4-worker · roster
> **Status: PLANNING — HOLD** (V4-0, DEC:4). No code until Luke says "start coding". This card is the stub.

## 1 · Identity
| Field | Value | Source |
|---|---|---|
| Crate | `hee4-worker` | UM:75 |
| Cluster | K2 (worker routing and capability) | UM:75; CMAP:3 |
| v3 origin | `roster` (`src/roster.rs`, 645 lines; types in `src/contracts/roster.rs`, 497 lines) | MA:13; MIG:11 |
| Status | PLANNING — HOLD; HARDEN module staged verbatim, not wired | DEC:6 (V4-2); MIG:5 |
| Binary | none | UM:82 |

**Design section:** [K2 hee4-worker › roster](obsidian://open?vault=herdr-engineering-engine-v4.vault&file=15%20Module%20Design/K2%20hee4-worker%23roster): logic flow, interfaces, S-n/actions, RL/E2E, refusals, Must not, size budget. Elaboration only; this card and the authorities win. Index: [Module Design Index](obsidian://open?vault=herdr-engineering-engine-v4.vault&file=15%20Module%20Design%2F00%20-%20Module%20Design%20Index)

## 2 · Purpose, owned state, allowed dependencies
| Aspect | v4 shape | Source |
|---|---|---|
| Purpose | Roster **policy**: eligibility (one door), one TTL policy, selection permits for dispatch | UM:75; AR:17 |
| Owned state | **None durable.** Roster rows (`roster_records`, `_revisions`, `_observations`, `_instances`, `_instance_history`, `_pins`, `_cancel_causes`) are written by K1 `store::roster` via app | UM:75, UM:203 |
| Retention | Ledgered retention: latest observation per record + rows pinned by an attempt; per-principal cap (pure policy, failing-first test). *(rev 2026-10-01 ratified V4-58, DC-05)* The retention policy is pure data and a pure fn in **K1** (`hee4-core`), which the store calls. This card's K2 roster only observes it: K1 cannot depend on K2 (UM §2 law) | UM:203; AT:227 |
| Allowed deps | `hee4-contracts` (K0), `hee4-host` (K0h). Never K1/K3-K6 | UM:62-66, UM:75 |
| Consumers (not deps) | K1 `store/roster.rs`, K6 `app/routing.rs` | MIG:11 |

## 3 · Deployment
| Phase(s) | Entry gate | Exit evidence | Held-for-Luke |
|---|---|---|---|
| P2 (retention policy: per-principal cap, latest-per-record kept) | P1 green | gate + scoped `cargo mutants` (`CARGO_TARGET_DIR` unset); `roster.list` count ≤ cap after N synthetic attempts | none (AT:81) |
| P3 (dispatch-time permit through the worker path) — **INTERP**: AT:82 does not name the permit; inferred from MA:13 ("Dispatch-time permit through main") | P2 | gate; plants per rule | none (AT:82) |
| P9 (roster.list/inspect/update/disable served, v4.1) | tag pushed | per-slice gate, l2 move (RT S11: +2 with B15) | H-1 for pushes (AT:88, AT:185) |
D-rows fed: indirectly D6 (dispatch permit on the spine) and the "Roster / observation growth" rollback row (AT:63, AT:227).

## 4 · v3 basis
Flag **PARTIAL** — "dispatch-time permit through main; two eligibility doors, 4 roster actions unserved" (MA:13). Recommendation **HARDEN** — sound, but a suspected release blocker in retention (AR:17).
| Finding | v3 file:line | Source | Errata |
|---|---|---|---|
| Observations capped at 4,096 **globally, never pruned**, one row per attempt and per service probe — suspected release blocker | `contracts/roster.rs:9` (`MAX_HISTORY: usize = 4096`), `store/roster.rs:615`, `app/runtime.rs:1327` | AR:17; V2:40-41; UM:42; migrated `src/contracts/roster.rs:9` | — |
| Two eligibility rules (second door: `query_snapshot`) | `roster.rs:576` | AR:17; DP:78 | — |
| Two staleness policies (TTL) | `contracts/roster.rs:272` freshness, `:343` `Selection::permits` | AR:17; DP:66 | — |
| Roster revisions recorded under the unserved `roster.update` id by the installer (pre-populates its idempotency space) | `app/native_provider.rs:273` | AR:35 (SYS HIGH); UM:135 | — |
| 4 roster actions refused (C01) | `actions/control.rs:428-440` | IM:22; UM:159-160 | E10 (thread lines shift, not roster) |
| Imports contracts only (`roster.rs:302`) | `roster.rs:302` | MIG:11; migrated `src/roster.rs:302` | — |

## 5 · Decision points
| site · kind · note | Jev |
|---|---|
| `roster` `query_snapshot :576` · EXACT · a second eligibility door (DP:78) | not-Jev (EXACT, JM:22) |
| `contracts/roster` freshness `:272` · `Selection::permits :343` · EXACT (+TTL) (DP:66) | not-Jev (EXACT/THRESHOLD, JM:22-23) |
| `store/recovery · reconciliation · roster` `selected_pins :97` · EXACT (DP:65) | not-Jev |
No JUDGMENT site in DP for roster (DP:74-131).

## 6 · Migrated inputs
| File | Lines | Anchor lines | Notes |
|---|---|---|---|
| `migrated/v3-b5367bc/src/roster.rs` | 645 | 246 (block ends `:297`) | strip the `HEE3-ANCHORS` block when brought into v4 (V4-10, DEC:27) |
| `migrated/v3-b5367bc/src/contracts/roster.rs` | 497 | 0 | the types move to K0 `hee4-contracts`; `MAX_HISTORY` 4096 becomes a K0 bound + retention policy |
| `migrated/v3-b5367bc/tests/t05_roster.rs` | 711 | 0 | primary test |
| `schemas/actions/control-v1.{request,result}.roster.{list,inspect,update,disable}.schema.json` | — | — | 8 files; regenerate from K0 emitted schema (UM:80) |
Change on entry: hee4 namespace (V4-11, DEC:31); crate path `habitat_engine::` → `hee4_worker::`; retention/cap tests written failing-first (AT:227).

## 7 · Quality guard
| id | why for THIS module |
|---|---|
| AP-04 | the 4096 cap bounds nothing downstream; the per-attempt observation set is a derived set that needs its own bound (APX AP-04 row) |
| AP-01 | two eligibility doors, two TTL policies (AR:17) |
| AP-12 | roster revisions keyed by the wire id `roster.update` (AR:35) |
| AP-13 | observations written per attempt with no retention reader (AR:17) |
| AP-19 | assert the cap off the origin: N > cap attempts, rows 2 and 3 asserted whole |
| EX-05 | one spelling per enum (InstanceState moves to K0, UM:216) |
| EX-13 | `take(N+1)` then refuse, for any roster listing |
| EX-04 | retention as a pure policy over values |
| D-09 | K2 may not import K1; persistence goes through app (UM:62-66) |
| D-01 | the P9 roster stack must name its l2 delta (RT:53) |

## 8 · Interfaces
| Kind | Item | Status | Source |
|---|---|---|---|
| Action | `roster.list`, `roster.inspect` | v4.1 (read-only) | UM:159 |
| Action | `roster.update`, `roster.disable` | v4.1 (mutating; installer decoupled first) | UM:160 |
| Internal op | roster install under `Owner::Deploy`, **not** `roster.update` | K1 op kind | UM:135 |
| Store tables | `roster_*` (7 tables) via K1 | written by store | UM:203 |

## 9 · Done criteria
| # | criterion | evidence / read-back |
|---|---|---|
| 1 | One eligibility door: exactly one site decides eligibility | one-door census `duplicate_sites=0` (DEC:37 V4-17) |
| 2 | One TTL policy | census + unit test over both former call shapes |
| 3 | Retention: after N ≫ cap synthetic attempts `roster.list` count ≤ per-principal cap, latest-per-record and attempt-pinned rows kept | gate test, both numbers printed (AT:227) |
| 4 | Installer rows under `Owner::Deploy`; `roster.update` idempotency space empty after install | ledger query in gate (UM:135) |
| 5 | Mutation: scoped `cargo mutants` on the retention/eligibility policy, survivors named or equivalent-with-reason | Tier-1 line (AT:149) |
| 6 | Plants: drop the cap check; merge TTL; each killed by a named test under `--cap-lints=warn` | `plants=k/k killers=named` (AT:149) |

## 10 · Open decisions and risks
- Per-principal cap value: UNMEASURED; decided at P2 with a failing-first test (AT:227). (The "N days" retention belongs to backup prune in the ATLAS §6 rollback map, and is carried on `hee4-app/backup-target` §10.)
- Risk: the store-side writer (`store/roster.rs`, K1 REFACTOR) and this policy drift apart — the policy must be the single door the store calls (D-09).
- v4.1 scope for roster actions depends on RT S11 ordering (UM:268).
- Resolved, not open *(rev 2026-10-01 funnel audit)*: the register rows naming this module are all RESOLVED, ratified under delegation inside H-27's range: DC-40 (V4-62): NF-ROSTER-INSTALL: phase and owner. The decision text is the V4 row in `plan/DECISIONS.md`; `hee4db highway --dc <DC-nn>` shows the row.

## 11 · Pull commands
```bash
grep -n 'roster' ~/hee4-evidence/design/ULTRAMAP.md
sed -n 13p ~/hee4-evidence/reference/v3-evidence-b5367bc/module-audit-b5367bc.md
sed -n 17p ~/hee4-evidence/reference/v3-evidence-b5367bc/architecture-review-b5367bc.md
grep -n 'roster' ~/hee4-evidence/design/DECISION_POINTS-b5367bc.md ~/hee4-evidence/design/DEPLOYMENT_ATLAS.md
ls ~/herdr-engineering-engine-v4/migrated/v3-b5367bc/src/roster.rs ~/herdr-engineering-engine-v4/migrated/v3-b5367bc/src/contracts/roster.rs ~/herdr-engineering-engine-v4/migrated/v3-b5367bc/tests/t05_roster.rs
grep -n 'MAX_HISTORY\|fn query_snapshot\|fn freshness\|fn permits' ~/herdr-engineering-engine-v4/migrated/v3-b5367bc/src/roster.rs ~/herdr-engineering-engine-v4/migrated/v3-b5367bc/src/contracts/roster.rs
grep -n 'AP-04\|AP-12\|AP-13' ~/herdr-engineering-engine-v4/docs/ANTIPATTERNS.md
```
