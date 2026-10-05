# HEE v4 feature map

Behaviour-level inventory of HEE v4 as a person or an agent reaches it: the 22 release-scope control actions, the crash/restart/restore behaviour, and the end-to-end journeys that cross them. Status: planning text under the HOLD (V4-0); nothing here is built, so every "Driving it" block is the procedure the P-phase that lands the feature must make true, and every claim carries its tag.

## Purpose

This directory is the canonical behaviour-level substrate every change to HEE v4 is verified against. A module card (`modules/<crate>/<module>/MODULE.md`) says what a module owns and what proves it is done; a feature file says what a user of the socket sees and how to drive it. Cards cite feature files; feature files do not restate cards, and neither restates the system maps in the vault (`16 System Maps/`), which stay the one home of the design facts cited here.

Each file answers the same four questions, in the same four H2s, in this order:

1. `## Sub-features`: what exists, behaviour by behaviour.
2. `## How to get to it (user POV)`: how a caller reaches it, in which scope and phase, and what it looks like when it is not reachable.
3. `## Driving it with hee4`: the concrete invocations, the socket request and reply shape, and the side effects to read back.
4. `## Gotchas`: what usually lies: refusals, state legality, must-nots, and the places the maps leave open.

Where the maps and cards do not specify something a driver needs, the file says `UNWRITTEN: <what is needed>` instead of inventing it. An UNWRITTEN marker is a work item for the slice that lands the feature, not a known behaviour.

## The harness: `hee4` over the control socket

The harness is the product's own CLI client, not a test double (Command Map C-8; card `hee4-app/main` §2):

- `hee4 <action> …` connects to S-1 `$XDG_RUNTIME_DIR/hee4/control.sock`, sends **one** compact JSON frame terminated by one LF, and prints the reply. For `events.subscribe` it prints frames until the stream closes.
- The bash wrapper is `hee4-sh` (DC-19, V4-56), generated from the K0-emitted schema. It reaches S-1 only through the `hee4` binary. Its argument grammar is the v3 wrapper's (FACT `migrated/v3-b5367bc/integrations/bash/hee3`): `name=value` (literal body field), `name:=JSON` (JSON body field), `@idempotency_key=UUID`, `@precondition:=JSON`; the deadline comes from `HEE4_TIMEOUT_MS` and is **refused** above 60 s, never clamped. Wrapper-only verbs: `--check <action> …` (print the request, send nothing), `--actions`, `--inspect <action>`, `--version`, `--pin`, `chain <spec.json | ->`.
- Pi tools (`hee4_<action with . → _>`, PROPOSAL Command Map P-1) also go through the binary and are not a second harness.
- `UNWRITTEN: the hee4 binary's own body-argument grammar for C-8.` The v3 client in `main.rs` was not migrated; ATLAS fixes only the positional forms `hee4 health` and `hee4 task.get <id>` (Command Map §1 note "Argument form of C-8"). Until the P1 slice decides it, the concrete body spellings in these files are the `hee4-sh` ones.

### Wire shape (one home for all 22 files)

Every request is one line (FACT `control-v1.request.<id>.schema.json`; v4 renames the protocol, DEC V4-11):

```json
{"protocol":"hee4.control","version":1,"kind":"request","request_id":"<uuid4>","action":"<id>","action_version":1,"idempotency_key":"<uuid4 for mutating actions, else null>","deadline_unix_ms":"<now + at most 60000>","authority":{"grant_id":"<grant>","scope_sha256":"<hex>"},"precondition":null,"body":{}}
```

- `precondition` is `null` unless the action requires it; then it is `{"resource":"task|thread|roster|service|analysis","id":"<id>","generation":"<n>"}` (FACT `PreconditionV1`).
- Principal comes from SO_PEERCRED, never from the JSON; `grant_id` is principal-scoped, not a bearer token (UM §3a IN-4, IN-5).
- Frame ≤ 1,048,576 bytes payload, nesting ≤ 32; deadline ≤ 60,000 ms ahead of the receiver's clock.

A correlated reply is one of:

