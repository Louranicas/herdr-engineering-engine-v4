---
name: setup-pstack
description: Configure which models pstack uses per role and at what reasoning budget. Detects your available models and writes an always-applied rule that overrides the skill defaults. Use for /setup-pstack, "configure pstack models", "pstack budget", or changing pstack's model choices.
---

> Ported note: Cursor-only in substance. It wrote `~/.cursor/rules/pstack-models.mdc`; in Claude Code edit `.claude/skills/models.md` by hand instead (Claude family only; the `Agent` tool takes `model: opus|sonnet|haiku`). The budget ladder (`max`/`xhigh`/...) has no Claude Code equivalent. Step 7 still applies.

# Setup pstack

Write `.claude/skills/models.md`, an always-applied rule that sets pstack's model per role.

## Steps

### 1. Detect available models

Enumerate the model slugs you can pass to an `Agent` subagent in this session (`opus`, `sonnet`, `haiku`, or a full `claude-*` id). That is the dependable source. If `claude` lists entitled models (`/model` in the CLI), prefer it for completeness. If you cannot detect any, ask the user to paste the slugs they have access to. Never write a real slug you have not confirmed is available. The aliases `inherit-parent` and `auto` are always valid even though they are not detected slugs.

### 2. Load current state

The default role-to-model mapping is the rule shape shown in step 5 below. If `.claude/skills/models.md` already exists, read it and treat its `# budget` line and its role values as the current choices. Otherwise start from those defaults. A line whose role is not in step 5, such as `how critics`, is from a retired role. Drop it.

### 3. Budget, map, and confirm

**(a) Ask for a budget.** Prefer AskUserQuestion over free text. Offer these four options with these exact labels, and name the current budget when the rule records one.

- `unlimited — keep max`
- `large — xhigh reasoning`
- `medium — high reasoning`
- `small — medium reasoning`

**(b) Apply it.** Build the working table from the skill defaults, and on a re-run keep any role you changed by family, list, or alias (`inherit-parent`, `auto`). `unlimited` leaves every effort as in that table. `large`, `medium`, and `small` set the effort token of every real slug, panel entries included, to `xhigh`, `high`, or `medium`. The effort token is the last token, or the one before a trailing `fast`, on the ladder `max` > `xhigh` > `high` > `medium` > `low`. If the result is not a detected slug, use the same family's detected slug with the highest effort at or below the target, else mark the role as needing a choice. `inherit-parent` and `auto` do not change. (Cursor's effort suffixes such as `-max` or `-medium` have no Claude Code equivalent; choose the tier, `opus`, `sonnet`, or `haiku`, instead.)

**(c) Show the roles and confirm.** Show every role with its model, marking any real slug not in the detected set as needing a choice. Also list each line step 2 dropped. Ask whether to accept as-is or change specific roles, offering the detected models plus `inherit-parent` and `auto` (both mean: this role runs on the parent chat model, which is how Auto users stay on Auto) as the options. Prefer AskUserQuestion over free text. For panel roles (arena runners, architect runners, interrogate reviewers) the value is a list, and one subagent runs per entry, alias entries included, so the list length sets the count. `arena cross-judge pool` is also a list, but Arena selects one value from it whose model family differs from the parent's when possible. `swarm workers` is the default model for every worker unless a race or comparison assigns another model per arm.

### 4. Validate

Every real slug written must be in the detected set. `inherit-parent` and `auto` always pass. If a chosen real slug is not available, stop and ask again.

### 5. Write the rule

Write `.claude/skills/models.md` with `alwaysApply: true`, a `# budget` line with the chosen label and its target effort, and one line per role, using the same labels poteto-mode uses. Overwrite the whole file so re-runs stay idempotent. Shape:

```
---
description: pstack per-role model choices (overrides skill defaults)
alwaysApply: true
---
# pstack model configuration. One line per role. Delete a line to fall back to the skill default.
# `inherit-parent` or `auto` as a value: the role runs on the parent chat model (omit `Agent` `model`). Alias entries in a panel list still count toward its fan-out.
# budget: unlimited (max)
feature, refactoring: claude-sonnet-5-5
bug-fix: claude-sonnet-5-5
perf-issue: claude-sonnet-5-5
hillclimb: claude-sonnet-5-5
judgment and prose: claude-opus-5-5
hardest tasks: claude-opus-5-5
how explorer: claude-sonnet-5-5
how explainer: claude-opus-5-5
why investigators: claude-sonnet-5-5
why synthesizer: claude-opus-5-5
reflect tooling: claude-opus-5-5
reflect judgment, divergent, synthesizer: claude-opus-5-5
arena runners: claude-opus-5-5, claude-opus-5-5, claude-sonnet-5-5
arena cross-judge pool: claude-opus-5-5, claude-opus-5-5, claude-sonnet-5-5
swarm workers: claude-sonnet-5-5
architect runners: claude-opus-5-5, claude-opus-5-5, claude-sonnet-5-5
interrogate reviewers: claude-opus-5-5, claude-opus-5-5, claude-sonnet-5-5
```

### 6. Confirm

Tell the user the rule was written and that it applies to new sessions. Re-running this skill updates it.

### 7. Offer a verification skill (optional)

Check whether the project has a way to drive the real app for proof (a `verify-*` skill, or an existing harness). If not, offer once: "want a project-local verification skill, so agents can drive the app the way a user does and prove changes work? I can generate one with /create-verification-skill." On yes, invoke `/create-verification-skill` (installed at `.claude/skills/create-verification-skill`). On no, move on without pushing.
