# Herdr Engineering Engine v4: planning repository

**Status (2026-10-02): PLANNING ONLY. Luke's HOLD: no code until he says "start coding".** This repository holds the v4 charter, decisions, gate intents and the staged v3 modules. It contains no buildable code.

**Jev (2026-10-02, `plan/DECISIONS.md` V4-74):** no v4 text goes to Jev (ATLAS H-10a). The installed Jev door refuses every HEE v4 name and path for every sender, and `just jev-entry` reads that back. Re-run it at P0 entry and after any Jev or Claude Code update; never quote an old line.

| Path | What | Owner / home |
|---|---|---|
| `CHARTER.md` | purpose, scope, done-line, authority, the hold | this repo |
| `plan/DECISIONS.md` | v4 decision register (append-only) | this repo |
| `gates/REQUIREMENTS.md` | the scaffolding requirements that will become gate steps | this repo |
| `docs/INDEX.md` | where every design and learning lives | this repo → `~/hee4-evidence`, v4 vault |
| `migrated/v3-b5367bc/` | verbatim v3 HARDEN modules + tests + schemas, **staged, not wired** (`MIGRATION.md`, `MANIFEST.sha256`) | this repo |
| `.claude/agents/hee4-*.md`, `ops/roster/` | the agent roster: `hee4-curator` (corpus) and `hee4-workflow-curator` (workflows and loops), one runner `ops/roster/run-agent.sh`; schedule in `ops/roster/README.md` |
| `justfile`, `runbooks/*.toml`, `ops/checks/` | ops recipes (`just verify`, `regen`, `repin KEY`, `trace`, `render`, `highway MODULE`, `jev-entry [SINCE]`, `roster-selfcheck AGENT`, `drill`, `restore-v3-modes confirm`), one habitat runbook per recipe, the checks they call (`ops/checks/render/README.md`, `ops/checks/regen.py`, `ops/checks/funnel_trace.py`) | this repo |

## Quickstart (2026-10-02; commands re-run and expectations updated by V4-67 and V4-74)

### 1 · Orient (state is computed, never trusted from a note)
**Verify everything: `just verify`.** It runs cite pins, the module funnel and its control, `hee4db check`, the DB control, the `jev-entry` control, the funnel trace and the mermaid render. It runs every step even after one fails, and ends `verify verdict=PASS|FAIL steps=N/M` (PASS only when N = M; expect it, not a count: a count literal goes stale with the next step, AP-28). `just` (no arguments) lists the recipes. Each recipe has a runbook in `runbooks/<name>.toml` (precondition, step, verify), which `habitat-runbook preview|run <name> --directory runbooks` checks and runs. After a register edit run `just regen`; after editing a cited file run `just repin <KEY>`.
```bash
cd ~/herdr-engineering-engine-v4
just verify                                       # every check, one verdict: expect verify verdict=PASS steps=N/N
habitat-scope set --session "$HABITAT_SCOPE_SESSION" --charter "HEE v4" ~/herdr-engineering-engine-v4 ~/hee4-evidence /var/mnt/STORAGE-10TB/fedora-obsidian-vaults/herdr-engineering-engine-v4.vault
python3 ops/checks/module_funnel.py | tail -1     # the planning corpus is coherent: expect verdict=PASS
hee4db check 2>&1 >/dev/null | tail -1           # ops database + substrate alignment: expect verdict=PASS measured=N/N (17 checks on 2026-10-02)
hee4db readiness 2>&1 >/dev/null | tail -1       # expect deploy_ready=0/53 ready_to_build=40 arch_review=0 blocked_luke=0 deferred=13 (2026-10-01, after the DC ratification V4-55…V4-66)
just trace | tail -1                             # (ops/checks/funnel_trace.py; the old evidence path still works) vault index → card → design section → maps → phases → H → DC → migrated → highway, 53 modules: expect verdict=PASS
hee4db recipe blocks-phase P0 2>/dev/null | python3 -c 'import json,sys; print([r["id"] for r in json.load(sys.stdin)["rows"]])'   # what still blocks P0: each id is held for Luke (ATLAS §5); on 2026-10-02: H-1, H-5, H-27
just jev-entry                                   # P0 Jev entry: expect verdict=PASS senders_measured=4/4 sent_engine_rows=0/<sent>
```