```json
{"protocol":"hee4.control","version":1,"kind":"result","request_id":"…","request_sha256":"…","replayed":false,"effect":"none|committed|pending","operation_id":null,"observed_generation":"…","readback":null,"body":{}}
{"protocol":"hee4.control","version":1,"kind":"error","request_id":"…","request_sha256":"…","code":"<ErrorCodeV1>","retry":"never|same_exact_request|after_readback|after_condition","effect":"none|unknown","message":"<static>","details":{"field":"<json pointer>","constraint":"…"},"readback":null}
```

An uncorrelated refusal (a `FrameFault`: oversize, garbled, EOF inside a record) has **no reply**: the connection closes (Error and Refusal Map §0). The 18 `ErrorCodeV1` values are the Error map's vocabulary; each feature file names only the codes its owner adds to the common set (`invalid_frame, unsupported_protocol, unsupported_version, unknown_action, unsupported_action_version, unauthenticated, forbidden, invalid_argument, deadline_exceeded, resource_exhausted, unavailable, internal`).

Per-action files print the `body` of the request and the `body` of the result only.

## `doctor`: the fresh-instance readiness check

There is no `hee4 doctor` verb (Command Map §1 lists C-1…C-8 and none is it). `doctor` here is a **procedure** the driver runs before any evidence is captured, assembled from the read-backs the maps already define. Run it first; a result against a stale binary, a foreign socket or an unreconciled ledger is not evidence.

