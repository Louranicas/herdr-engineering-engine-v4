# pstack for HEE v4 (Claude Code port)

Local integration: [Poteto Weave](</mnt/storage-10tb/hee4-evidence/prototypes/turso-tool-context/assimilation-20261005/README.md>) is the Herdr context and tool-chain framework inspired by Lauren Tan's pstack. Its project map links back here. This local reference does not change upstream attribution or skill behavior. Active skill directories are siblings of this provenance directory under `.claude/skills/`.

Lauren Tan's [pstack](https://github.com/cursor/plugins/tree/main/pstack) 0.15.9 (MIT) plus the brain
loop from [brainmaxxing](https://github.com/poteto/brainmaxxing) (MIT), ported to Claude Code skills for
**building HEE v4**. Sources, hashes and every porting change: `PROVENANCE.md`. Licenses: `LICENSE`.
This is Claude tooling, not engine code (allowed under the V4-0 HOLD).

## Invoke in Claude Code
- Every directory here is a skill: `/<name>` in the prompt, or name it ("apply poteto-mode"). 49 of 53 are
  `disable-model-invocation: true`, so they run only when you (or a skill that cites them) invoke them.
- Subagents: `.claude/agents/poteto-agent.md` (`subagent_type: "poteto-agent"`) and
  `.claude/agents/comment-sicko.md` (`subagent_type: "comment-sicko"`, used by `/no-comments`).
- Models per role: `models.md` (edit by hand; `/setup-pstack` is Cursor-only).
- Brain hooks are opt-in: `.claude/hooks/BRAIN-HOOKS-INSTALL.md`.

## The router: `/poteto-mode`
Reads the task, copies the matching playbook's steps into a todo list, and names the principle behind each
decision. Non-negotiables route to `how`, `architect`, `swarm`/`arena`, `interrogate`, `unslop`,
`technical-writing`, `no-comments`, `benchmark-checklist`, `show-me-your-work`. Playbooks live in
`poteto-mode/playbooks/`; `figure-it-out` designs a bespoke one when none fits.

## 23 playbooks by group
| Group | Playbooks |
|---|---|
| Understand | investigation · runtime-forensics · trace-forensics · session-pickup |
| Build | feature · bug-fix · refactoring · prototype · visual-parity · perf-issue · hillclimb |
| Plan and run | multi-phase-plan · autonomous-run · orchestrate · autopilot-full · autopilot-stack · pause-safely |
| Ship | opening-a-pr · babysit · shipping · worktree-cleanup |
| Meta | authoring-a-skill · eval |

## Workflow skills (26)
`architect` `arena` `automate-me` `benchmark-checklist` `blast-radius` `bro` `correct`
`create-verification-skill` `figure-it-out` `how` `interrogate` `maintain-verification-skill`
`make-bot-ui` (Cursor-only, reference) `no-comments` `poteto-mode` `recall` `reflect` `setup-pstack`
(Cursor-only, reference) `show-me-your-work` `swarm` `tdd` `teach` `technical-writing`
`typescript-best-practices` `unslop` `why` · brain loop: `brain-reflect` `brain-ruminate` `brain-meditate`.

## 24 principles (`principle-*`)
| Group | Principles |
|---|---|
| Core | laziness-protocol · foundational-thinking · redesign-from-first-principles · attack-the-premise · subtract-before-you-add · minimize-reader-load · outcome-oriented-execution · experience-first · exhaust-the-design-space · build-the-lever |
| Architecture | model-the-domain · boundary-discipline · type-system-discipline · make-operations-idempotent · migrate-callers-then-delete-legacy-apis · separate-before-serializing-shared-state |
| Verification | prove-it-works · fix-root-causes · sequence-verifiable-units · test-behavior-not-implementation · explain-the-number |
| Delegation | guard-the-context-window · never-block-on-the-human |
| Meta | encode-lessons-in-structure |

## Three things that differ from Cursor
1. **Subagents.** Cursor's `Task(readonly, environment: cloud, run_in_background)` is Claude Code's `Agent`
   tool: `general-purpose` (full tools), `Explore` (read-only), `isolation: "worktree"` for writers, always
   local and background. `AskQuestion` is `AskUserQuestion`. Transcripts are `~/.claude/projects/<slug>/`.
2. **Models.** No `~/.cursor/rules/pstack-models.mdc`; roles map in `models.md`, Claude family only
   (opus for judgment, sonnet for workers, haiku for explorers). Cross-family panels become cross-tier.
3. **Not bundled.** `/deslop`, `control-ui`, `control-cli` (`cursor-team-kit`), Bugbot, Cursor's
   `create-skill` (use `anthropic-skills:skill-creator`). Each affected file carries a `> Ported note:`.

## HEE v4 fit
pstack's rigor (prove it works, name the data shape first, verify before declaring done) matches the
repo's law in `hee4-brief`; where they differ, `CLAUDE.md`, `CHARTER.md` and the HOLD win. The brain
(`brain/`) is build memory only; decisions and design stay in `plan/`, `docs/` and the vault.