### 2 · Find anything
| You need | Go to |
|---|---|
| **Everything about one module, in one call** (card, design section, phases, budget, readiness, open DCs, held items, AP/EX/D, migrated paths, decisions, V-reports, maps) | `hee4db highway <module>`; per phase `hee4db highway --phase P3`; per conflict `hee4db highway --dc DC-38`. Skill: `hee-v4-corpus` *(rev 2026-10-01 alignment)* |
| What v4 is, its scope and done-line | `CHARTER.md`, then `~/hee4-evidence/design/DEPLOYMENT_ATLAS.md` §1 |
| Architecture (authority) | `~/hee4-evidence/design/ULTRAMAP.md`; its §7 points at the design detail |
| **Per-module design**: logic flow, interfaces, actions, sockets, loops, refusals, must-nots, size budget | the vault **[Module Design Index](obsidian://open?vault=herdr-engineering-engine-v4.vault&file=15%20Module%20Design%2F00%20-%20Module%20Design%20Index)**; every card links its own section |
| API (22 actions) · sockets/IPC (S-1…S-11) · CLI commands | vault `16 System Maps/`: `API Map`, `Socket and IPC Map`, `Command Map` |
| Runtime loops (RL-1…11) · dev workflows (DW-1…7) · end-to-end traces (E2E-01…12) | vault `16 System Maps/`: `Workflow and Loop Map`, `End-to-End Flow Traces` |
| State tables and transitions · refusals · size budgets | vault `16 System Maps/`: `State and Transition Map`, `Error and Refusal Map`, `Anti-Bloat Budget` |
| A module's funnel (phase, H-items, v3 basis, done criteria, AP/EX/D) | `modules/<crate>/<module>/MODULE.md` (53 cards, `modules/MODULES.toml`) |
| Anti-patterns · exemplars · drift controls | `docs/ANTIPATTERNS.md`, `docs/EXEMPLARS.md`, `docs/DRIFT_AND_OVERENGINEERING.md` |
| Decisions · held-for-Luke items | `plan/DECISIONS.md` (append-only) · ATLAS §5 (the H-list, single home); a phase's open holds: `hee4db recipe blocks-phase Pn` |
| Jev egress and the P0 Jev entry | `just jev-entry` (`hee4db jev-entry`); per day and sender: `hee4db record jev-daily` then `hee4db recipe jev-egress`; Jev fits: vault `50 Jev/Jev Fit Map`, `hee4db highway --jev` |
| Query any of it as data | `hee4db help` · `hee4db recipes` · `hee4db search <words>` (see `ops/db/README.md`) |
| **Restart in a fresh context** | `~/handoffs/HEE4_RESTART.md` (routes + queries), then `hee4db recipe restart` and `just verify` |
| **Claude tooling for this repo** (hooks, slash commands /verify /restart /highway /module /regen /hee4-status /lessons, project skills, agents) | [`.claude/README.md`](.claude/README.md); the skills and agents are also queryable: `hee4db recipe skills`, `hee4db recipe agents`, `hee4db recipe reflexes` |
| Lessons by trigger (anti-patterns, exemplars, spells, mistakes from v2/v3/prototypes) | `.claude/skills/hee4-lessons/` (`/lessons <situation>`) |

### 3 · Writing a module, after "start coding" (DW-1)
1. Read the card, then its design section, then the system maps it names, then `Anti-Bloat Budget`.
2. Write a one-page FLOW and a compiling skeleton. Run at most 2 design rounds (D-02, D-03).
3. Write the slice. Tier-1 checks per commit, Tier-2 repo gate plus cold clone per stack (ATLAS §4).
4. Where the design conflicts with an authority, the authority wins. Record it in the design conflict register (DC-nn).
5. After editing any file the cards cite: `just repin <KEY>`. After editing a file home the database derives from: `hee4db ingest`; after a design-conflict register edit: `just regen`. Then `just verify`.

### 4 · Rules that do not bend
- **The HOLD:** no code until Luke says "start coding" (V4-0).
- **The fence:** never work in v3 (`CLAUDE.md`).
- **Commits and pushes:** commit only when asked; push only on Luke's word.
- **Jev:** no v4 text goes to Jev (H-10a); `just jev-entry` must PASS before P0 and after any Jev change.
- **One topic, one home:** the vault and the database link and index; they never become a second home.

Design and evidence live in `~/hee4-evidence/` (ULTRAMAP, DEPLOYMENT_ATLAS, PROCESS-LEARNINGS, JEV_QUESTION_RESPONSES). Navigation lives in the v4 vault. One topic, one home: the vault links here, never copies.

State is computed, never stored here: `ops/roster/run-agent.sh hee4-curator selfcheck` prints it (see `ops/roster/README.md`).
