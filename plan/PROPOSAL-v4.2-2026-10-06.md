# HEE v4, v4.2 proposal: cohort (`thread.get`, `thread.list`) and numerical (`analysis.request`, `analysis.get`)

> Committed 2026-10-06 as a proposal (ledger R-004), after U-harden-05 wave 12. Its baseline is main `873e79c`; waves 11 and 12 since then changed the settle path, backups, the K0 VERIFY detector and the gate, not the cohort or numerical surface it designs. Luke decides the open decisions in §9.

**Status: PROPOSAL. Luke decides; this recommends.** Read-only on the repo: no edits, commits or cargo builds. One sandbox probe ran in the session scratchpad and was removed.

**Baseline.** The brief said main is at `661ed28`. Main is now at **`873e79c`**: two commits since then, touching only `tools/drill`, `tools/drive-cut` and their tests (MEASURED: `git log 661ed28..HEAD --stat`). The working tree is clean (MEASURED: `git status --short` is empty).

**Labels.** MEASURED means I read the file:line or ran the command named. INFERRED means reasoning from measured facts. I name the source of every claim.

---

## 0 · The recommendation in brief

1. **Cohort: serve a thread view over the attempts ledger (option C1).** A thread is one attempt, keyed by attempt identity as the card asks. v4.2 adds no new tables, because nothing in the 22-id catalogue writes a thread. The T-12 tables (`cohort_threads`, `cohort_reports`, `dissent_log`) wait until something writes them.
2. **Numerical: one ledger table, `analyses` (migration `m007_analyses`), with one writer (`store/numerical.rs`).** The life cycle is asynchronous: `queued → running → validated | failed | unknown`. `cancelled` is dropped because nothing writes it. The dataset is built from the engine's own append-only events at the cutoff, not uploaded.
3. **The julia child runs only through the K0h spawn door**, which needs four changes first:
   - exact environment (`--clearenv` plus an allowlist);
   - output capture bounded during the read;
   - `/proc` and `/dev` mounts on request;
   - a cgroup memory bound with read-back.

   Network stays none: `--unshare-net` is always in the argv.
4. **Grants (PT-06): gate by effect (option G2).** Grant files are JSON under `~/.config/hee4/grants/`. They are written by a local `hee4 grant` verb, never over the socket. At v4.2 the only gated effect is `BoundedAnalysis`, and every deployed action keeps today's admission.
5. **The production gate (O-15/H-14) is a separate runtime profile file that Luke pins.** Without it, `analysis.request` answers `unavailable`, with a `because` distinct from the scope's.
6. **Six slices.** Three can start now in parallel (cohort reads, the spawn door, the K4 pure core). The live julia slice waits on Luke: julia is not installed on this host (MEASURED).

---

## 1 · What each family is for (quoting the plan)

### Cohort (`Owner::Cohort`, K3 `hee4-cohesion`)
- Card `modules/hee4-cohesion/cohort/MODULE.md:17`: "Thread/cohort policy: assignment, conflicts, reports, rebrief, and **the one Join/Blocked vocabulary** (shared with workflows)". MEASURED.
- Same card, `:18`: "Owned state: none; durable rows `cohort_threads`, `cohort_reports`, `dissent_log` (append-only) owned by K1 via app". `:28`: v3 was "in-memory refusals only; no allocation, lifecycle, persistence, paging or caller … thread identity ≠ store attempt identity". `:31`: "Re-key threads onto attempt identity". `:67`: done criterion #1, "Threads keyed by attempt identity; restore from store". MEASURED.
- Vault `15 Module Design/K3 hee4-cohesion.md:301`: "Threads are keyed by **attempt identity**". The design bounds are 64 threads and 8 rebriefs (`:291-296`). MEASURED.
- Catalogue `crates/hee4-contracts/src/catalogue.rs:491`: "One cohort thread: its task, brief revision, state, obligations, children and artifacts, read against an expected brief revision." `:511`: "Paged thread heads under a task, filtered by state." MEASURED.
- ULTRAMAP `hee4-evidence/design/ULTRAMAP.md:162`: "thread.get, thread.list | refused (C06) | v4.2 | module-audit-b5367bc.md:18 cohort NOT FINISHED; needs K1 durable owner (migration)". MEASURED.
- STACK-MAP principle 8 (`plan/STACK-MAP-2026-10-04.md:116`): "Parallel workers, one aggregate". This is the use a cohort would serve (INFERRED). STACK-MAP I2 (`:62`) and INTEGRATION-MAP `:36` defer the outer loop (orders file with parallel `group`) to P9, K3. MEASURED.

### Numerical (`Owner::Numerical`, K4 `hee4-evidence`)
- Card `modules/hee4-evidence/numerical/MODULE.md:17`: "Offline numerical analysis of a dataset (descriptive recipe) checked against an independent Rust reference; caller of the julia `bin/analysis.jl` process". MEASURED.
- Catalogue `catalogue.rs:523`: "Request a bounded numerical analysis of a dataset by a pinned recipe and runtime: a julia child (S-7) under the K0h spawn door with wall, memory and output limits, recorded as an operation and an analysis row with its own state." MEASURED.
- CD RC03, `hee4-evidence/reference/v3-records/contract-decisions-b5367bc.md`:
  - `:69`: "Analysis | RC03 4,096 rows, 1-MiB request, 64-KiB result, 60 seconds and one child at a time".
  - `:493-508`: the dataset is attempt outcomes (`accepted|failed|cancelled|abandoned|running`) at an immutable cutoff.
  - `:513`: an error "grants no policy/task mutation".

  MEASURED.
- v3 `migrated/v3-b5367bc/src/numerical.rs:521-522`: a report is "Advisory only; accepting one grants no task or policy change". MEASURED. So **an analysis never changes task state**. It is a bounded, verified, advisory computation over the ledger's attempt outcomes. The use the plan names is the router baseline-versus-challenger report (CD `:79`); that link is INFERRED.

### What the two plan maps say
`plan/STACK-MAP-2026-10-04.md` and `plan/INTEGRATION-MAP-2026-10-04.md` define **neither** a thread nor an analysis. MEASURED: I read both in full. STACK-MAP mentions `analysis.request` only as the feature file with the most UNWRITTEN markers (`:98`). The definitions come from the cards, the catalogue and CD RC03.

---

## 2 · Ground truth the design must fit (all MEASURED)

