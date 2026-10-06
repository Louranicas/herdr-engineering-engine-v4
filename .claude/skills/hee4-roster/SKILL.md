---
name: hee4-roster
description: Route a HEE v4 task to its facet specialist(s) from .claude/agents/ROSTER.md, name the watchers that run alongside, and emit the eleven-field brief skeleton for hee4-coordinator. Use for /hee4-roster, "which agent owns this", or before any roster spawn. A task matching two facets is two briefs plus the coordinator, never one agent.
disable-model-invocation: true
---

# HEE v4 roster router

Routing is the first decision of a unit (narrative principle 7, `plan/STACK-MAP-2026-10-04.md` §7). Match the task against the trigger table, apply the two rules under it, name the watchers, and print the brief skeleton(s). Routing writes nothing; the coordinator does. All agents obey `.claude/agents/PROTOCOL.md`; the HOLD (V4-0) means builders write cards, feature files, maps and design notes, not engine code.

## Trigger table (one row per agent; match on any token)

| Agent | Triggers (crates, modules, paths, words) |
|---|---|
| `hee4-contracts-architect` | `hee4-contracts`, `modules/hee4-contracts/*`, `contracts`, `state-enums`, `bounds`, `judge-types`, `catalogue-data`, `TaskState`, "transition whitelist", "schema field", "brief schema", I1, "make it a type", "unrepresentable", "typestate" |
| `hee4-store-recovery` | `hee4-core`, `modules/hee4-core/store`, `modules/hee4-core/recovery`, `task`, `budget`, "ledger", "SQLite", "WAL", "fsync", "objects/sha256", R01–R14, "kill -9", "drill", "crash", "restart", "restore", `effect_unknown`, "non-terminal", `gates/features/crash-restart.md` |
| `hee4-isolation` | `modules/hee4-worker/namespace`, `modules/hee4-host/spawn`, `cgroup-io`, `modules/hee4-habitat/service`, "bwrap", "seccomp", "namespace", "sandbox", "mount", "capability", "permit", "actuation", "least privilege", "cgroup", "systemd unit", `Restart=`, `TimeoutStopSec` |
| `hee4-app-runtime` | `modules/hee4-app/{dispatcher,runtime,main,routing,startup-coordinator,plan,candidates,workload,repair,backup-target,tasks}`, "dispatch loop", RL-2, "startup order", "candidate plan", "repair", "backup target", "wire together", "K6 merge" |
| `hee4-verdict` | `hee4-evidence`, `modules/hee4-evidence/check`, `judge-admission`, `numerical`, `julia-decoders`, "decide", "verdict", "observation", I3, "severity lattice", "tier-0", "tier-1", "advisory", "mutation score", "sealed", `judge.inspect`, `analysis.*` |
| `hee4-control-socket` | `modules/hee4-app/control-socket`, `actions`, `tasks`, "socket", `control.sock`, "SO_PEERCRED", "frame", "request_id", "idempotency", "action_version", "error code", "refusal name", `events.subscribe` (emitter), "API Map", "Error and Refusal Map", any of the 22 `task.*`/`thread.*`/`tools.*`/`service.*`/`roster.*`/`health` action names |
| `hee4-worker-route` | `modules/hee4-worker/route`, `roster`, `native`, `inference`, `aggregate-policy`, `pi-adapter`, "route", "router", R02–R13, "capability floor", "model per role", "ollama", "retry", "backoff", `roster.list/inspect/update/disable` |
| `hee4-receipts-chain` | "receipt", `hash_prev`, `hash_self`, "checkpoint", "Merkle", "canonical JSON", "RFC 8785", "chain verify", I4 (chain discipline; the field types stay with contracts-architect) |
| `hee4-gate` | "gate", `gate.toml`, "tier", "commit/stack/cut", "doctor", "feature-map drive", `gates/features/README.md`, `modules/tooling/*`, `live-verifier-adapter`, "deep-diff-forge", "cargo-mutants", "plants", "proptest", "git archive", "elapsed/budget", "count literal" |
| `hee4-outer-loop` | `orders.json`, `control.ndjson`, "ack", "sequence number", "mode dial", "manual", "supervised", "auto", "admission door", "backlog adapter", I2, `modules/hee4-cohesion/workflows`, `cohort`, `notify`, `context`, `skills` (K3 cards) |
| `hee4-craft-curator` | `.claude/skills/pstack`, `SKILL.md`, "playbook", "PROVENANCE", `/correct`, `/reflect`, `/meditate`, "Diátaxis", "lesson", "recurring mistake", "rung 4", "router" (this file) |
| `hee4-floor-display` | `modules/hee4-habitat/herdr`, "floor", "herdr", "pane", "seat", "bay", "glass", "navigator", "Zellij", "LoomLattice projection", "alarm", "quiet-dark", "hooks over scraping", `events.subscribe` (consumer) |
| `hee4-watch-drift` | "apparatus_ratio", "drift", "generated block", "second home", "card over budget", "largest_file", D-01…D-16, AP-28, AP-44…AP-48 |
| `hee4-watch-contradiction` | "contradiction", "map vs card", "dangling cite", "stale pin", `cite_pins`, "two names", STACK-MAP §8 |
| `hee4-watch-evidence` | "unlabelled", "MEASURED", "witness command", "looked at nothing", `input_sha256`, `head_sha`, "tier-1 PASS" |
| `hee4-watch-recovery` | anything in the store-recovery row, plus `Restart=`, "non-terminal after drill", "ledgered before verdict" |
| `hee4-watch-fence` | "fence", "v3 path", `/var/home`, "bypassPermissions", "disableAllHooks", "secret", "Jev", H-10a, H-8 |
| `hee4-watch-budget` | "fan-out", `planned_agents`, "budget", "70%", "TIMEBOX", "resumed", "nested" |
| `hee4-coordinator` | every spawn; "run this unit", "brief", "ledger" |
| `hee4-refuter` | "verify the claim", "re-run", "refute", "is this report true", `record verify` |
| `hee4-scribe` | "brain", "reconcile", "decision row", "standing order", "unit record", `/meditate` output |

