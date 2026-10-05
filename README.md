# Herdr Engineering Engine v4

HEE v4 is a local engineering engine. It admits a task over a Unix control socket, dispatches it to a local model inside an isolated namespace, verifies it with one verdict authority, settles it durably with a hash-chained receipt, and survives a crash, a restart and a restore. It runs on this machine as a systemd user unit.

The repository holds the engine (`crates/`), its gate (`tools/`, `gate.toml`), the behaviour-level feature map it is verified against (`gates/features/`), the planning corpus it was built from (`plan/`, `modules/`, `docs/`), the agent roster that builds it (`.claude/agents/`, `.claude/skills/`), and the orchestration glue for Firstmate (`ops/firstmate/`).

Every claim here is labelled. MEASURED means a command ran on this machine and its output is quoted or cited. DESIGN means written and not yet built. HELD means it waits on the owner. Where a number would go stale, a command stands in its place: run it.

## Status (2026-10-05)

| What | State | Witness |
|---|---|---|
| Live unit | `hee4.service` active, `Restart=on-failure` | `systemctl --user is-active hee4.service` |
| Binary | `hee4 4.0.0-skeleton <sha12>`, the sha baked at build | `hee4 --version` |
| Health | `recovery_complete=true` after reconcile | `hee4 health` |
| Crash drill | SIGKILL → systemd restart → recovery complete, socket perms intact | `tools/drill --unit hee4.service --socket $XDG_RUNTIME_DIR/hee4/control.sock --repo .` |
| Feature drive | health, task.submit, task.get, task.list, task.cancel against the live unit, restart included | `tools/drive --doctor-first --allow-restart` |
| Gate | cut tier **7/7** at `6bd7579e20b7` (fmt, clippy, 129 tests, sealed diff, kill -9 drill, feature drive, doctor) with the live model on | `just gate commit`, `just gate cut` |
| Model | user-space ollama 0.35.1 on `127.0.0.1:11434`, `qwen2.5:0.5b`, GPU via Vulkan; the engine reaches it only through the door | `systemctl --user is-active ollama.service`, `hee4 doctor --repo .` |
| First live Pass | one task through the deployed unit: `admitted -> accepted` in 0.2 s, receipt `71a15c2b…`, `command` and `model-door` observations, one `model_request` row | `plan/DECISIONS.md` V4-94 |
| Tags | `skeleton-deployed-2026-10-05`, `hardened-deployed-2026-10-05`, `live-model-deployed-2026-10-05` | `git tag -n` |

The engine has run a real model attempt: a `sh:` VERIFY step inside the sandbox curled the model door, the door forwarded one request to ollama on loopback, and `decide` sealed `Pass` over two observations bound to the brief's VERIFY digest. Before the model existed, the same path ended in a refusal, not a Pass; that refusal is the design, and the Pass is the same code with a model behind the door. This is still not the ATLAS D10 version cut: that tag has its own field list (`tools/check-deployed` over D1–D9, a cold clone, `push-scan`, `apparatus_ratio=`) and none of those tools exist yet.

## The one rule

The owner's meta goal: an environment that makes it hard to write bad code. Every door in the engine has a rung, and the highest rung wins:

1. **Impossible.** Types and architecture. A wrong state has no representation.
2. **Refused at the door.** Admission checks that cost seconds.
3. **Caught by a check.** Build, tests, plants, the sealed diff observation, the feature drive.
4. **Caught by review.** Advisory only; never a verdict.
5. **Caught in production.** The drill, receipts, recovery.

A recurring mistake class moves up a rung, and the new door must be proven to fail on the real past instance before it is admitted (`plan/STACK-MAP-2026-10-04.md` §0; the `correct` skill).

## Architecture

Six crates under `crates/`, one owner each. The arrows are the only ways data crosses a boundary.

