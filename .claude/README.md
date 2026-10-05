# `.claude/`: the HEE v4 project harness

What each file does and how it is proven. Planning-only under the HOLD (H-5, V4-0): nothing here is
engine code. All hooks are **advisory** (warn, never block), bounded at ≤ 2 s, make no network or
Jev call, and exit 0 on any internal error. Roster runs (`ops/roster/run-agent.sh`) pass
`--settings ops/roster/<agent>/settings.json` with `disableAllHooks: true`, so none of these hooks
run inside a roster agent.

## Proof (run both; each prints one typed line)
```
python3 .claude/hooks/tests/run_all.py                         # hooks proven=N/N verdict=PASS
python3 .claude/skills/hee4-lessons/tests/check_pointers.py    # pointers resolved=N/N … verdict=PASS
python3 .claude/skills/hee4-lessons/tests/check_pointers.py --control   # control cases=k/k verdict=PASS
```
`run_all.py` takes its world from the hooks **registered** in `settings.json` (not a list): every
registered script needs `tests/test_<stem>.py`, which feeds sample hook JSON on stdin and must show
at least one case that **fires** and one that **stays quiet**. A missing test is a FAIL by name.
On 2026-10-01 each hook was also neutered or mutated in place (nine mutants: drop the output, drop
EXCLUDED, glob crossing dirs, drop `cd` tracking, first-vs-last arg, frozen readiness line, no
timeout) and each mutant turned `proven` below N/N.

## Files
| Path | What it does | Proven by |
|---|---|---|
| `settings.json` | `permissions.deny` (v3 Edit roots, `hee3-gate`, `hee3-recapture`; unchanged) + `hooks` registering the three scripts below, `timeout: 2` (seconds) | JSON loads; `run_all.py` enumerates it |
| `hooks/hee4-session-start.sh` | SessionStart (startup/resume/clear/compact): ≤ 12-line banner as context: HOLD state from `hee4db get held H-5`, the fence line quoted from `CLAUDE.md`, `~/handoffs/HEE4_RESTART.md` (or ABSENT), newest `HEE4_HANDOVER_*`, "first: just verify", the `hee4db readiness` and `hee4db check` verdict lines (each `timeout 0.4`; UNMEASURED if unreachable), the commands and skills | `tests/test_hee4-session-start.py`: two fake worlds differing in every field (whole-line asserts), DB unreachable, DB hanging 10 s (< 2 s total), missing binary |
| `hooks/hee4-regen-nudge.sh` + `lib/regen_nudge.py` | PostToolUse Edit\|Write: names the follow-up an edited file owes: design-conflict register → `just regen`; a LEGEND file → `just repin <KEY>` (KEY from `ops/checks/module_funnel.py` LEGEND); a `hee4db` world file or card or `MODULES.toml` → `hee4db ingest`; then `just verify` (Jev Fit Map: ingest only). Silent otherwise. No list kept here: LEGEND and hee4db `WORLD_FILES`/`WORLD_GLOBS`/`EXCLUDED` are imported | `tests/test_hee4-regen-nudge.py`: exact follow-up lists (whole-list equality), quiet paths (incl. EXCLUDED `docs/INDEX.md`, a nested dir), a fake LEGEND key `ZZ` |
| `hooks/hee4-v3-guard.sh` + `lib/v3_guard.py` | PreToolUse Bash: warns when a command names a frozen v3 path with a write verb (rm/mv/chmod/tee/`sed -i`, cp/rsync destination, `>` redirect, mutating `git`, cargo/just after `cd v3`). v3 roots are read from this `settings.json` deny rules. Defence in depth behind the read-only trees (V4-68) | `tests/test_hee4-v3-guard.py`: write verbs fire; reads stay quiet ( `cp` FROM v3, `git log`, v4 reference copies), a fake deny root |
| `hooks/hee4-config-guard.sh` + `lib/config_guard.py` | PreToolUse Edit\|Write\|MultiEdit: names the door a file belongs to before it is edited (lint floor, `Cargo.toml` lints, crate `[lints]`, `gate.toml`, `layers.toml`, `.cargo/`, toolchain, `tools/gate`, `tools/lint-ratchet`, `tools/tests/`, `ops/checks/`, this `settings.json`, `.claude/hooks/`). From ECC config-protection (2026-10-05); the refusal itself is `tools/lint-ratchet` in the commit tier, so this hook only warns | `tests/test_hee4-config-guard.py`: every door row fires with its own path, MultiEdit per-edit paths, a worktree's own `gate.toml`; quiet on code, docs, cards, paths outside any tree, garbage stdin |
| `hooks/hee4-edit-facts.sh` + `lib/edit_facts.py` | PreToolUse Edit\|Write\|MultiEdit: first touch of `crates/<crate>/` per session gets the module card(s) from `MODULES.toml` and the crates that import it from `crates/*/Cargo.toml`; a new file owes its owning module. From ECC gateguard (2026-10-05), facts supplied by the hook instead of demanded from the agent | `tests/test_hee4-edit-facts.py`: importers counted independently, exact-module card, new-file line, a fake tree (derived, not hard-coded); quiet on the second touch, non-crate paths, garbage |
| `hooks/hee4-context-pressure.sh` + `lib/context_pressure.py` | PostToolUse `*`: says once per band (60/75/90% of `$HEE4_CONTEXT_WINDOW`, default 200000; a usage above it implies 1M) the token count from the transcript's last usage record. From ECC suggest-compact (2026-10-05), its context signal only | `tests/test_hee4-context-pressure.py`: band fires with the measured count, same band quiet, newest record wins, 1M inference; quiet on no usage, missing transcript, no session |
| `hooks/lib/hee4_paths.py` | shared: the HEE v4 tree a path belongs to (nearest ancestor with `gate.toml` + `crates/`), MultiEdit paths, the hook output shape | through the three tests above |
| `commands/verify.md` | `/verify`: `just verify`, verdict line verbatim | — (prompt) |
| `commands/restart.md` | `/restart`: read `HEE4_RESTART.md` (or newest handover), `hee4db recipe restart` (handles absence) | — |
| `commands/highway.md` | `/highway <module>`: `hee4db highway $ARGUMENTS` | — |
| `commands/module.md` | `/module <name>`: highway → card → design section → AP/EX ids → open questions | — |
| `commands/regen.md` | `/regen`: `just regen`, then `just verify` only on PASS | — |
| `commands/hee4-status.md` | `/hee4-status`: readiness, P0 holds, roster agent status (`/status` is built in, so not that name) | — |
| `commands/lessons.md` | `/lessons <situation>`: triggers table → matching reference lines → homes | — |
| `skills/hee4-module-slice/` | DW-1 procedure for AFTER "start coding"; refuses while H-5 is open | — |
| `skills/hee4-brief/` | subagent brief template: 14 LAW lines, each with its incident id | — |
| `skills/hee4-review-lenses/` | three review lenses for Rust diffs (silent failure, type design, Rust review) from ECC's reviewer agents (2026-10-05), minus every class a lint or the gate already catches; each finding names the higher rung that should have caught it. Used by `hee4-reviewer` step 3 | — |
| `skills/hee4-lessons/` | index (never a copy) of AP/D, EX/A, spells, mistakes, L/K/S, v2 and prototype lessons; `reference/triggers.md` is situation → ids | `tests/check_pointers.py` (+ `--control`: one plant per pointer grammar) |
| `agents/hee4-reviewer.md` | adversarial verifier (opus): read-only except its report; FACT/DESIGN/PROPOSAL sources; typed verdict | — |
| `agents/hee4-builder.md` | module builder (opus) for after "start coding": refuses on H-5, follows `hee4-module-slice`, one worktree per slice | — |
| `agents/hee4-curator.md`, `agents/hee4-workflow-curator.md` | roster agents run by cron through `ops/roster/run-agent.sh`; **do not edit** (the roster depends on them) | their roster selfchecks |