| Check | Command (host, via `flatpak-spawn --host` from the toolbox) | Pass reads | Source |
|---|---|---|---|
| Unit active and fresh | `systemctl --user is-active hee4.service` → `active`; `systemctl --user show -p MainPID,ExecStart,TimeoutStopUSec hee4.service`; `sha256sum /proc/<MainPID>/exe` equals the manifest engine digest under `readlink -f ~/.local/lib/hee4/current`; `/proc/<MainPID>/cgroup` ends in `/hee4.service` | the running bytes are the installed release you mean to test | ATLAS D1, D2 |
| Socket perms and holder | `stat -c '%a %U' $XDG_RUNTIME_DIR/hee4` → `700 <you>`; `stat -c '%a %U' $XDG_RUNTIME_DIR/hee4/control.sock` → `600 <you>`; the socket inode appears in `/proc/<MainPID>/fd` (or `ss -xlp`) | one listener, owned by the unit, no v3 leftover under `habitat-engine/` | ATLAS D3; Socket map S-1, S-2 |
| Ledger opens | `hee4 health` → `ready=true recovery=complete database=ready socket=owned`; host read-only `sqlite3 'file:<root>/generations/<g>/ledger.sqlite3?mode=ro' 'PRAGMA user_version'` = schema `CURRENT`; `stat` the root 0700 and the ledger 0600 | the engine serves from a commissioned, reconciled generation | ATLAS D3, D4; API Map A-01 |
| Model reachable | two rows of `tools/doctor`: `model_endpoint` is read from `/api/ps` reachability (`curl --noproxy '*' http://127.0.0.1:11434/api/ps` parses; unreachable is FAIL) and `model_gpu` only from `size_vram > 0` for the pinned model (`--model`, else `HEE4_MODEL`, else the unit's `Environment`; a listed CPU copy is FAIL; not resident is UNMEASURED `scope=advisory`, counted only under `--require-gpu`) | the live GPU model, not a CPU fallback | ATLAS D6, §6 "Model daemon absent"; Socket map S-3 |

`health` itself carries no model field (API Map A-01 reply: `protocol_version, engine_version, ready, recovery, database, socket, checked_unix_ms`), so the fourth row is a separate read, not a parse of the health line. In the gate world (disposable HOME under `~/.cache/hee4-host/<run>/`, never `/tmp`, ATLAS §3.2) the first row is replaced by the pid of `hee4 serve --until-stdin-closes` and the fourth row is whatever the gate's model precondition says; say which world the evidence came from.

## Shared preconditions

- A commissioned state root: `hee4 commission <seconds>` ran once on this root (exit 0) and refuses a second time by name (ATLAS D4). `serve` refuses an uncommissioned root by name (DC-03).
- A running `serve` that owns custody: S-2 `control.lock` held, S-1 bound 0700/0600, `health` → `recovery=complete` (E2E-05). A client can never connect before recovery is complete: the socket is bound only after it (K6 PR-6).
- A grant record under `~/.config/hee4/grants/` for the connecting uid that covers the action's effect (PT-06 `Grants::resolve`); without it every action is `forbidden` at `/authority/grant_id` (Error map class 2). `UNWRITTEN: the grant record file format and the wrapper's env names for grant_id/scope_sha256 (v3 used HEE3_GRANT_ID / HEE3_SCOPE_SHA256; the V4-11 rename gives HEE4_TIMEOUT_MS, so HEE4_GRANT_ID / HEE4_SCOPE_SHA256 is INFERRED, not written).`
- A fresh `request_id` per request; a fresh `idempotency_key` per *intent* for mutating actions, reused only to replay the same bytes.
- Rate and cap: 100 requests/s, burst 32, at most 8 connections per principal, read from K0 bounds (RL-8). A sweep that fans out must stay under them or it measures the refusal, not the feature.
- Scope at the v4.0 tag: only A-01…A-10 are served (`health`, `tools.*`, `task.*`, `events.subscribe`). `roster.*` (v4.1), `service.*`, `thread.*`, `analysis.*` (v4.2) and `judge.inspect` (held, H-8) have no registry entry and are refused `unavailable` by name with a `because` naming the scope (Error map class 10). Their files describe the refusal as the reachable path and the served path as the P9 target.
- The HOLD: no code exists. Every block below is the contract the landing slice must meet, and every claim is labelled.

## The four-questions rule

A feature file is complete only when all four H2s are present, in order, and each answers its question for that feature alone. A change that cannot be placed under one of the four questions is either a new feature file or a card concern, not a fifth section. The validation script in this directory's parent (`gates/`) is the check: exactly `## Sub-features`, `## How to get to it (user POV)`, `## Driving it with hee4`, `## Gotchas`, nothing else at H2.

## Evidence rules

Adapted from the pstack verification-skill proof bar.

- **Production user path only.** Drive the engine through the `hee4` binary over S-1 (or `hee4-sh`/Pi, which go through it). A test that calls `serve_composed`, the store or `transition` in-process is a unit test, not feature evidence. Doubles count only behind the same production boundary an unavailable external dependency would use (the K0h spawn door for a missing model); they do not count when they skip the socket read, admission, the store or the verdict path.
- **`doctor` first.** A result against a stale binary is not evidence. Quote the `sha256sum /proc/<MainPID>/exe` and the `health` line with the result.
- **Cover the paths the change can affect:** success, cancel, error, empty, persistence. For a mutating action "persistence" means a lost-reply replay (same bytes → `replayed=true`) and the readback selector the catalogue names; for the store it means a restart (`crash-restart.md`).
- **Verify side effects, not just output.** Read the ledger row (host `mode=ro` query), the operations row, the outbox `delivered` count, the socket mode, the unit's `MainPID` — whatever the file's "Driving it" names. Output alone is a claim.
- **Name an unreachable path.** If a path cannot be driven, say which one and what blocks it: scope (v4.1/v4.2/held), a Luke grant (H-n), the HOLD, a host precondition (HP-n), or a missing driver. Then cover the closest real path that remains.
- **Label every claim** `MEASURED` (a command ran and its line is quoted), `INFERRED` (derived from a measured line or a cited design row), or `UNMEASURED` (not known; name the owner). A row that looked at nothing is UNMEASURED and counts as red (ATLAS §1, F138).
- **Carry the tree.** Every quoted line carries `tree=<sha> dirty=N`, as the gate lines do (ATLAS §4.2).

## Full sweep (top to bottom)

Walk this order for a broad regression; finish with `multi-surface-journeys.md` and `crash-restart.md`. Each entry is one action file.

### Readiness and catalogue (v4.0, P1)

- [health](health.md): the readiness line; the only action that reads custody, recovery and ledger state at once.
- [tools.list](tools.list.md): paged catalogue listing with its revision.
- [tools.inspect](tools.inspect.md): one action's schema digests, bounds and readback action.

### Task lifecycle (v4.0, P2–P5)

- [task.preview](task.preview.md): read-only planning: eligibility and exclusions for a spec.
- [task.submit](task.submit.md): durable admission; the one way a task is created.
- [task.get](task.get.md): one task with its attempts, evidence and delivery count.
- [task.list](task.list.md): keyset-paged task heads by state.
- [task.cancel](task.cancel.md): cancel intent with a generation precondition.
- [task.resolve](task.resolve.md): operator disposition of an obligation (quarantine, abandon).

### Streams (v4.0, P5)

- [events.subscribe](events.subscribe.md): the only `Reply::Stream` action; outbox delivery and acknowledgement.

### Roster (v4.1, P9)

- [roster.list](roster.list.md), [roster.inspect](roster.inspect.md), [roster.update](roster.update.md), [roster.disable](roster.disable.md).

### Services, threads, analysis (v4.2, P9)

- [service.inspect](service.inspect.md), [service.probe](service.probe.md), [service.action](service.action.md).
- [thread.get](thread.get.md), [thread.list](thread.list.md).
- [analysis.request](analysis.request.md), [analysis.get](analysis.get.md).

### Held

- [judge.inspect](judge.inspect.md): catalogued, never registered until H-8.

### Cross-cutting

- [crash-restart](crash-restart.md): kill -9 mid-task, unit restart, restore from backup.
- [multi-surface-journeys](multi-surface-journeys.md): E2E-01…12 as journeys across the files above, each with the done-line row it evidences.

## The change rule

A change to behaviour updates its feature file in the same change. "Behaviour" means anything a caller of S-1 or an operator at the CLI can observe: a body field, a refusal code or its `field`/`constraint`, a legal state pair, a bound, a readback selector, a unit or socket property. A card edit alone does not satisfy this rule; a slice whose diff touches a module's observable surface and not its feature file is incomplete. Resolving an `UNWRITTEN` marker is such a change: replace the marker with the measured behaviour and its source in the same commit.

## Entry contract

Every feature file uses the same four H2s:

1. `Sub-features`
2. `How to get to it (user POV)`
3. `Driving it with hee4`
4. `Gotchas`

Sources: `16 System Maps/API Map.md`, `Command Map.md`, `Socket and IPC Map.md`, `Error and Refusal Map.md`, `State and Transition Map.md`, `End-to-End Flow Traces.md`, `Workflow and Loop Map.md` (RL-5, RL-7, RL-10); `~/hee4-evidence/design/DEPLOYMENT_ATLAS.md` §1 (D1–D10), §4, §6; cards `hee4-app/{actions,control-socket,main}`, `hee4-evidence/check`, `hee4-core/recovery`; migrated v3 schemas `migrated/v3-b5367bc/schemas/actions/` and wrapper `integrations/bash/hee3` (FACT only; read-only).

## The drive: `tools/drive`

`tools/drive [--doctor-first] [--allow-restart] [--socket PATH]` runs the "Driving it with hee4" procedure of each served feature (`health`, `task.submit`, `task.get`, `task.list`, `task.cancel`) over the JSON-line frame, against the live unit by default or a `hee4 serve` you started on `--socket`. It covers success, replay, conflict, error, empty and cancel paths, checks the checkable Gotchas (one refusal name with its FLOW.md retry class and field; replay versus conflict; socket 0700/0600), and writes every frame sent and received to `~/.cache/hee4-drive/<sha12>/<feature>.jsonl`. It prints `drive feature=<name> verdict=PASS|FAIL|UNMEASURED paths=k/n evidence=<path>` per feature and `drive verdict=… features=k/n unserved=u head=<sha12>` last; exit 0 PASS, 1 FAIL, 3 UNMEASURED. `--doctor-first` runs `tools/doctor` and refuses (its unit or socket line, exit 3) when either is not MEASURED. The restart persistence path runs only with `--allow-restart` against the live unit, otherwise it is UNMEASURED with that reason. A feature whose Driving block lacks the `(rev 2026-10-05 drive)` marker is UNMEASURED ("procedure UNWRITTEN"); features with no action in the deployed catalogue print UNMEASURED `scope=unserved` and are counted in `unserved=`. It is the `drive` step of the `cut` tier.