| # | Fact | Where |
|---|---|---|
| F1 | `composed()` registers health, task, events, tools, roster and service. Cohort and Numerical are absent, so their ids are refused `unavailable` at `/action` with "scope v4.2: this action's family is not composed in this release". | `crates/hee4-app/src/actions/mod.rs:36-44`, `:238-245`; `catalogue.rs:152` |
| F2 | The envelope's `authority` member is "accepted and ignored". No `Grants` type exists anywhere in `crates/`. `~/.config/hee4/` does not exist. | `crates/hee4-app/src/wire.rs:3-4`; `crates/hee4-app/FLOW.md:39-40`; `roster.rs:10-11` ("no grants exist (owner: the grants slice)"); `ls ~/.config/hee4` |
| F3 | `forbidden` has retry class `never`, and its doc names only the peer uid. | `wire.rs:42-43`, `:150-158` |
| F4 | `Fault.because` is `Option<&'static str>`, so every `because` is a compile-time constant. | `wire.rs` (`struct Fault`) |
| F5 | Spawn door: the argv always carries `--unshare-all --unshare-net --die-with-parent --new-session`. No `--clearenv`. stdin is null. stdout and stderr are read with unbounded `read_to_end`. The only bound is the wall-timeout kill. No memory bound, rlimit or cgroup. No `--proc` or `--dev` mount. | `crates/hee4-host/src/spawn.rs:301-327`, `:370-376`, `:437`, `:495-499`; `FLOW.md:203-205` |
| F6 | **Probe:** the plan's own flags with a planted `HEE4_PROBE_SECRET=leaked`. The child printed it, plus `HOME`, `XDG_RUNTIME_DIR` and `DBUS_SESSION_BUS_ADDRESS`: 220 variables in all, inherited from my shell. Under the unit the inherited set is the user manager's environment plus `hee4.service:9-11` (INFERRED). | `bwrap 0.12.0`, command in §10 |
| F7 | `service.probe` checks the 4,096-byte busctl bound only **after** the unbounded read. | `crates/hee4-app/src/service_runner.rs` (`fn read`, `out.stdout.len() > budget.stdout_max`) |
| F8 | julia is not installed: not on `PATH`, no `~/.julia`, no mise entry. The repo has no `julia/` directory. | `command -v julia`; `ls`; `mise ls` |
| F9 | The user manager delegates the `cpu memory pids` controllers. `hee4.service` sets no `MemoryMax` and uses `KillMode=control-group`. | `user@1000.service/cgroup.subtree_control`; `systemd/hee4.service:19`; only `hee4-backup.service:18` and `hee4-daily.service:16` set `MemoryMax` |
| F10 | Migrations are keyed by **name**, not number: `m001_v1` … `m004_serve_cgroup`, four separate `m005_*` entries, and `m006_service_claims`. Append only. | `crates/hee4-core/src/store/migrations.rs:1-5`, `:113-135`; `crates/hee4-core/FLOW.md:49-66` |
| F11 | `attempts`: id `a-<task>-<generation>`, `state ∈ {running, settled, unknown}`, `effect`, `cleanup`, `outcome`, `receipt_id`, pid and start ticks. No token-usage column. Pre-ledger attempts have no row and are never backfilled. | `crates/hee4-core/src/store/attempts.rs:12-14`, `:19`, `:75-108` |
| F12 | No content-addressed object store, no `artifacts` table and no `EvidenceRef` type in v4. Backup "objects" are only the brief files. | grep over `crates/`; `hee4-core/FLOW.md:91-93` |
| F13 | Budgets are JSON (`serde_json`). The workspace has no TOML dependency. | `crates/hee4-contracts/src/budgets.rs:765-766`; `Cargo.toml:34-42` |
| F14 | `socket.client_read_ms` is 30,000 and `read_deadline_ms` is 60,000. `MAX_DEADLINE_MS` is 60,000. | `budgets.rs:68-73`; `crates/hee4-contracts/src/bounds.rs:18` |
| F15 | `NUMERICAL_CLEANUP_RESERVE` is **not in code**, although V4-63 records "Applied: K0 bounds". | grep over `crates/`; `plan/DECISIONS.md:237-239` |
| F16 | Once an action is served, it needs a `tools/drive.d` procedure and the `(rev 2026-10-05 drive)` marker. Without them the drive is UNMEASURED and the cut fails. Today the four are driven by their refusal: PASS 2/2, `scope=v4.2 served=false`. | `gates/features/README.md:148`; `DECISIONS.md:372` (V4-105) |
| F17 | Keyset paging with boot- and filter-pinned cursors already exists, and `roster.list` uses it. | `crates/hee4-app/src/actions/page.rs:1-80`; `roster.rs:182-230` |
| F18 | `events.ts` is in milliseconds (`now_ms`). Events are append-only (triggers). | `crates/hee4-core/src/store/mod.rs:299-303`, `:401`; `migrations.rs:31-34` |
| F19 | `hee4-evidence` depends only on `hee4-contracts` and `hee4-host`, so D-09 holds. **But** its ddf adapter spawns with `std::process::Command`, not the K0h door. | `crates/hee4-evidence/Cargo.toml`; `crates/hee4-evidence/src/ddf.rs:7`, `:290` |
| F20 | v3's julia call: the env allowlist, a 60 s window with a 10 s cleanup reserve, pinned project files, and alias files refused before and after the run. | `migrated/v3-b5367bc/src/numerical/process.rs:23`, `:99-145`, `:203-243`, `:259-300` |

---

## 3 · Wire shapes, and contradictions with file:line

### 3.1 The catalogue entries (MEASURED, `catalogue.rs`)
| Id | Line | Effect | Readback | Request fields | Result fields |
|---|---|---|---|---|---|
| `thread.get` | 489 | Read | — | `thread_id, expected_brief_revision` (:497) | `thread_id, task_id, brief_revision, state, obligations, children, artifacts` (:498-506) |
| `thread.list` | 509 | Read | — | `task_id, states, page` (:517) | `page` (:518) |
| `analysis.request` | 521 | BoundedAnalysis (mutates) | `analysis.get` (:527) | `subject, dataset, cutoff_unix_ms, recipe_id, recipe_version, runtime_id, limits` (:529-537) | `analysis_id, task_id, attempt_id, generation, dataset_sha256, state` (:538-545) |
| `analysis.get` | 548 | Read | — | `selector` (:556) | `analysis_id, state, dataset_sha256, report, error_code` (:557-563) |

All four have `PreconditionRule::None` and scope `V42`.