```
task.submit {brief}        events.subscribe
      │                          ▲
      ▼                          │
 hee4-app ── control socket (0700 dir, 0600 socket, SO_PEERCRED) ── actions ── dispatcher
      │ Store::apply(Event) is the only state writer
      ▼
 hee4-core ── SQLite ledger (events are truth; tasks is a replay cache) ── recovery R01–R14 before any dispatch
      │
      ▼
 hee4-worker ── route (capability floor, typed refusal) ── namespace plan ── native attempt
      │                                                            │
      ▼                                                            ▼
 hee4-host ── spawn behind a permit (bwrap, --unshare-net) ── model door (unix socket → 127.0.0.1:11434)
      │
      ▼
 hee4-evidence ── decide: the one verdict (fail-closed lattice over sealed observations) ── receipt chain
      │
      ▼
 hee4-contracts ── TaskState via transition only · parsed newtypes · the eleven-field brief · Receipt::seal
```

| Crate | Owns | Door it holds | Proof |
|---|---|---|---|
| `hee4-contracts` | `TaskState`, `Event`, the `transition` whitelist, `Observation`, `Brief`, `Receipt` | rung 1: a `TaskState` cannot be built except by `transition` or `replay`; digests and ids parse or refuse | `cargo test -p hee4-contracts` prints `legal=65/65 illegal=383/383` and the tamper sweep |
| `hee4-core` | the SQLite ledger, idempotent admission, the receipt chain, recovery | rung 2: no SQL outside `store.rs` (a recursive token census); `Dispatch` refused until reconcile completes | `tests/crash.rs` SIGKILLs a child mid-write in two landing modes and reopens |
| `hee4-host` | `spawn` behind a `Permit`, `Clock`, the model door | rung 2: no permit, no spawn; no network inside the sandbox; the door refuses by name (400/408/411/413/429/503) | sandbox tests run real `bwrap` and prove the negative paths |
| `hee4-worker` | route rules R02–R13, namespace plans, attempts | rung 2: no model meets the floor → a typed refusal, never a default; skipped steps stay recorded | `cargo test -p hee4-worker` |
| `hee4-evidence` | `decide`, the deep-diff-forge adapter | rung 2: zero tier-0 observations → refused; an unbound observation → `Unreconciled`; a census allows sealing only in `decide.rs` | the exhaustive lattice test and `tests/one_sealer.rs` |
| `hee4-app` | the `hee4` binary: socket, actions, dispatcher, doctor, unit | rung 2: one name per refusal (a test ties every emitted refusal to `FLOW.md`), peer uid must match, connection and frame caps | `tests/e2e.rs` runs the server, submits, SIGKILLs it, restarts, checks recovery |

Each crate carries a one-page `FLOW.md` with its types, its doors, and what it may and may not construct. Read those before the code.

### The frame

One JSON object per line over the socket. Request: `{request_id, action, action_version: 1, idempotency_key, body}`. Reply: `{kind: "result", request_id, replayed, body}` or `{kind: "error", code, retry, field, message}`. Actions in the deployed catalogue: `health`, `task.submit`, `task.get`, `task.list`, `task.cancel`, `task.preview`, `task.resolve`, `events.subscribe`. The refusal names are listed once, in `crates/hee4-app/FLOW.md`, and a test fails if the code can emit one that is not there.

### The brief

`task.submit` takes a brief with eleven fields: `GOAL SCOPE CONTEXT ACCEPTANCE VERIFY TIMEBOX FORBIDDEN REPORT STANDING RECON RESTATEMENT`. A missing field is refused by name; an empty `RESTATEMENT` is refused. `VERIFY` is one step per line: an absolute path runs as a bare argv in the sandbox; `sh: <line>` runs `/bin/sh -c <line>` there, which is how a step reaches the model (`curl --unix-socket "$HEE4_MODEL_SOCKET" http://model/api/generate …`); `model: <prompt>` is a recorded skip; anything else is a named skip. Every `Run` step records a `command` observation (`exit`, `stdout`), Pass only on exit 0, and every request through the door adds a `model-door` observation with one `model_request` row. The same brief shape is what the agent roster uses to dispatch work to itself.

## Build, test, gate

Rust 1.99, offline builds (the registry cache is on this machine).

