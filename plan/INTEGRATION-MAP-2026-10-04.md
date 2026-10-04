# Integration map: poteto's stack, LoomLattice, deep-diff-forge, noodle → HEE v4

Drafted 2026-10-04 from read-only study of the local clones (`~/noodle`, `~/brainmaxxing`, `~/how`, `~/verification-skill-example`, `~/cursor-plugins/{pstack,orchestrate,cursor-team-kit,continual-learning}`, `~/loom-lattice-habitat`, `~/deep-diff-forge`), the "Lauren-tan" transcript (`~/Downloads/lauren-tan-transcript.txt`) and the May-2025 `dev_ops_for_tool_master_project` doc (`~/Downloads/dev_ops_for_tool_master_project.txt`). Every claim below is labelled MEASURED (from files or a run), DESIGN (written but not wired) or VISION (transcript only). Nothing here is a decision; the proposed rows at the end are for Luke to accept or strike.

## 1 · The thesis, and what is actually real

The transcript's closing line: "one thesis expressed in nine bindings — isolation at the data plane, accountability at the governance layer, calibration at the decision layer, verification at the merge gate, memory at the vault, craft at the playbook." That is a sound layering. The problem is that the transcript assigns each layer to a repo as if the repo already did it. Measured:

| Layer (transcript) | Assigned to | Measured state |
|---|---|---|
| Substrate | Omarchy + Hyprland + herdr | Live on this machine |
| Outer loop ("dark factory") | noodle | Go, v0.1.5, 843 tests; **dormant since 2026-03-19**; agents run with `bypassPermissions` / `--dangerously-bypass-approvals-and-sandbox`; isolation is git-worktree only; no cron, "when" is an LLM each cycle |
| Craft / playbooks | pstack (Cursor plugin v0.15.9) | 23 playbooks, **24** principles (not 23), **50** skills (not 47), 2 sub-agents; markdown is portable to Claude Code; `mode`/`reminder`/`is_background`/`hooks.json followup_message` are Cursor-only |
| Memory / vault | brainmaxxing | Already Claude Code native (`SessionStart` inject, `PostToolUse` re-index, `/reflect` `/ruminate` `/meditate`) |
| Verification substrate | verification-skill-example | Feature map (4 H2s per feature) + Launch/Doctor/Drive/Evidence/Cleanup; "a video against a stale bundle is not evidence" |
| Review bay | deep-diff-forge | Rust, 954 tests, deterministic `rank.v0`/`cluster.v0` in ~2 ms; **Rust-only** tree-sitter on a side path; risk = fixed path-rule weights; learning crate wired to nothing; "L9" is a constant; no verdict, no input digest |
| Accountable floor | loom-lattice-habitat | 155k lines Rust, 8,100 tests, 52 gate receipts verified; **daemon does not persist across restarts**; nothing advances a loom past Seated outside a stub test; `SendPermit` correct but no live actuator; hook installer wires a CLI verb that does not exist; router has no transport; **no Turso/SQLite/Jev/Omarchy anywhere**; Zellij is the substrate, herdr is clean-room-banned |
| Inner engine | HEE v4 | Planning only; 0 lines of code; 53 cards; staged v3 code that does not compile as staged |
| Governance rules | dev_ops doc (May 2025) | A ChatGPT log: ~70% role-played "NAM/ANAM" pseudo-maths (fiction), ~30% durable rules (sandbox everything, 100% tests + zero HIGH/MED security to merge, log every action, rollback latch within an observation window, approve-before-run) |

Transcript claims **not found** in any repo: the herdr "under 50 columns reports idle" / "latches working" bug; Turso per-loom isolation; "Jev scores it"; the "trust ladder" phrase (closest: LL's gate-receipt chain, which *is* mechanical and re-verifiable from disk); "indirect prompt"; "README-driven development"; prototyping as "measure 100 times, cut once" (pstack's `prototype.md` says the opposite).

## 2 · One owner per concern

The corpus's own rule is "one door per rule, one topic one home" (REQUIREMENTS rank 9; AP-48). Applied across the five repos, today there are three receipt formats, two control sockets, two routers, two unit-of-work state machines and two verdict vocabularies. Proposed ownership:

