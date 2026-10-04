# pstack model configuration (Claude Code port)

One line per role. This file replaces Cursor's `~/.cursor/rules/pstack-models.mdc`. Skills name a
role line and a default; the line here wins, a missing line falls back to the skill's default, and
`inherit-parent` or `auto` means "omit `model` so the role runs on the parent chat model".

Claude Code's `Agent` tool takes `model: opus | sonnet | haiku` (aliases) or a full id. Only the
Claude family exists here, so pstack's "different model family" for cross-judging and interrogation
panels becomes a different **tier** (opus vs sonnet vs haiku) or a differently framed prompt.
Cursor's effort suffixes (`-max`, `-xhigh`, `-fast`) have no equivalent; drop them.

| Role line | Model id | `Agent` alias | Why |
|---|---|---|---|
| `judgment and prose` | `claude-opus-5-5` | `opus` | judge |
| `hardest tasks` | `claude-opus-5-5` | `opus` | judge |
| `feature, refactoring` | `claude-sonnet-5-5` | `sonnet` | worker |
| `bug-fix` | `claude-sonnet-5-5` | `sonnet` | worker |
| `perf-issue` | `claude-sonnet-5-5` | `sonnet` | worker |
| `hillclimb` | `claude-sonnet-5-5` | `sonnet` | worker |
| `how explorer` | `claude-haiku-4-5-20251001` | `haiku` | explorer (read-only `Explore`) |
| `how explainer` | `claude-opus-5-5` | `opus` | judge |
| `why investigators` | `claude-sonnet-5-5` | `sonnet` | worker |
| `why synthesizer` | `claude-opus-5-5` | `opus` | judge |
| `reflect tooling` | `claude-sonnet-5-5` | `sonnet` | worker |
| `reflect judgment, divergent, synthesizer` | `claude-opus-5-5` | `opus` | judge |
| `arena runners` | `claude-opus-5-5, claude-sonnet-5-5, claude-opus-5-5` | one seat per entry | panel |
| `arena cross-judge pool` | `claude-opus-5-5, claude-sonnet-5-5` | pick a tier the parent is not on | judge |
| `swarm workers` | `claude-sonnet-5-5` | `sonnet` | worker |
| `architect runners` | `claude-opus-5-5, claude-sonnet-5-5, claude-opus-5-5` | one seat per entry | architect |
| `interrogate reviewers` | `claude-opus-5-5, claude-sonnet-5-5, claude-opus-5-5` | one seat per entry | judge |
| `brain-ruminate analysts` | `claude-opus-5-5` | `opus` | judge |

Explorers that only read (`how` explorers, `Explore`-type fan-outs) default to haiku; anything that
writes code defaults to sonnet; anything that judges, synthesizes, or designs defaults to opus.

Panel lists: one subagent runs per entry, so list length sets the fan-out. Edit this file by hand;
`/setup-pstack` is Cursor-only and does not write it.
