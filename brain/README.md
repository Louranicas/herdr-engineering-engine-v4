# brain/ (HEE v4 build memory)

What Claude learned **while building HEE v4** and would need again in a different session: corrections
Luke gave, gotchas in this repo's tooling, patterns that held, friction worth not repeating. Ported from
[brainmaxxing](https://github.com/poteto/brainmaxxing) (MIT). Browsable as an Obsidian vault if you
open this folder; no frontmatter needed.

## What does NOT go here (one topic, one home)
- Decisions, charter, requirements: `CHARTER.md` → `plan/DECISIONS.md` → `gates/REQUIREMENTS.md`.
- Module design, maps, learnings catalogued by id (AP-/EX-/L/K/S): the design vault
  `/mnt/storage-10tb/fedora-obsidian-vaults/herdr-engineering-engine-v4.vault` and `docs/`. Link to it
  (`obsidian://open?vault=herdr-engineering-engine-v4.vault&file=...`), never copy it.
- Readiness, holds, roster state: the ops DB (`hee4db`, `just verify`). Derived facts live there.
- Session state ("I was in the middle of X"): `~/handoffs/HEE4_RESTART.md`, not a brain note.
- Engine code: none, anywhere, until "start coding" (V4-0 HOLD).

## Shape
- One topic per file, `lowercase-hyphenated.md`, under 50 lines. Split when longer.
- Group by directory (`codebase/`, `tooling/`, `preferences/`, ...). A new top-level directory gets
  an index-style entrypoint (links only).
- `# Title`, then bullets. No preamble, no prose essays.
- Link notes with `[[dir/file-name]]` wikilinks. Resolution: same dir, relative, then root.
- `index.md` is the root and is **generated**: the `brain-auto-index.sh` hook rebuilds it when a note
  is added or removed (bare wikilinks, grouped by directory). Do not hand-edit it beyond grouping.

## Durability test before writing
"Would I include this in a prompt for a *different* task?"
- Yes → a brain note.
- No, it is plan-specific → the plan or decision record.
- No, it is a skill issue → edit the skill under `.claude/skills/`.
- No, it needs follow-up → file it (handover or `just` backlog), not here.
- Can it be a check, hook, or script instead? → encode it (`principle-encode-lessons-in-structure`).

## Loop
[/brain-reflect](../.claude/skills/brain-reflect/SKILL.md) at session end or after a correction ·
[/brain-ruminate](../.claude/skills/brain-ruminate/SKILL.md) over past transcripts ·
[/brain-meditate](../.claude/skills/brain-meditate/SKILL.md) to prune and distil. Writing conventions
in this file; principles live in `.claude/skills/principle-*/` ([pstack port](../.claude/skills/pstack/README.md);
moved out of `pstack/` 2026-10-05, see `.claude/skills/pstack/PROVENANCE.md`), not duplicated here.

## Hooks (opt-in; see `.claude/hooks/BRAIN-HOOKS-INSTALL.md`)
`brain-inject.sh` prints `index.md` at session start; `brain-auto-index.sh` regenerates it after an
Edit/Write inside `$HEE4_BRAIN` (default `<repo>/brain/`).

## See also
- Candidate notes from measured workflow failures come from the habitat's shadow workflow-curator
  ([handoff](/mnt/storage-10tb/workflow-curator/docs/WORKFLOW_CURATOR_20261005.md)); proposals go to
  [hee4-scribe](../.claude/agents/hee4-scribe.md), the one writer here.
- Design vault [HEE v4 Master Index](</mnt/storage-10tb/fedora-obsidian-vaults/herdr-engineering-engine-v4.vault/00 Hub/00 - HEE v4 Master Index.md>) · habitat memory nexus [herdr habitat](</mnt/storage-10tb/fedora-obsidian-vaults/herdr.habitat.vault/Home.md>).
