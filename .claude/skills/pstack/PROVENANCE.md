# Provenance

Ported 2026-10-04 into the HEE v4 planning repo as Claude Code tooling (not engine code; V4-0 HOLD
respected). Port script and rules were applied mechanically and are listed below so a re-port of a newer
upstream can repeat them. Nothing was committed or pushed; `.claude/settings.json` was not edited.

## Sources
| Source | Repo | Version / commit | Local checkout | License |
|---|---|---|---|---|
| pstack (Cursor plugin) | https://github.com/cursor/plugins (path `pstack/`) | plugin.json `0.15.9`; plugins repo HEAD `e43c7ee26e0038c6c1fa8380dd34ce86ff94cb2a` | `~/cursor-plugins/pstack` | MIT, Lauren Tan (verbatim in `LICENSE`) |
| brainmaxxing | https://github.com/poteto/brainmaxxing | commit `ec4d8e43dee639d6f5d94aed3ca16af7b298ecfc` | `~/brainmaxxing` | MIT (verbatim in `LICENSE`) |

## What was taken
- pstack `skills/*` (50 directories) → `.claude/skills/pstack/<name>/`, with `playbooks/`, `references/`,
  `scripts/` intact (the `scripts/` TypeScript under `poteto-mode/` needs `bun`; it was copied unmodified
  except `worktree-audit.sh`, see below).
- pstack `agents/poteto-agent.md`, `agents/comment-sicko.md` → `.claude/agents/`.
- brainmaxxing `.claude/hooks/inject-brain.sh`, `auto-index-brain.sh` → `.claude/hooks/brain-inject.sh`,
  `brain-auto-index.sh`; `.agents/skills/{reflect,ruminate,meditate}` → `brain-reflect`, `brain-ruminate`,
  `brain-meditate` (with `meditate/references/agents.md`, `meditate/scripts/snapshot.sh`,
  `ruminate/scripts/extract-conversations.py`).