```bash
cargo build --workspace --release
cargo test --workspace
just gate commit        # fmt, clippy -D warnings, test, on a git archive of HEAD, never the live tree
just gate stack         # commit + a sealed deep-diff-forge observation (--require-files --require-hunks) + doc
just gate cut           # stack + drill + drive + doctor
```

`tools/gate` exports the subject at a sha, sets `HEE4_HEAD` for `build.rs`, builds in a per-subject target dir, and prints `step=<name> rc=<n> elapsed=<s>/<budget>s margin=<s>` per step and one verdict line. A step whose expected output is absent (zero tests collected) is marked `looked_at_nothing` and fails. Tiers and budgets live in `gate.toml` only.

## Deploy, drill, drive

```bash
cargo build -p hee4-app --release
install -Dm755 target/release/hee4 ~/.local/bin/hee4
install -Dm644 systemd/hee4.service ~/.config/systemd/user/hee4.service
systemctl --user daemon-reload && systemctl --user enable --now hee4.service
hee4 health
hee4 doctor --repo .                      # unit, socket perms, ledger, model endpoint, binary sha == HEAD
tools/drill --unit hee4.service --socket $XDG_RUNTIME_DIR/hee4/control.sock --repo .
tools/drive --doctor-first --allow-restart
```

The unit runs with `ProtectSystem=strict`, `NoNewPrivileges=yes`, `PrivateTmp=yes`, and writes only under `~/.local/share/hee4` and `$XDG_RUNTIME_DIR/hee4`. The drill sends SIGKILL to the main pid and requires the restart, the socket perms and `recovery=complete` within their budgets. The drive runs each feature file's "Driving it with hee4" procedure (success, cancel, error, empty and persistence paths) and keeps every frame as evidence under `~/.cache/hee4-drive/<sha12>/`.

## Verification, in order of trust

1. **Deterministic observations decide.** Build, tests, plants, the sealed diff observation and the feature drive feed `decide`. Model-based review is advisory and can only refuse.
2. **Every observation is bound to its subject.** `head_sha` and `input_sha256` are mandatory; a mismatch is `Unreconciled`.
3. **Nothing passes by looking at nothing.** Zero files, zero hunks, zero tests, zero tier-0 observations are refusals.
4. **Doctor first.** A result against a stale binary is not evidence; `binary_sha == HEAD` is a doctor row.
5. **Observations are ledgered before the verdict.** Recovery replays them.
6. **Refuters, not reviewers, close a slice.** Every load-bearing crate was handed to an independent agent whose brief was to refute the builder's claims and attack the code. Their findings became fix slices before the merge was trusted.

## Orchestration

Firstmate is the orchestrator (`~/firstmate`, backend herdr). Each Firstmate home holds one orchestration database, `data/firstmate.db`, written only by `ops/firstmate/fm-db` (units, briefs, spawns, claims, verifications, receipts, exits, andon) and read back with `tursodb --readonly`. Its doors refuse an unlabelled claim, a claim verified by its own author, a spawn past `planned_agents`, a spawn under an open andon, and a spawn without a recorded brief; `fm-db close-unit` closes a unit only when every spawn has an exit.

The roster (`.claude/agents/ROSTER.md`, bound by `PROTOCOL.md`) is twelve facet specialists, six read-only watchers and three collaboration roles. Builders work in git worktrees under `/mnt/storage-10tb/hee4-wt/`, one branch each, merged only after the first mate re-runs their gates. `treehouse` cuts crew worktrees from the local mirror `origin` (`/mnt/storage-10tb/hee4-origin.git`); GitHub is the remote `github`, pushed on the owner's word.

The craft layer is Lauren Tan's pstack and brainmaxxing, ported under `.claude/skills/` (MIT, provenance recorded). `brain/` is the build memory, injected at session start.

## Planning corpus