### 3.2 The authority the feature files missed
CD RC03 §4 (`contract-decisions-b5367bc.md:345-355`) gives the full shapes that the feature files mark UNWRITTEN:
- the thread state enum `planned|assigned|running|joining|blocked|settled` (6 states, matching A-19's `states ≤ 6`);
- the thread head `{thread_id, task_id, brief_revision, state, unresolved_obligations}`;
- `AnalysisSelectorV1 = {analysis_id} | {source_action:"analysis.request", idempotency_key}`;
- the full `analysis.get` result.

MEASURED.

### 3.3 Contradictions (each with a proposed resolution)
| # | Contradiction | Sources | Proposed resolution |
|---|---|---|---|
| K1 | `analysis.get` result: the catalogue has 5 fields. The feature file says "…" and "the full field list is not in any authority". CD lists 8, adding `task_id, attempt_id, generation`. | `catalogue.rs:557-563`; `gates/features/analysis.get.md:7,26,37`; CD `:355` | Add the three fields (catalogue revision moves, OD-14); fix the feature file in the same change |
| K2 | Thread state enum: UNWRITTEN in the feature files, defined in CD. | `thread.get.md:26`, `thread.list.md:26`; CD `:345-346` | Use CD's spellings; v4.2 carries only the states something writes (§4) |
| K3 | Thread head fields: UNWRITTEN in the feature file, defined in CD. | `thread.list.md:26`; CD `:346` | Adopt CD's head |
| K4 | `expected_brief_revision`: the feature file says "FACT required"; CD says `U64Decimal \| null`. | `thread.get.md:8,26`; CD `:345` | Member required; `null` = unguarded read; a number is checked (OD-3) |
| K5 | `thread.list` `task_id`: the feature file says "required all three"; CD allows `null` (all tasks). | `thread.list.md:7,26`; CD `:346` | Required, non-null at v4.2 (OD-3) |
| K6 | Analysis states: the feature file has `{running, validated, failed, cancelled, unknown}`. CD adds `queued`. API Map P-4 says "drop `queued` unless something writes it". Nothing writes `cancelled`. | `analysis.request.md:8,36`; CD `:354`; vault `16 System Maps/API Map.md:138` | Async design: `queued` **is** written, so keep it; drop `cancelled` (OD-4) |
| K7 | `error_code` domain: the feature file says the closed decoder set; CD says `ErrorCodeV1` (the control-wire codes). | `analysis.get.md:29,38`; CD `:355` | A closed `AnalysisError` set owned by K4, distinct from the wire `Code` (OD-6) |
| K8 | `report`: the feature file says the decoded output inline; CD says `EvidenceRefV1 \| null`. v4 has no object store (F12). | `catalogue.rs:561`; `analysis.get.md:7,26`; CD `:355` | Inline decoded report, at most `output_bytes` (OD-6) |
| K9 | `dataset`: CD says `EvidenceRefV1`, the feature file "a dataset object", and v4 has no object store. The digest is "of the input **at** `cutoff_unix_ms`". | CD `:354`; `analysis.request.md:10,21`; F12 | Build the dataset from the ledger at the cutoff (OD-5) |
| K10 | Ids: CD uses UuidV4 everywhere. v4 uses `t-`+24 hex for tasks and `a-<task>-<gen>` for attempts. | CD `:345-355`; `hee4-app/FLOW.md:102`; `attempts.rs:19` | v4 grammars; `analysis_id = n-`+24 hex (OD-13) |
| K11 | Sync or async: UNWRITTEN in the request file, yet the get file says "a caller polls it … until `validated` or `failed`". The CLI waits only 30 s, against v3's 60 s window. | `analysis.request.md:33`; `analysis.get.md:14`; F14, F20 | Async (OD-4) |
| K12 | Grants: the README says "without it every action is `forbidden`" and the API Map says `authority` is required. The code ignores `authority` and has no grants. | `gates/features/README.md:67`; `API Map.md:21`; F2 | Effect-gated grants (G2, §6) |
| K13 | `forbidden` retry class: the Error map (class 2) says `after_condition`; the code says `never`. | vault `16 System Maps/Error and Refusal Map.md:63`; `wire.rs:150-158` | `after_condition` (OD-10) |
| K14 | The `JULIA_*` allowlist is required, but the door passes the whole parent environment. | `analysis.request.md:7`; numerical card `:74`; F5, F6 | Exact environment in the door (S2, OD-11) |
| K15 | S-11 says "K0h spawn (bounded capture)"; the code reads to end. | vault `16 System Maps/Socket and IPC Map.md:31`; F5, F7 | Bounded capture in the door (S2) |
| K16 | Stale text, all MEASURED:<br>• `hee4-app/FLOW.md:4` says "eight ids served by three families", but roster and service are now composed.<br>• `FLOW.md:121` gives the because text "v4.2 the cohort/numerical families", while the code's text names no family (`catalogue.rs:152`; V4-105 at `DECISIONS.md:372`).<br>• The cohort, numerical, julia-decoders and tooling/julia cards still say "PLANNING — HOLD … No code" at line 2. | as listed | Fix with each slice (the same-change rule) |
| K17 | `NUMERICAL_CLEANUP_RESERVE` is "Applied: K0 bounds" in the record but absent from code. | `DECISIONS.md:237-239`; F15 | Add it as a K0 constant in S2 |
| K18 | Dissent: UNWRITTEN in the feature file. A-18 reads `dissent_log`; the catalogue has no dissent field. | `thread.get.md:9`; `API Map.md:94`; `catalogue.rs:498-506` | No dissent field until something writes dissent |
| K19 | T-12 names three new tables, but nothing catalogued writes them. | `thread.get.md:3`; vault `16 System Maps/State and Transition Map.md:34`; catalogue | C1 creates none; this is a deliberate deviation for Luke (OD-1) |
| K20 | "The only spawn is the K0h door" (UM-P13), but the K4 ddf adapter spawns directly. | `analysis.request.md:44`; F19 | Add a census in S2: `std::process::Command` only in `hee4-host`, with ddf migrated or named as the one exception (OD-12) |

---

## 4 · Cohort design

### 4.1 Options
- **C1, thread view over attempts (recommended).** A thread is an attempt (`thread_id` = attempt id, architecture-review-b5367bc.md:12). `thread.list` lists a task's attempts and `thread.get` reads one. Everything is computed from `attempts`, `tasks` and `receipts` at read time. No migration, no new writer, no second home for attempt state (AP-01).
- **C2, full T-12.** `m007_cohort` adds `cohort_threads`, append-only `cohort_reports` and append-only `dissent_log`, plus a K3 crate with the assign (≤ 64), rebrief (≤ 8), conflicts and join policy. It also needs a **writer that the 22-id catalogue does not have** (a new action or a submit-side field; the catalogue count is fixed by DC-09, `DECISIONS.md:167`) and concurrent attempts per task (today `UNIQUE (task_id, generation)` and a one-task dispatcher; `attempts.rs:96`, `FLOW.md:134-138`). This is the P9 outer loop (V4-79).
- **C3, park.** Keep the scope refusal, which already passes the cut (F16).

**Why C1.** INFERRED. It serves both reads with real ledger data and keys threads on attempt identity, as the card's done criterion #1 asks. It survives a restart for free, because the rows are K1's. It adds no table without a writer (AP-13). C2 stays the named next step, triggered by a catalogued cohort writer.

### 4.2 C1 data model
- **Tables:** none new. Reads: `attempts` (`state`, `effect`, `cleanup`, `receipt_id`, `generation`), `tasks` (existence), `receipts` (`id`, `hash_self`).
- **Store verbs** (in `store/attempts.rs`, the home of `attempts` SQL; F10 and the one-door rule): `attempt_view(id) -> Option<AttemptView>` and `attempts_of(task, states, after_generation, limit) -> Vec<AttemptView>`. Both are reads with no writer.
- **One writer:** unchanged. The attempts hook inside `apply_in` (`attempts.rs:36-44`) is the only writer of the facts a thread shows.
- **K0 types** (wire-visible, so K0 per DC-14): `ThreadState` and `Obligation`, each with one spelling per variant (EX-05).

| Attempt `state` | `ThreadState` |
|---|---|
| `running` | `running` |
| `settled` | `settled` |
| `unknown` | `blocked` (waiting on an operator obligation) |

| Attempt column | `Obligation` token |
|---|---|
| `effect = pending` / `unknown` | `effect_pending` / `effect_unknown` |
| `cleanup = pending` / `unknown` | `cleanup_pending` / `cleanup_unknown` |

`planned`, `assigned` and `joining` enter the enum only together with their writer (the P-4 rule, applied to threads). INFERRED mapping; OD-2.
- `brief_revision` is the constant `1`. The brief is written once at admission and never revised (`FLOW.md:103`), and no rebrief exists. `children` is always `[]`: v4 tasks have no parent link. `artifacts` is `[{kind:"receipt", receipt_id, hash_self}]` when a receipts row carries the attempt's `receipt_id`; that the two ids are the same is INFERRED from `FLOW.md:143`, `:153`.

### 4.3 Wire (v4.2)
- `thread.get` request: `{"thread_id":"a-t-<24hex>-<gen>","expected_brief_revision":1|null}`.
  Result: `{"thread_id","task_id","brief_revision":1,"state":"running|settled|blocked","obligations":[token…],"children":[],"artifacts":[{"kind":"receipt","receipt_id","hash_self"}]}`.
- `thread.list` request: `{"task_id":"t-…","states":[…≤6, v4.2 spellings only; [] = all],"page":{"limit":1..100,"cursor":null|{after_key,boot,filter_sha256}}}`.
  Result: `{"page":{"items":[{"thread_id","task_id","brief_revision","state","unresolved_obligations"}],"cursor":…}}`.
  The keyset is the zero-padded generation, so lexical order matches numeric order. The cursor is pinned to the boot and to the digest of `{task_id, states}`, reusing `page.rs` (F17).

### 4.4 C2 sketch (recorded, not proposed for v4.2)
`m007_cohort`:
- `cohort_threads(thread_id PK REFERENCES attempts(id), task_id FK, brief_revision ≥ 1, state CHECK(6), parent_thread_id NULL FK, updated_ts)`;
- `cohort_reports(seq PK, thread_id FK, brief_revision, report_json, ts)` with no-update and no-delete triggers;
- `dissent_log(seq PK, thread_id FK, brief_revision, dissent TEXT CHECK(length 1..512), ts)`, also append-only.

The one writer would be `store/cohort.rs` verbs called by K6 from pure K3 policy.

---

## 5 · Numerical design

### 5.1 Life cycle: asynchronous (OD-4)
1. `analysis.request` admits a `queued` row and its `operations` row in one `Store::operate` transaction, and replies at once.
2. A single runner thread (concurrency 1, as CD `:69` requires) takes the oldest `queued` row and records `running` with the child's pid and start ticks (`spawn::start` returns both; `spawn.rs:471`). It runs julia, then settles the row to `validated` or `failed`.
3. `analysis.get` polls.

**Why async.** INFERRED from F14, F20 and `analysis.get.md:14`. The CLI waits only 30 s. v3's window was 60 s with a 10 s reserve, and the get file describes polling. The synchronous `service.probe` pattern would need `wall_ms ≤ ~20 s` and would hold a connection thread. That is viable only if a measured julia cold start fits, and that is UNMEASURED because julia is absent.

### 5.2 Data model: `m007_analyses` in `crates/hee4-core/src/store/numerical.rs`, one line in `MIGRATIONS`
```sql
CREATE TABLE analyses(
  analysis_id TEXT PRIMARY KEY NOT NULL,            -- 'n-' + 24 hex of sha256(operation_id)
  operation_id TEXT NOT NULL UNIQUE,
  grant_id TEXT NOT NULL, grant_scope_sha256 TEXT NOT NULL CHECK (length(grant_scope_sha256)=64),
  subject_task_id TEXT NOT NULL REFERENCES tasks(id),
  subject_attempt_id TEXT NOT NULL REFERENCES attempts(id),
  subject_generation INTEGER NOT NULL CHECK (subject_generation >= 1),
  window_start_unix_ms INTEGER NOT NULL, cutoff_unix_ms INTEGER NOT NULL CHECK (cutoff_unix_ms >= window_start_unix_ms),
  dataset_sha256 TEXT NOT NULL CHECK (length(dataset_sha256)=64),
  dataset_rows INTEGER NOT NULL CHECK (dataset_rows BETWEEN 1 AND 4096),
  recipe_id TEXT NOT NULL CHECK (recipe_id = 'descriptive'),
  recipe_version INTEGER NOT NULL CHECK (recipe_version = 1),
  runtime_id TEXT NOT NULL, runtime_sha256 TEXT NOT NULL CHECK (length(runtime_sha256)=64),
  wall_ms INTEGER NOT NULL CHECK (wall_ms >= 1),
  memory_bytes INTEGER NOT NULL CHECK (memory_bytes >= 1),
  output_bytes INTEGER NOT NULL CHECK (output_bytes BETWEEN 1 AND 65536),
  state TEXT NOT NULL CHECK (state IN ('queued','running','validated','failed','unknown')),
  pid INTEGER, pid_start_ticks INTEGER,
  admitted_ms INTEGER NOT NULL, started_ms INTEGER, settled_ms INTEGER, elapsed_ms INTEGER,
  report_json TEXT CHECK (report_json IS NULL OR length(CAST(report_json AS BLOB)) <= 65536),
  report_sha256 TEXT CHECK (report_sha256 IS NULL OR length(report_sha256)=64),
  error_code TEXT CHECK (error_code IS NULL OR error_code IN (<the closed AnalysisError set>)),
  CHECK ((state='validated') = (report_json IS NOT NULL)),
  CHECK ((state='failed') = (error_code IS NOT NULL))
) STRICT;
-- analyses_no_delete; analyses_identity_immutable (UPDATE OF every column but state/pid/ticks/started/settled/elapsed/report/error);
-- analyses_no_reopen: OLD.state IN ('validated','failed','unknown') → ABORT; OLD.state='running' AND NEW.state='queued' → ABORT
```
**One writer.** Every SQL statement for `analyses` lives in `store/numerical.rs`, which the existing one-door census covers (`hee4-core/FLOW.md:9-22`). The verbs:

| Verb | Writes | Refuses |
|---|---|---|
| `analysis_admit(key, bytes, NewAnalysis)` (one `operate` closure) | `queued` row + `operations` row | key conflict; replay writes nothing |
| `analysis_started(id, pid, ticks)` | `queued → running` | any other state |
| `analysis_settle(id, Validated{report}\|Failed{code})` | `running →` terminal | any other state |
| `analysis_reconcile_start()` | `running → unknown`, all rows | — |

The readers are `analysis_get(id)`, `analysis_by_key(key)` (through `operation_by_key`, subject = `analysis_id`), `analysis_queued_count()` and `attempt_facts(window_start, cutoff, limit)` over `events` and `attempts`.

**Restart rules.** Named A1 and A2 so they do not collide with R01-R14. A1: `queued` stays `queued`; the runner takes it. A2: `running` becomes `unknown`, with one line `analysis reconcile analysis=<id> rule=A2 running -> unknown`; there is no re-attach, the same stance as R06. A request for `unknown` replays the stored row; the caller asks again under a new key.

### 5.3 Dataset: ledger-derived at the cutoff (OD-5)
`dataset: {"source":"attempts","window_start_unix_ms":<u64>}` selects every attempt whose `Dispatch` event `ts` falls in `[window_start, cutoff]`, at most 4,096 rows. K6 reads the facts through K1. A **pure** K4 builder turns them into canonical bytes, the same value-passing pattern as `ClassFacts` (V4-61, `DECISIONS.md:226`). `dataset_sha256` is the digest of those bytes. Events are append-only (F18), so a rebuild at the same cutoff gives identical bytes (INFERRED; a test pins it).

The outcome mapping is an exact rule (INFERRED; OD-5). For each attempt, take its closing event at or before the cutoff:

| Closing event | Outcome |
|---|---|
| none | `running` (`censored = true`; elapsed = cutoff − dispatch) |
| `Settle(Ready)`, then `Accept` | `accepted` |
| `Settle(Ready)` with no Accept or Decide by the cutoff | `running` (censored) |
| `Settle(NotReady)`, or `Decide(Fail)`, or `Stop` with the task `failed` | `failed` |
| `Stop` with the task `cancelled` | `cancelled` |
| `Resolve(Abandon)`, `Settle(Unsettled)` or `Recover(R07\|R08)` | `abandoned` |

`usage_tokens` is always `null`, an honest "unknown": v4 records no usage (F11), and v3 keeps `null` distinct from `"0"` (`numerical.rs:470-473`). `elapsed_ms` is the closing event's `ts` minus the dispatch event's `ts`.

### 5.4 Wire (v4.2; integers and v4 ids, OD-13)
- `analysis.request` request: `{"subject":{"task_id","attempt_id","generation"},"dataset":{"source":"attempts","window_start_unix_ms"},"cutoff_unix_ms","recipe_id":"descriptive","recipe_version":1,"runtime_id":"<token>","limits":{"wall_ms","memory_bytes","output_bytes"}}`, plus `idempotency_key` and `authority{grant_id, scope_sha256}`.
  Result: `{"analysis_id","task_id","attempt_id","generation","dataset_sha256","state":"queued"}`.
- `analysis.get` request: `{"selector":{"analysis_id"}}` or `{"selector":{"source_action":"analysis.request","idempotency_key"}}`, exactly one form (CD `:355`, `:452`).
  Result: `{"analysis_id","task_id","attempt_id","generation","state","dataset_sha256","report":null|{descriptive/1 report},"error_code":null|code}`.

The closed `AnalysisError` set:
- the julia codes `bound domain duplicate encoding identity schema stale` (`numerical.rs:382-401`);
- the engine codes `wall_exceeded output_over_bound memory_exceeded nonzero_exit diagnostic unsettled decode_refused binding_mismatch statistics_mismatch limits_unverified`.

### 5.5 The julia child under the K0h spawn door (S-7)
**Who calls what.** K4 `hee4-evidence::numerical::run(profile, dataset_file, limits) -> Exchange` is the caller (card `:73`; ULTRAMAP.md:174). It builds the plan and goes through `spawn::plan` and `spawn::start`, nothing else. The K6 runner thread commits through K1. This is the `service_runner` shape: the runner returns values and the actions commit (`service_runner.rs:1-4`).

**Plan, as a sketch.** INFERRED; it needs the S2 door additions:
```
systemd-run --user --scope --quiet --collect --unit=hee4-analysis-<id> \
  -p MemoryMax=<memory_bytes> -p MemorySwapMax=0 -p TasksMax=<numerical.tasks_max> -p CPUQuota=100% -- \
bwrap --unshare-all --unshare-net --die-with-parent --new-session --clearenv \
  --setenv LC_ALL C --setenv LANG C --setenv JULIA_LOAD_PATH @:@stdlib --setenv JULIA_PKG_OFFLINE true \
  --setenv JULIA_PKG_PRECOMPILE_AUTO 0 --setenv JULIA_NUM_THREADS 1 --setenv OPENBLAS_NUM_THREADS 1 \
  --setenv JULIA_DEPOT_PATH <scratch>:<depot> --setenv HOME <scratch> --setenv TMPDIR <scratch> \
  --ro-bind /usr /usr --ro-bind /lib /lib --ro-bind /lib64 /lib64 --ro-bind /bin /bin \
  --ro-bind <project> <project> --ro-bind <depot> <depot> --ro-bind <dataset.json> <dataset.json> \
  --proc /proc --dev /dev --bind <scratch> <scratch> --chdir <scratch> -- \
  <julia> --startup-file=no --project=<project> --check-bounds=yes --depwarn=error --threads=1 \
  --compiled-modules=no <project>/bin/analysis.jl <dataset.json>
```
Sources: the environment list and flags are v3's (`process.rs:203-243`). `--proc` and `--dev` are INFERRED needs: julia locates its install through `/proc/self/exe`, and `FLOW.md:203` records that the sandbox has no `/dev`. Both are measured once julia exists. The dataset goes in a **read-only bound file**, not stdin: stdin is null (F5), and a read-only bind stops the child changing its own input. `--die-with-parent` should kill the sandbox if `serve` dies, even though the scope sits outside `hee4.service`'s cgroup. That is INFERRED from the bwrap manual and must be tested by a kill -9 e2e.

**Limits as budgets.** A new `numerical` section in `Budgets`, validated by field name like the rest (`budgets.rs`):

| Limit | Request bound | Home | Default | Ceiling | Source |
|---|---|---|---|---|---|
| `wall_ms` | 1..=`numerical.wall_ms_max` | budget | 50,000 | 50,000 (= 60 s − 10 s reserve) | CD `:69`; `process.rs:23`, `:186-196`; DC-41 |
| cleanup reserve | — | K0 const `NUMERICAL_CLEANUP_RESERVE` | 10,000 ms | — | `DECISIONS.md:237` |
| `memory_bytes` | 64 MiB..=`numerical.memory_bytes_max` | budget | 2 GiB (INFERRED; measure) | 8 GiB | CD `:64` per-candidate `MemoryMax` |
| `output_bytes` | 1..=65,536 | K0 const | — | 65,536 | CD `:69`, `:354`; `numerical.rs:338` |
| dataset rows / bytes | 1..=4,096 / ≤ 1 MiB | K0 consts | — | — | CD `:69`; `numerical.rs:335`, `:340` |
| queued rows | ≤ `numerical.queue_max` | budget | 4 | 16 | CD `:69` (one child at a time) |
| tasks (pids) | — | budget `numerical.tasks_max` | 64 (INFERRED) | 256 | `DECISIONS.md:238` |
| runner idle poll | — | budget `numerical.idle_ms` | as `dispatcher.idle_ms` | — | `FLOW.md:30` |

**Network: none.** `--unshare-net` is unconditional in the door (`spawn.rs:301-307`). The plan binds no sockets and no model door. A pure plan test asserts the argv.

**What is verified, before a row becomes `validated`:**
1. **Runtime pins**, before **and after** the run, as v3 did (`process.rs:100-130`, `:259-300`): the julia executable's sha256, every project file's sha256, the depot digest and the DC-44 schema digest. A julia "alias" file (`JuliaProject.toml`, `LocalPreferences.toml`, …) is refused.
2. **Limits applied.** The child's `memory.max`, `memory.swap.max` and `pids.max` are read back from its cgroup while it runs (AP-49: "a StartTransientUnit success is not a limit"). If the read-back fails, the child is killed and the row becomes `failed(limits_unverified)`. The wall limit is the door's kill. Output is bounded during the read: at most `output_bytes + 1` bytes, then kill, with both numbers recorded.
3. **Custody settled.** The leader is reaped and the scope is empty (v3's `Unsettled`, `process.rs:150-162`). An OOM kill (`memory.events oom_kill ≥ 1`) becomes `memory_exceeded`.
4. **Output.** Exactly one JSON object with no unknown members and no duplicate keys, bound to the request digest, subject, cutoff and recipe (`numerical.rs:732-760`). A julia refusal counts only as exit 2 with empty stderr and a code from the closed set (`process.rs:164-172`).
5. **Numbers.** An independent Rust reference recomputes the counts exactly. `acceptance_fraction` must be within `1e-12` and `mean_observed_ms` within `8·ε·max(|mean|,1)` (CD `:517-522`; `numerical.rs:752-754`). AP-21 applies: the reference shares no code with the decode.
6. **Dataset.** The file's sha256 is checked before the run and after it.
7. **Budget printed.** `elapsed/budget` and `margin=` appear for every run (AP-31).

### 5.6 The production gate (O-15/H-14) is a runtime profile, not a PT-06 grant
`~/.config/hee4/runtimes/<runtime_id>.json` (0600; JSON, F13) holds:
- `{version, runtime_id, executable, executable_sha256, julia_version, project, project_files{path: sha}, depot, depot_sha256, schema_sha256, granted{decision:"H-14", by, at_unix_ms}}`.

Luke writes it with a local verb, `hee4 runtime pin …`, which measures every digest. The v4 file set drops `Cohesion.jl`, per ULTRAMAP.md:262. The profile is checked at serve start, with one line `numerical runtime=<id> pin=PASS|FAIL reason=`, and again before and after each run. When it is absent or fails, `analysis.request` answers `unavailable`, `field /body/runtime_id`, with one of these `because` constants, each from one site:
- `"numerical runtime not granted (O-15/H-14)"`;
- `"julia digest"`;
- `"julia depot not pinned"`.

None equals `Scope::V42.because()`, as Error map class 10 requires: "distinct `because` strings, one site each" (`Error and Refusal Map.md:71`).

---

## 6 · Grants (PT-06)

**The contract.** API Map `:117`: `Grants::resolve(principal, grant_id, scope_sha256, now) -> Option<Caller>`, "K6 config loader over `~/.config/hee4/grants/`". The v3 trait is at `migrated/v3-b5367bc/src/actions/control.rs:40-60`. v3's `Caller` is `{visible: Vec<Owner>, granted: Vec<Effect>}` (`migrated/v3-b5367bc/src/actions.rs:905-945`). v3 refused in two places: `forbidden` at `/authority/grant_id` (no grant) and at `/action` (the effect is not covered) (`control.rs:382-408`; Error map F-12, `:89`). CD `:416`: "`grant_id` is a server-side grant reference scoped to that principal … It is not a bearer grant. `scope_sha256` binds the reviewed scope record". All MEASURED.

### 6.1 File format (shared by both options)
`~/.config/hee4/grants/<grant_id>.json`:
```json
{"version":1,"grant_id":"g-bounded-analysis-01","principal":"uid:1000","effects":["bounded_analysis"],
 "issued_unix_ms":1791234567000,"expires_unix_ms":1793826567000,"issued_by":"luke","reason":"H-14: descriptive analyses"}
```
Rules. A file that breaks any rule **does not resolve**, and `serve` logs `grant refused grant_id=<id> reason=<name>`:
- the directory is 0700 and the file 0600, a regular file (not a symlink), owned by the engine's uid, at most 4,096 bytes;
- exactly these members, with duplicate keys refused;
- `grant_id` equals the file stem and matches `g-[a-z0-9-]{1,61}`, so no path is built from client text;
- `principal` uses the engine's spelling, `uid:<n>` (`actions/mod.rs:93-94`);
- `effects` is a non-empty list of `Effect` wire names with no duplicates;
- `issued < expires`.

`scope_sha256` = SHA-256 of K0 `canonical_json` of the whole object: the exact record the operator reviewed. The directory comes from a `--grants DIR` serve flag, else `HEE4_GRANTS`, else the default, matching the budgets precedent (`FLOW.md:12`; `hee4.service:12` already uses `%h/.config/hee4/`).

### 6.2 Option G1: every action needs a grant (PT-06 as first written)
`authority` becomes required on every request. Any action without a matching grant is `forbidden`.
- **Pro:** one door for every effect, exactly as RC03 says.
- **Con:** the blast radius takes in every deployed client (the CLI, `tools/drive`, `drill`, `check-deployed`, Firstmate) and the cut that passes today (V4-105). Deploy would have to write a bootstrap operator grant, and a missing file would turn every deployed action into `forbidden`.

### 6.3 Option G2: grants gated by effect (recommended)
- K0 gains `Effect::requires_grant(self) -> bool`, a `const fn` with an exhaustive `match` and **no wildcard**. A new effect does not compile until someone decides its gate (rung 1). At v4.2 it is true for `BoundedAnalysis` only.
- Dispatch gains one step after the registry check and before the idempotency key, `not_ready` and precondition checks. An unserved action therefore still answers `unavailable`, and authority is checked before the body (CD §5: "peer, grant, visibility … before materializing large action bodies").
- `wire::parse` parses `authority` as optional. A malformed `authority` is `invalid_argument` at `/authority`.
- Every other effect keeps today's admission (`SO_PEERCRED` uid, the 0700/0600 socket). Gates widen one effect at a time, each a reviewed one-line K0 change with its own test.
- **No visibility filtering in v4.2.** There is one principal; the trigger to revisit is "a second principal uid admitted", the same trigger V4-65 uses (`DECISIONS.md:249-251`).
- **Unrelated to H-8.** `Judge` stays unregistrable by construction (`registry.rs`), and the judge grant record lives apart in `~/.config/hee4/judge/` (UM §4d).

### 6.4 Who writes it
- **W1 (recommended): local `hee4 grant issue|revoke|list|scope` verbs.** No socket and no engine. They refuse to run as another uid and write a temp file, then rename it, with mode 0600. `issue --effect bounded_analysis --expires-in <dur> --reason <text>` prints `grant_id=` and `scope_sha256=`. The verbs are Luke's operator act.
- **W2:** Luke edits the file by hand, and `hee4 grant scope ID` prints the digest.

The socket never mints a grant, because a client must not authorise itself (CD `:416`). Grants are read **per request** by direct lookup of `<dir>/<grant_id>.json`, so a revoked grant takes effect on the next request, as in v3 (`actions.rs:998`). The analysis row records `grant_id` and `grant_scope_sha256` (§5.2), so each use is audited where it happens.

### 6.5 How a refusal names the missing grant
| Case | Code | Field | `because` (constant) | Message (static template) |
|---|---|---|---|---|
| no `authority` | `forbidden` | `/authority/grant_id` | `"no grant presented"` | "analysis.request (effect bounded_analysis) needs a grant: `hee4 grant issue --effect bounded_analysis`" |
| no file, or the file does not resolve | `forbidden` | `/authority/grant_id` | `"grant not found"` | names `~/.config/hee4/grants/<id>.json` |
| expired | `forbidden` | `/authority/grant_id` | `"grant expired"` | — |
| another principal | `forbidden` | `/authority/grant_id` | `"grant principal"` | — |
| `scope_sha256` differs | `forbidden` | `/authority/grant_id` | `"grant scope moved"` | — |
| effect not covered | `forbidden` | `/action` | `"grant lacks effect"` | names the effect's wire name |

Retry class: `after_condition`, because a grant changes by an operator act (Error map `:63`; OD-10).

---

## 7 · Doors per action (refusal by name) and the tests that prove them

### `thread.get` / `thread.list`
| Refusal | Field | When |
|---|---|---|
| `invalid_argument` | `/body/thread_id`, `/body/expected_brief_revision`, `/body/task_id`, `/body/states`, `/body/page/limit`, `/body/page/cursor` | malformed id; revision not a u64 or `null`; `states` not an array, more than 6, an unknown spelling or duplicates; page outside 1..100 |
| `not_found` | `/body/thread_id`, `/body/task_id` | no attempts row (pre-ledger attempts included, F11); task never admitted (as `task.get`) |
| `stale_generation` | `/body/expected_brief_revision` | a number other than the brief revision; `current_generation` = 1 |
| `resync_required` | `/body/page/cursor` | `because` `"epoch moved"` or `"filter moved"` |

Tests:
- `thread_get_reads_an_attempt_as_a_thread_and_agrees_with_task_get`;
- `thread_state_maps_running_settled_unknown_to_running_settled_blocked` (driven through `Store::apply`; a mutant mapping `unknown→settled` must fail it);
- `obligations_name_cleanup_pending_after_settle_and_clear_after_cleanup_settled`;
- `stale_brief_revision_is_stale_generation_with_current_generation_1`;
- `null_revision_is_an_unguarded_read`;
- `unknown_and_pre_ledger_attempts_are_not_found_at_body_thread_id`;
- `list_pages_are_disjoint_and_complete_over_five_generations_at_limit_2`;
- `cursor_from_another_boot_is_resync_required_epoch_moved`;
- `cursor_under_other_states_is_resync_required_filter_moved`;
- `states_over_six_or_unknown_spelling_is_invalid_argument_at_body_states`;
- `task_with_no_attempts_is_an_empty_page`;
- `threads_read_the_same_after_store_reopen` (card done #1);
- `tools/drive.d/thread.py`, with paths success, stale, not_found, empty, continuation and resync.

### `analysis.request` (dispatch order: scope → grant → key → `not_ready` → body → runtime → admit)
| Refusal | Field | When |
|---|---|---|
| `unavailable` | `/action` | scope (until composed) |
| `forbidden` ×2 | `/authority/grant_id`, `/action` | §6.5 |
| `invalid_argument` | `/idempotency_key`; `/body/limits/{wall_ms,memory_bytes,output_bytes}` (the message names the bound with both numbers); `/body/recipe_id`; `/body/cutoff_unix_ms`; `/body/dataset/*`; `/body/subject/*` | missing key; out of bounds; recipe not `descriptive`; cutoff in the future or before the window |
| `unsupported_action_version` | `/body/recipe_version` | not 1 (the precedent is `probe_version`, `service.rs` `probe_body`) |
| `not_ready` | `/action` | reconcile not complete |
| `not_found` | `/body/subject/task_id`, `/body/subject/attempt_id` | not in the ledger |
| `unavailable` | `/body/runtime_id` | §5.6, three `because` constants |
| `resource_exhausted` | `/` | queued rows at `queue_max`; dataset over 4,096 rows or 1 MiB (both numbers) |
| `conflict` | `/idempotency_key` | same key, other bytes |

Run-time failures are **not** wire refusals. They are `state=failed` with an `error_code`.

Tests:
- `request_without_authority_is_forbidden_at_authority_grant_id_because_no_grant_presented`;
- each limit at 0 and at bound+1, `invalid_argument` naming both numbers;
- `runtime_absent_is_unavailable_with_a_because_unlike_the_scope_because` (plus a census that each `because` has one emission site);
- `replay_returns_the_stored_row_replayed_true`;
- `same_key_other_bytes_is_conflict`;
- `queue_full_is_resource_exhausted_with_both_numbers`;
- store trigger tests: no delete, no reopen, identity immutable, and the validated↔report and failed↔error CHECK pair;
- `reconcile_moves_running_to_unknown_and_keeps_queued`;
- `dataset_bytes_are_identical_for_two_builds_at_one_cutoff`;
- a mapping-table test, one case per row of §5.3;
- K4 pure tests: a plan argv test (`--unshare-net`, `--clearenv`, the exact environment list, `--threads=1`, no socket bind); the classify table carried from v3 `process.rs:143-180`; a decoder planted with an unknown code is refused by name; J01–J05 (CD `:458-466`) from a **recording** (DEPLOYMENT_ATLAS.md:138); tolerance boundaries J05; a planted `use hee4_core` in K4 fails `cargo check` (card done #1);
- door tests (S2): `planted_env_var_does_not_reach_the_child`; `stdout_over_bound_kills_the_child_and_names_both_numbers`; existing e2e and live-model tests stay green;
- live, gated by `HEE4_LIVE_JULIA=1` (else an `UNMEASURED` line): J01 through the socket to `validated`; a memory hog becomes `failed(memory_exceeded)` with the `oom_kill` read back; kill -9 mid-run, then a restart, gives `unknown` and no surviving `hee4-analysis-*` scope;
- `tools/drive.d/analysis.py`, live: the `forbidden` and production-`unavailable` paths. On a disposable serve with a test grant: the success paths, or UNMEASURED by name with no julia.

### `analysis.get`
| Refusal | Field | When |
|---|---|---|
| `invalid_argument` | `/body/selector` | neither form, both forms, or a malformed id or key |
| `not_found` | `/body/selector` | no such analysis or key |

Tests:
- `get_by_id_equals_get_by_key`;
- `queued_and_running_have_null_report_and_null_error_code`;
- `failed_has_a_closed_error_code_and_null_report`.

---

## 8 · Slice plan (six, in order)
1. **S1 cohort-reads (C1).** Independent; start now.
2. **S2 K0h spawn-door hardening.** Independent; start now. It also fixes the live environment leak (F6) and the unbounded busctl read (F7).
3. **S3 grants (PT-06, G2).** Independent. Tested through `dispatch_with` with a stub Numerical family, so live behaviour does not change.
4. **S4 numerical ledger and actions.** Depends on S3. Once served, the family is driven by its production refusal.
5. **S5 K4 numerical pure core.** Independent. Recording the fixtures waits for julia.
6. **S6 numerical runner, live.** Depends on S2, S4 and S5, plus Luke's host and grant acts.

Every slice keeps `just cut-check` passing, and lands its feature-file edits, FLOW rows and drive procedure in the same change (`gates/features/README.md:148`).

## 9 · Open decisions
Eighteen decisions are Luke's. Each carries a recommended default. (Copied verbatim from the design run's `open_decisions`, `hee4-evidence/roster/U-harden-05/understand-results.json` key `design-v42`.)

1. OD-1 Cohort scope for v4.2. Options: C1, a thread view over the attempts ledger (thread = attempt, no new tables); C2, the full T-12 tables plus a K3 crate and a catalogued cohort writer (needs a catalogue change past DC-09's 22 ids, and concurrent attempts per task); C3, park (the scope refusal already passes the cut). RECOMMENDED DEFAULT: C1 now, with C2 triggered by a catalogued cohort writer or the P9 outer loop (V4-79).
2. OD-2 Thread state enum and mapping. v4.2 carries only states something writes: running, settled, blocked (attempt running/settled/unknown). Obligations are the closed tokens effect_pending, effect_unknown, cleanup_pending, cleanup_unknown. CD's planned, assigned and joining are added with their writer. RECOMMENDED DEFAULT: adopt as stated (the P-4 'drop unless written' rule, applied to threads).
3. OD-3 Thread wire nullability (K4, K5). RECOMMENDED DEFAULT: expected_brief_revision is a required member where null means an unguarded read (as CD :345) and a number is checked (stale_generation, current_generation=1); thread.list task_id is required and non-null at v4.2 (as the feature file), so pages stay per task.
4. OD-4 Numerical life cycle (K6, K11). Options: async (states queued|running|validated|failed|unknown, cancelled dropped since nothing writes it, a single runner thread, restart rules A1 queued stays, A2 running->unknown); or sync on the service.probe pattern (states validated|failed only; wall_ms must fit the CLI's 30 s client_read_ms). RECOMMENDED DEFAULT: async; the reply state of analysis.request is queued, which is a named deviation from analysis.request.md:8,33.
5. OD-5 Dataset source and outcome mapping (K9). Options: D1, built from the ledger, attempts dispatched in [window_start_unix_ms, cutoff], with the closing-event-to-outcome table of section 5.3 and usage_tokens always null; D2, inline dataset bytes in the request body (at most 1 MiB, inside the frame bound); D3, an EvidenceRefV1 into a new object store (no catalogued action uploads objects). RECOMMENDED DEFAULT: D1 with the section 5.3 table, pinned by one test per row.
6. OD-6 Analysis reply shape (K7, K8). RECOMMENDED DEFAULT: report is the decoded descriptive/1 report inline, at most output_bytes and stored in the row with its sha256 (v4 has no object store); error_code comes from a closed AnalysisError set owned by K4 (the seven julia codes plus wall_exceeded, output_over_bound, memory_exceeded, nonzero_exit, diagnostic, unsettled, decode_refused, binding_mismatch, statistics_mismatch, limits_unverified), never the control-wire Code.
7. OD-7 Grants model (PT-06). Options: G1, every action needs a grant (all clients and the passing cut change; deploy writes a bootstrap grant); G2, gated by effect through an exhaustive const fn Effect::requires_grant, true only for BoundedAnalysis at v4.2, checked right after the registry check. RECOMMENDED DEFAULT: G2, widening one effect at a time with its own test; no visibility filtering until a second principal uid exists.
8. OD-8 Grant file format and writer. RECOMMENDED DEFAULT: JSON, no new dependency (members version, grant_id, principal 'uid:<n>', effects, issued_unix_ms, expires_unix_ms, issued_by, reason); dir 0700, file 0600, regular file, owned by the engine uid, at most 4 KiB, unknown and duplicate members refused; scope_sha256 = SHA-256 of K0 canonical_json of the file. Written only by local 'hee4 grant issue|revoke|list|scope' verbs that Luke runs, never over the socket. Read per request by direct lookup. Directory from --grants, else HEE4_GRANTS, else ~/.config/hee4/grants. Analysis rows record the grant_id and scope used.
9. OD-9 Maximum grant lifetime. RECOMMENDED DEFAULT: at most 90 days from issue (INFERRED; any cap works if it is one constant in K0); renewal is a new issue, never an in-place edit.
10. OD-10 Retry class of forbidden (K13). The Error map class 2 says after_condition; the code says never. RECOMMENDED DEFAULT: after_condition for every forbidden (a grant, or the operator's configuration, can change), changing the one table in wire.rs and the FLOW row together.
11. OD-11 Spawn environment (K14, F6). Options: --clearenv plus an explicit environment list for every plan, task attempts included; or only for numerical plans. RECOMMENDED DEFAULT: every plan, in its own slice (S2). The task-attempt list is PATH, HOME=<work>, LANG and HEE4_MODEL_SOCKET when a door is bound. The bar is the full e2e suite, the live-model test and the cut drive.
12. OD-12 One spawn door everywhere (K20, F19). The K4 ddf adapter spawns with std::process::Command. RECOMMENDED DEFAULT: add a census in S2 that permits std::process::Command only in hee4-host; move ddf behind the door in a follow-up slice, and until then name it as the census's single exception with a reason.
13. OD-13 v4 wire forms (K10). RECOMMENDED DEFAULT: JSON integers for u64 fields (not CD's U64Decimal strings) and v4 id grammars (t-<24hex>, a-<task>-<gen>, n-<24hex> for analyses), matching the deployed families.
14. OD-14 Catalogue change (K1). RECOMMENDED DEFAULT: add task_id, attempt_id and generation to analysis.get result_fields in S4 (CD :355). This moves catalogue.rs revision(), so Poteto Weave's catalogue_contract_changed door needs requalifying by its owner.
15. OD-15 Memory bound for the julia child. Options: M1, a transient user scope via systemd-run with MemoryMax, MemorySwapMax=0, TasksMax and CPUQuota, read back from the child's cgroup while it runs (fail closed as limits_unverified); M2, declare the memory bound absent; M3, RLIMIT_AS (INFERRED to break julia's address-space reservations). RECOMMENDED DEFAULT: M1, with a kill -9 e2e proving that --die-with-parent removes the scope.
16. OD-16 Production gate and host acts (O-15/H-14). RECOMMENDED DEFAULT: the runtime profile ~/.config/hee4/runtimes/<id>.json, written by 'hee4 runtime pin' run by Luke, IS the production grant; when it is absent, analysis.request answers unavailable with because 'numerical runtime not granted (O-15/H-14)'. Installing julia (none on this host today) is Luke's host act; recommended: an official release tarball, pinned by digest in a read-only directory, depot offline and pinned, no Cohesion.jl.
17. OD-17 Fixture provenance (AP-21, DEPLOYMENT_ATLAS.md:138). The v3 J01 fixture was not migrated (migrated/v3-b5367bc/MIGRATION.md:25), and agents must not read HEE v3 paths. RECOMMENDED DEFAULT: re-record J01-J05 with the pinned v4 runtime, and accept them only when the independent Rust reference reproduces CD :458-466's stated values (total 5, accepted 1, unknown usage 3, known usage 4, fraction 0.2, mean 30 ms).
18. OD-18 Numerical budgets. RECOMMENDED DEFAULT: a new Budgets 'numerical' section: wall_ms_max 50,000 (ceiling 50,000 = 60 s minus NUMERICAL_CLEANUP_RESERVE 10 s), memory_bytes_max 2 GiB (ceiling 8 GiB, CD :64), queue_max 4 (ceiling 16), tasks_max 64 (ceiling 256), idle_ms as dispatcher.idle_ms. output_bytes 65,536, rows 4,096 and dataset 1 MiB stay K0 constants. Revisit memory and wall after the first live measurement of julia's cold start.

## 10 · Evidence index (commands run, all read-only)
- `git log --oneline -8`; `git log 661ed28..HEAD --stat`; `git status --short` (clean, 873e79c).
- `cat -n` of the four feature files, `catalogue.rs`, `hee4-app/FLOW.md`, `hee4-core/FLOW.md`, `migrations.rs`, both plan maps, and the module cards for cohort, numerical, julia-decoders and tooling/julia.
- `sed -n` over `DECISIONS.md:160-175,230-256,360-372`, `service.rs`, `service_runner.rs`, `spawn.rs`, `attempts.rs`, `roster.rs`, `page.rs`, `registry.rs`, `actions/mod.rs`, `wire.rs`, the v3 copies under `migrated/` (`process.rs`, `numerical.rs`, `actions.rs`, `actions/control.rs`), ULTRAMAP §4-5, the vault API, Socket, Error and State maps, K3 design §cohort, and CD RC03 `:320-355`, `:405-425`, `:487-560`.
- Greps for grants, `clearenv`, rlimit, cgroup, `NUMERICAL_CLEANUP_RESERVE`, the object store and `std::process`; `command -v julia`; `cat /proc/self/cgroup` and `user@1000.service/cgroup.subtree_control`; `man bwrap`; `man systemd-run`.
- The bwrap environment probe: `HEE4_PROBE_SECRET=leaked bwrap --unshare-all --unshare-net --die-with-parent --new-session --ro-bind /usr /usr --symlink usr/lib /lib --symlink usr/lib /lib64 --symlink usr/bin /bin --bind $W $W --chdir $W -- /usr/bin/env` (rc 0; the secret was printed). The scratch directory was removed afterwards.
- **Not read:** the Anti-Bloat Budget rows for cohort and numerical, the vault's K4 design section, and the E2E traces. Their size and trace numbers are not used here.