## Formats used (from the Claude Code docs, 2026-10-01; binary 2.1.285)
- Hooks: `hooks.<Event>[].{matcher, hooks[].{type:"command", command, timeout}}`; `timeout` is in
  seconds; `$CLAUDE_PROJECT_DIR` is set for hook commands. Exit 0 + JSON `hookSpecificOutput.
  {hookEventName, additionalContext}` adds context (PostToolUse, PreToolUse); SessionStart plain
  stdout is added as context; exit 2 would block (PreToolUse), which no hook here uses.
- Commands: `commands/<name>.md`, frontmatter `description`, `argument-hint`, `allowed-tools`;
  `$ARGUMENTS` substitution. Commands and skills share one namespace (`/name`).
- Skills: `skills/<name>/SKILL.md`, frontmatter `name`, `description`.
- Agents: `agents/<name>.md`, frontmatter `name`, `description` (required), `tools`, `model`
  (`opus`), `skills` (preloaded).

## Linked from, and linking back to
- Restart pointer: [`~/handoffs/HEE4_RESTART.md`](file:///var/home/Louranicas/handoffs/HEE4_RESTART.md).
- Repo [README.md Quickstart](../README.md) and [CLAUDE.md](../CLAUDE.md).
- Habitat [Quick Start](obsidian://open?vault=herdr-fedora-habitat.vault&file=20%20Usage%2FQuick%20Start) (v4 section).
- [ULTRAMAP](file:///var/home/Louranicas/hee4-evidence/design/ULTRAMAP.md) §7 points to `hee4-module-slice` and `hee4-lessons` for per-module work.
- Ops DB registry: `hee4db recipe skills`, `hee4db recipe agents`, `hee4db recipe reflexes` and `hee4db recipe restart` ingest this folder's skills, agents and hooks (derived; this folder stays the home).
- Vault: [Master Index](obsidian://open?vault=herdr-engineering-engine-v4.vault&file=00%20Hub%2F00%20-%20HEE%20v4%20Master%20Index) · [HEE v4 Ops Database](obsidian://open?vault=herdr-engineering-engine-v4.vault&file=90%20Database%2FHEE%20v4%20Ops%20Database).