`START.md` is the one-hop entry for a fresh agent. `plan/DECISIONS.md` is the append-only register (V4-0 onward). `plan/STACK-MAP-2026-10-04.md` is the end-to-end map with the rung ladder; `plan/INTEGRATION-MAP-2026-10-04.md` assigns one owner per concern across the five repos the stack draws on. `modules/` holds one card per module and the manifest the checks read. `gates/features/` is the feature map. `ops/checks/` and `just verify` keep the corpus coherent with the code: cite pins, the module funnel, the ops database, the mermaid render.

```bash
bash .claude/hooks/context-doctor.sh   # can a fresh agent reach every home and tool? present|MISSING per row
just verify                            # every corpus check, one verdict line
hee4db highway <module>                # everything about one module in one call
```

## Doors that fired on the builders

Recorded because each is now an instance a future door must fail on:

- The gate refused a zero-test run on the empty workspace (AP-29).
- The dispatcher refused to run a binary whose baked head sha was `unknown`; the cause was the gate's `git archive` export sharing a target dir with the live tree.
- A census test bound `CARGO_MANIFEST_DIR` at compile time and broke in a reused export; paths are now read at runtime.
- A `CLOSED` marker written in the wrong table cell left two held items open in the ops database; `hee4db check=held` now refuses it.
- The gate resolved `[gate].base = HEAD~1` against the checkout's moving HEAD; a commit landing mid-run emptied the diff and `deep-diff-forge` refused `0 files`. The base is now resolved against the subject sha before any step runs.
- Under the gate's deeper target dir the model door's socket path reached 109 bytes and `bind(2)` refused it mid-attempt; the live tree, at 77 bytes, never saw it. The door now lives beside the control socket (50 bytes in production) and an over-long path is a typed `DoorPath` refusal at plan time.
- The first live run ended `effect_unknown`: the door observation's `input_sha256` digested the request digests, so `decide` could never reconcile it with the subject. Every observation now carries the VERIFY digest.

## Not yet

- The D10 version cut as the ATLAS defines it: `tools/check-deployed` (D1–D9 aggregate and its `--control`), the cold clone, `push-scan` and `apparatus_ratio=` are unbuilt, so no `v4.0.0` tag exists and none is claimed.
- A brief whose VERIFY is trivially true (`sh: true`) now reaches `accepted`, since a ran-and-exited-0 step is a Pass with exit evidence. That is a rung-2 gap at admission (refuse a VERIFY that looks at nothing), not at the verdict.
- Firstmate live crew in herdr: decided (V4-93) as captain-supervised on this harness; zero-touch work runs through the Agent-tool roster recorded in `firstmate.db`. Revisit when herdr gains a key-send or Firstmate delivers briefs to raw launches.
- Nineteen release actions beyond the skeleton (`thread.*`, `tools.*`, `service.*`, `roster.*`, `analysis.*`, `judge.inspect`): the drive reports each `UNMEASURED` by name.
- An attempts table so recovery rules R03, R09 and R13 can fire; socket and door limits as K1 budgets rather than literals.
- The Jev advisory port: the boundary door is installed and refuses every v4 name; no sender is installed.

## Layout

```
crates/           hee4-contracts hee4-core hee4-host hee4-worker hee4-evidence hee4-app (each with FLOW.md)
systemd/          hee4.service
tools/            gate · doctor · drill · drive · tests/
gate.toml         tiers and budgets
gates/features/   one file per release action, four questions each; crash-restart; journeys
plan/             DECISIONS.md (append-only) · STACK-MAP · INTEGRATION-MAP · FIRSTMATE-ORCHESTRATION
modules/          53 module cards + MODULES.toml
docs/             anti-patterns, exemplars, drift controls
ops/              checks (cite pins, funnel, trace, render) · db (hee4db) · firstmate (fm-db, schema)
.claude/          agents (the roster), skills (pstack + hee4-*), hooks (context-doctor, brain, v3 guard)
brain/            build memory, one topic per file
migrated/         v3 modules staged verbatim, not wired
```

## Licence and provenance

The engine is UNLICENSED (private). `.claude/skills/pstack/` is MIT (Lauren Tan); its `PROVENANCE.md` lists every porting change. `migrated/` is the v3 engine at `b5367bc` with a manifest.