- Not taken: pstack `automations/benny` (Cursor automations), `docs/guide` (Cursor-flavoured guide; the
  skills are self-describing), `assets/logo.png`; brainmaxxing `brain/principles/*` (16 notes: 15 overlap
  the 24 pstack `principle-*` skills, `cost-aware-delegation` is covered by `models.md`), `brain/plans/`,
  and the `brain`, `plan`, `review` skills (pstack's `poteto-mode` playbooks, `architect`, `interrogate`
  and the repo's `hee4-reviewer` cover them; `brain` skill conventions were folded into `brain/README.md`).

## Overlap note: `reflect` vs `brain-reflect`
pstack `reflect` spawns three reviewers over the transcript and routes learnings into **skill edits**.
brainmaxxing `reflect` is a single pass that writes learnings into **brain/**. They are distinct, so both
are kept: `reflect` (pstack, unchanged in intent) and `brain-reflect` (brainmaxxing, renamed).
`ruminate` and `meditate` have no pstack counterpart and are renamed `brain-*` for the same namespace.

## Changes made in porting (mechanical, every ported `.md`)
Frontmatter
- Kept only `name`, `description`, `disable-model-invocation`. Dropped Cursor-only `mode`, `icon`, `color`,
  `reminder` (poteto-mode) and `paths` (typescript-best-practices). Dropped `is_background` on agents.
- `name` set to the directory slug (`Poteto Mode` → `poteto-mode`, `Make Bot UI` → `make-bot-ui`,
  agent `Comment Sicko` → `comment-sicko`).
Subagent mechanics
- `Task` tool / calls / subagents → `Agent`; `subagent_type: generalPurpose` → `general-purpose`;
  `readonly: true` roles (how explorers and explainer, interrogate reviewers, arena judge) → `subagent_type:
  Explore`; `readonly: false` wording → "keep `general-purpose` so MCP stays available";
  `environment: "cloud"` / `cloud_base_branch` / "Cursor cloud agent" → local background `Agent` with
  `isolation: "worktree"` for writers; `run_in_background: true` → "background (the default)";
  `TeamCreate` (brain-ruminate) → N parallel `Agent` calls.
- `AskQuestion` → `AskUserQuestion`.
Paths
- `~/.cursor/rules/pstack-models.mdc` → `.claude/skills/models.md` (new file, role → model table).
- `~/.cursor/projects/<slug>/agent-transcripts` → `~/.claude/projects/<slug>/` (slug keeps its leading
  dash); also in `poteto-mode/scripts/worktree-audit.sh` (the only script edited).
- `.cursor/skills/`, `~/.cursor/skills/`, `~/.cursor/plugins/` → `.claude/...`; `.cursor/worktrees` →
  `.claude/worktrees`; `origin/main:pstack/skills/` → `origin/main:.claude/skills/pstack/`.
- brainmaxxing `.agents/skills/...` script paths → `.claude/skills/pstack/brain-*/...`.
Models
- `claude-opus-5-5-max` → `claude-opus-5-5`; `grok-4.7-xhigh-fast` → `claude-sonnet-5-5`;
  `gpt-5.6-sol-max` → `claude-opus-5-5`; `how explorer` default → `claude-haiku-4-5-20251001`.
  "Model family" fallback text rewritten for a single family. Cursor effort suffixes dropped.
Cursor built-ins
- `create-skill` → `skill-creator` (`anthropic-skills:skill-creator`); `/deslop` kept as a slash command
  with "not bundled, fall back to unslop"; `control-ui`/`control-cli` → the project `verify-<app>` skill
  or the `run` skill; "Cursor's built-in babysit skill" → "any other babysit skill"; Cursor `/loop` →
  Claude Code `/loop`; "Cursor restart" → "Claude Code restart"; MCP discovery → `mcp__<server>__*` tools.
- `> Ported note:` added at the top of 31 files (28 pstack skills, playbooks and references that still
  name a Cursor-side mechanism: setup-pstack, make-bot-ui, Bugbot triage, babysit, orchestrate, swarm,
  arena, interrogate...; plus the three `brain-*` skills).
Hooks (brainmaxxing)
- Brain dir is a seam `HEE4_BRAIN` (default `$CLAUDE_PROJECT_DIR/brain`); advisory (`trap exit 0`), no
  network; `find` called as `command find`.
- `auto-index`: brainmaxxing registered `matcher: "brain/"`, but Claude Code matchers match the tool
  name, so it never fired. The port matches `Edit|Write` and the script filters on
  `tool_input.file_path` being inside the brain. Tests added: `tests/test_brain-inject.py`,
  `tests/test_brain-auto-index.py` (same `Cases` harness as the repo's hooks).
- `inject`: adds a one-line header naming the brain path and `/brain-reflect`; caps output at 16 KB.

## Residual Cursor-only mechanics (kept, annotated, not portable)
- `make-bot-ui`: Cursor Grok Bot webhook routines (`update_state`, `api2.cursor.sh`). Reference only.
- `setup-pstack`: writes a Cursor rule and uses Cursor's effort/budget ladder. Reference only; edit
  `models.md` by hand.
- Bugbot (babysit, autopilot-*, bugbot-triage, `scripts/watch-pr/`): Cursor's GitHub review bot. Steps apply
  only if a repo has it; the triage posture generalises to any review bot.
- `/deslop`, `control-ui`, `control-cli` (`cursor-team-kit`): not bundled; fallbacks named inline.
- `worktree-cleanup` step 6 names macOS Cursor cache dirs as disk-reclaim examples (harmless).
- `poteto-mode/scripts/` (orch, watch-pr, check-plan) are bun/TypeScript tools; untested here.
- The `why` skill's "cursor location" and "precursor" are English, not Cursor.

## Verification at port time
- 53 skill dirs, every `SKILL.md` parses with `name` == dir and a `description`; 49 carry
  `disable-model-invocation: true`.
- `python3 .claude/hooks/tests/run_all.py` before and after the port: identical (`hooks proven=1/3`,
  the same five pre-existing host-specific failures in `hee4-regen-nudge` and `hee4-v3-guard`). The new
  brain tests pass standalone (`brain-inject.sh 5/5`, `brain-auto-index.sh 8/8`) and join `run_all.py`
  once the hooks are registered per `BRAIN-HOOKS-INSTALL.md`.
- `brain-auto-index.sh` run against the real `brain/` left `index.md` byte-identical.

## 2026-10-05 · flattened
Claude Code discovers project skills only at `.claude/skills/<name>/SKILL.md` (docs: code.claude.com/docs/en/skills), so the 53 skill directories were moved from `.claude/skills/pstack/<name>/` to `.claude/skills/<name>/` (measured in the 10-minute environment test). `pstack/` keeps LICENSE, PROVENANCE.md, README.md and models.md; references to `.claude/skills/pstack/models.md` are unchanged.

## 2026-10-05 · model invocation
`disable-model-invocation: true` removed from the workflow and principle skills so the model can load them through the Skill tool as poteto-mode's triggers intend (Cursor invoked them through the mode; Claude Code needs the flag off). Kept on `poteto-mode`, `automate-me`, `setup-pstack`, `make-bot-ui`, which are user-invoked modes/configurators. Luke, 2026-10-05: "make sure you can use the full array of p-stack skills".

## 2026-10-06 · habitat links
Local additions to the ported brain skills: brain-reflect names `hee4-scribe` as the one writer of `brain/` and points at the curator's measured failure families; brain-meditate and brain-ruminate read the curator's `playbook.md`. A re-port keeps these lines. Source: the workflow curator's link map, applied by the captain (Luke, 2026-10-06: "include bi directional links to the pstack brain skill").