## Rules

1. **One facet per brief.** Match every row whose trigger appears in the task. One builder row → one brief. Two or more builder rows → one brief per row **plus** `hee4-coordinator` (ROSTER "No agent may hold two facets"). A cross-facet field (e.g. a receipt field: receipts-chain designs it, contracts-architect types it, store-recovery stores it) is routed to each owner with a DC-nn hand-off named in SCOPE.
2. **Unowned homes.** Since V4-84, K5 `service` → `hee4-isolation` and the K6 runtime cluster → `hee4-app-runtime`; `class-profile` and `u64-class` → `hee4-verdict` (the K6 adapter rule), `native-provider` → `hee4-worker-route`. Still unowned: `tooling/{bash,deploy,julia,pi-extension}` outside the gate. For those, route the nearest facet **read-only** and put the write in `Luke:` as an ownership ask; do not invent an owner.
3. **Watchers alongside, by default.** `hee4-watch-evidence` and `hee4-watch-fence` always. `hee4-watch-budget` whenever more than one agent is spawned. `hee4-watch-recovery` on anything matching the store-recovery row, `Restart=`, or the drill. `hee4-watch-drift` and `hee4-watch-contradiction` on any card, map or feature-file change. Watchers get their own brief; they never fix.
4. **Refuter after.** Any brief whose ACCEPTANCE contains a number or a "done" gets a `hee4-refuter` brief on its report; the refuter never checks its own or the coordinator's briefing.
5. **Scribe last.** Every unit ends with a `hee4-scribe` brief over all reports.
6. **Nothing to Jev, nothing outward** in any brief (H-10a); engine code in no brief while H-5 is open.

## Output: the brief skeleton (one per routed agent; RESTATEMENT and RECON stay empty for the receiver)

```
UNIT         <unit id>            AGENT  hee4-<name>          planned_agents=<N, set by the coordinator before any spawn>
GOAL         <one sentence; the deliverable and its reader>
SCOPE        <the facet's Writes paths only; DC-nn hand-offs to other facets named here>
CONTEXT      <files and ids to read first: the card(s), feature files, maps, `hee4db highway <module>`, STACK-MAP sections>
ACCEPTANCE   <numbered criteria, each checkable by a command; n of these is the `cases=k/n` denominator>
VERIFY       <the witness commands; `just verify` before and after; what each must print>
TIMEBOX      <minutes>            BUDGET <≤ N commands>
FORBIDDEN    <the agent's Refuses section, restated; the HOLD; the fence; nothing to Jev>
REPORT       <the one report path; last line `<name> verdict=... cases=k/n head=<sha12>`>
STANDING     <agents/standing-orders.md pasted verbatim, all eight lines; plus the hee4-brief LAW block>
RECON        <empty; the receiver fills with read-only findings before any mutation>
RESTATEMENT  <empty; the receiver's own-words GOAL, checked against ACCEPTANCE; a conflict is refused back>
```

Captain-only drafting aid (shadow; its text never reaches a subagent while the curator is at shadow): `workflow-curator brief --kind <kind> --goal "..."` ([handoff](/mnt/storage-10tb/workflow-curator/docs/WORKFLOW_CURATOR_20261005.md)).

## Worked routes

- "Add `hash_prev` to the receipt and store it" → `hee4-receipts-chain` (design note, DC-nn), `hee4-contracts-architect` (field type), `hee4-store-recovery` (ledger column) + `hee4-coordinator`; watchers evidence, fence, budget, recovery, contradiction; refuter; scribe. Three briefs, never one.
- "Fix the stale `check` card (STACK-MAP §8 #1)" → `hee4-verdict` alone; watchers evidence, fence, contradiction; refuter; scribe.
- "Which name does `tools.inspect` refuse with?" → `hee4-control-socket` (DC-nn row for §8 #4); watchers evidence, fence, contradiction.
- "Measure the corpus before the cut" → `hee4-watch-drift` + `hee4-watch-contradiction` only; no builder; scribe records.
- "Add `Restart=` to the unit" → `hee4-isolation` (K5 `service`, V4-84) with a DC-nn hand-off to `hee4-store-recovery` for the recovery consequence (D7/CN-06); watchers evidence, fence, recovery; refuter; scribe.