| Concern | Owner | What the others contribute |
|---|---|---|
| Unit of work + state machine | **HEE v4 `TaskState` (11 states) + K1 `transition`** | LL `LoomState` (8) becomes a *projection* for display (seat/bay), never a second lifecycle |
| Verdict | **HEE `decide` (K4), the one authority** | deep-diff-forge `rank.v0` = observation; LL `Verdict` and gate verdicts not reused; pstack swarm "PASS/ISSUES/BLOCKED, a gap is never a pass" = admission rule for observations |
| Receipt | **HEE receipt v1 (migrated schema + tests)** | Add LL's `hash_prev`/`hash_self` + Merkle checkpoint discipline, which HEE's card set does not state; LL `lattice.receipt.v1`/`fable.receipt.v1` and DDF learning receipts retired |
| Control socket | **HEE `control-socket`** (0700/0600, SO_PEERCRED, `events.subscribe`) | LL `ll-daemon` socket is a near duplicate; not reused |
| Router | **HEE `route` (R02–R13)** | Fold in LL's capability-floor-with-typed-refusal as one rule; pstack poteto-mode is a *playbook* router, a different layer (see §3) |
| Isolation | **HEE K2 namespace (bwrap) + K0h spawn** | noodle has none; LL has permits without an actuator; dev_ops rules (tempdir/VM, approve-before-run) become K2 policy rows |
| Durability / recovery | **HEE `store` (SQLite ledger) + `recovery` R01–R14** | noodle's reconcile (snapshot + PID adoption + recover "merging") is the best worked example of what R01–R14 must cover; LL has none |
| Outer loop (triggers → tasks) | **deferred (P9, K3 workflows)** — but adopt noodle's *shape* now: a file-API (`orders`/`control`/`ack` as JSON/NDJSON) that produces `task.submit` calls, and a single mode dial `auto/supervised/manual` as one door over admit/dispatch/settle | noodle itself is not a dependency (dormant, unsandboxed) |
| Playbooks / craft | **pstack markdown, ported into `.claude/skills/`** for *building* HEE (DW-1) | Not engine runtime; the 9-field brief (GOAL SCOPE CONTEXT ACCEPTANCE VERIFY TIMEBOX FORBIDDEN REPORT STANDING) becomes the `task.submit` payload contract |
| Memory / learning | **brainmaxxing hooks + pstack `reflect`/`correct`** | Replace the cron curator agents (V4-19/26) with on-demand `/reflect` and periodic `/meditate`; the `correct` ladder (architecture → types → lint-that-names-the-fix → test → docs, each proven to fail on a real past mistake) upgrades REQUIREMENTS rank 9 |
| Behaviour-level proof | **a HEE feature map** (one file per release action, the four H2s) | Written at P1; the "doctor first" rule is check-card done-criterion material |
| Review evidence | **deep-diff-forge** via K6 live-verifier-adapter | Seal patch digest + binary version with the observation; refuse zero-file runs (it returns rc=0 and `ranked: []` on empty stdin — AP-29) |
| Floor / display | **herdr** (Luke, 2026-10-05, V4-87): what Omarchy ships, what HEE adapts to, Firstmate's session backend | LoomLattice's glass/navigator (Zellij) is not built; LL contributes only its chain discipline |
| Jev | HEE only, held (H-8) | "Jev scores it" cannot happen before P9 |

## 3 · The loop as it would actually run

```
backlog item ──▶ orders file (noodle shape) ──▶ task.submit {9-field brief}
                                                   │ route (R02–R13 + capability floor)
                                                   ▼
                                    candidate namespace (bwrap, K2)  ◀── playbook steps as todolist, skip:<reason>
                                                   │
                       observations ──▶ decide (K4) ◀── feature-map drive + doctor
                       (ddf rank.v0, tests, lints)   │
                                                   ▼
                                     settle: receipt (hash-chained) → ledger (SQLite) → events.subscribe
                                                   │
                                            /reflect → brain/ ; /correct → one new door
```
Every arrow is HEE-owned. The other repos supply the *format* of the brief, the *shape* of the orders file, one observation source, the memory hooks and the playbooks for the humans and agents building it.

## 4 · What changes in the HEE v4 plan

1. **Walking skeleton first (unchanged from the 2026-10-04 review):** `task.submit` → 3-table SQLite store under `transition` → sync dispatcher to local ollama via spawn door → bwrap candidate → `decide` → `kill -9` + restart test. 6 crates, 6–8 slices.
2. **P1 adds the feature map** (22 release actions × four H2s) as the behaviour-level verification substrate; module-card done criteria point at feature files instead of restating them.
3. **P2 adds deep-diff-forge** as an observation source through the live-verifier-adapter, with digest+version sealing and the zero-file refusal.
4. **Recovery table before P3**, written as explicit `(state, observation) → Event` rows, using noodle's reconcile cases (live PID adopt / stale active / stuck merging) as the checklist.
5. **The mode dial** (`auto/supervised/manual`) becomes one door that gates admit, dispatch and settle; today's HOLD and grant sweep are the `manual` setting.
6. **Port pstack + brainmaxxing into `.claude/`** of v4 for the *build*: poteto-mode router, the 23 playbooks, 24 principles, `how`/`architect`/`interrogate`/`swarm`, `/reflect`, `/correct`. Portable per frontmatter; Cursor-only keys dropped. This replaces the hee4-curator/workflow-curator cron roster.
7. **Receipt chain**: add `hash_prev`/`hash_self` + periodic Merkle checkpoint to the receipt card; one format.
8. **Floor decided: herdr** (V4-87). `hee4-floor-display` owns the herdr adapter; no Zellij projection.

## 5 · What not to take
- noodle as a runtime (dormant, unsandboxed, schedule agent runs in the primary checkout with bypass).
- LL's in-memory daemon state, its socket, its router shape-without-transport, its `fable.receipt.v1`.
- The dev_ops doc's NAM/ANAM framework, metrics and "breakthroughs" (fiction); its WSL wrapper and 2025 dependency pins.
- deep-diff-forge's learning/promotion gate as anything but advisory (a second self-promoting policy engine).
- Any Jev scoring path before H-8.

## 6 · Proposed decision rows (for Luke; none recorded)
- **V4-77 (fix the dangling cite)** Ownership table §2 adopted: HEE owns unit-of-work, verdict, receipt, socket, router, isolation, recovery.
- **V4-78** One receipt format: HEE receipt v1 + hash chain + Merkle checkpoint.
- **V4-79** Outer loop stays P9; the orders-file shape and the mode dial land in P1 as `task.submit` producer and one door.
- **V4-80** pstack/brainmaxxing ported into v4 `.claude/` for the build; cron roster retired.
- **V4-81** Feature map at P1; deep-diff-forge as observation at P2.
- ~~**H-new** Floor substrate~~ decided herdr, V4-87.
