# Migration map: v3 HARDEN modules → v4 staging (2026-10-01, louranicas-2c, on Luke's instruction)

**What this is:** a verbatim copy of the 11 modules the architecture review flagged **HARDEN**, plus their primary tests and the two schema directories they read. Source: v3 commit **`b5367bc`** (= origin/main), taken with `git archive` (committed bytes, no working tree). Integrity: `MANIFEST.sha256` (103 files; `sha256sum -c MANIFEST.sha256` from this directory).

**Status:** STAGED, NOT WIRED. No Cargo.toml, not built, nothing edited. Luke's HOLD: no code changes until he says "start coding". This is a reference copy for the v4 crate layout (ULTRAMAP), not a build input.

| Module | v4 cluster (ULTRAMAP, pending) | Files | Lines | Hardening owed (architecture-review-b5367bc.md) | Depends on, NOT migrated |
|---|---|---|---|---|---|
| contracts | K0 `hee4-contracts` *(rev 2026-10-01, V1)* | 20 | 20,947 | Generate each RC04 record's Deserialize+Validate from one field list; the attempt limit is decided twice; two strict-JSON scanners documented | none |
| task | K1 core | 5 | 2,347 | The guard owns mask and deadline (`declared()`, `may_accept`); `Generation::FIRST`; TaskState from contracts | **recovery** (KEEP; `task/control.rs:26` imports `recovery::TaskState`) |
| roster | K2 | 3 | 1,659 | One eligibility door; one TTL policy; **observation retention + per-principal cap (suspected release blocker, S1b)** lives in store/roster | none (contracts only: `roster.rs:302`). The modules that USE it — store/roster.rs (store, REFACTOR) and app/routing.rs (app, REFACTOR) — are consumers, not dependencies *(rev 2026-10-01, V1)* |
| route | K2 | 2 | 6,352 | Typed rule per Step kind; keep the admitted policy in Routing; split route/config.rs | none (contracts only: `route.rs:312-313`). app/routing.rs (app, REFACTOR) uses it; it is a consumer *(rev 2026-10-01, V1)* |
| context | K3 | 2 | 3,007 | One Omission vocabulary; BTreeSet `seen`/Permit + MAX_PERMIT; permit before depth; first-fit stated or stopped | **budget** (REFACTOR; value types `Amount, Provenance, Unit, Usage`) |
| numerical | K4 | 4 | 2,620 | `ProcessReport::settled()` and one pinned hasher in worker; pin the Julia depot; compose or park | **worker** (REFACTOR; `worker::process`); julia/ (REFACTOR) |
| herdr | K5 | 2 | 2,600 | TaskStateV1 in contracts; a bounded frame decoder with a version check; bound every Snapshot field; correct the authority doc | source: contracts only. **Tests:** `t16_herdr` depends on **recovery** (not migrated) *(rev 2026-10-01, V1)* |
| actions | K6 | 49 | 40,554 | Dispatch by Owner registry; `dispatch` takes `Dispatch`; derive or delete `cli`; body schemas from one source (the size includes the generated control-v1 schemas) | task (migrated); app implements `Tasks` (REFACTOR) |
| skills | K3 | 8 | 2,192 | Share `_shape()` with workflows; one omission vocabulary; per-bound refusal codes; load the schema once | schemas/actions (migrated) |
| bash | outside the crate layout (tooling; ULTRAMAP §2) *(rev 2026-10-01, V1)* | 4 | 3,314 | Read bounds from the schema consts; **refuse over 60 s instead of clamping** (hee3:275); move the heredoc Python to .py | schemas/actions (migrated); the habitat-engine binary (app) |
| deploy | outside the crate layout (tooling; ULTRAMAP §2) *(rev 2026-10-01, V1)* | 4 | 4,706 | Real Quadlet or `.stub` for worker.container; measure features/rustflags/cleanliness from cargo/git; route every subprocess through bounded capture | the v3 build (cargo) |

**Not migrated, and why (Luke's instruction scoped the migration to HARDEN):**
- **REFACTOR (redesign in v4, not carried verbatim):** store, app, check, budget, worker, service, cohort, notify, workflows, julia, pi_extension.
- **KEEP:** recovery. It is sound, and task depends on it. Migrate it on Luke's word, since it is the cleanest reuse candidate.

**Tests:** each module's primary test files came with it. Cross-module suites (t04/t05/t06 store, t07 startup, t08 native/candidates, t28 socket/runtime/tasks/preview/coordinator/evidence, recovery.rs, accounting.rs, t13/t22/t11_notify) stay in v3 and are named here so nothing is silently dropped. Fixtures under `tests/fixtures/` were not copied; migrated tests that read them will need them when wired. `src/numerical/process.rs:417` (a unit test inside library source) `include_bytes!`s `tests/fixtures/t21/J01.json`, so it needs that fixture at compile time, not only at test time *(rev 2026-10-01, V1)*.

**Test dependencies not migrated** *(rev 2026-10-01, V1)* (the migrated tests import modules that were not copied):
- `t03_contract` → worker
- `t06_receipt_import` → app, check, store
- `t28_control` → store
- `t16_herdr` → recovery
- `t28_actions` → worker
- `t01_contracts.rs:13` `include_str!("../docs/contract-decisions.md")` → v3 `docs/contract-decisions.md`, not copied

**Provenance:** `git -C /var/home/herdr-engineering-engine-v3 archive b5367bc -- <paths>` (the path list is in this file's module table). Re-derive by re-running it, and compare against `MANIFEST.sha256`. From inside the toolbox the v3 root is an empty directory; use `git -C /run/host/var/home/herdr-engineering-engine-v3 archive b5367bc -- <paths>` *(rev 2026-10-01, V1)*.

*This file is the map, not migrated code: it is not listed in `MANIFEST.sha256` and was revised 2026-10-01 after verification V1 (`~/hee4-evidence/verification/V1-repo.md`).*

## 2026-10-05 · recovery staged
`src/recovery.rs` (1,305 lines, sha in `MANIFEST.sha256`) copied verbatim from the `b5367bc` tree. V4-59 ratified under H-27 (V4-86). The R01–R14 reconcile table in `gates/features/crash-restart.md` cites its line ranges. Staged, not wired.
