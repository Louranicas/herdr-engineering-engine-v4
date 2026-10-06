---
name: brain-reflect
description: >-
  HEE v4 build-memory variant (writes to brain/, not to skills): Reflect on the conversation and update the brain. Use when wrapping up, after mistakes or corrections, or when significant codebase knowledge was gained. Triggers: "reflect", "remember this".
---

> Ported note: from brainmaxxing (`reflect`), renamed `brain-reflect` because pstack ships its own `reflect`. The brain is `$HEE4_BRAIN` (default `<repo>/brain/`), the HEE v4 **build memory**: what Claude learned while building v4. It is not the ops DB and not the design vault (`/mnt/storage-10tb/fedora-obsidian-vaults/herdr-engineering-engine-v4.vault`); link to those, never copy. Conventions: `brain/README.md`.

# Reflect

Review the conversation and persist learnings — to `brain/`, to skill files, or as structural enforcement.

## Process

1. **Read `brain/index.md`** to understand what notes already exist
2. **Scan the conversation** for:
   - Mistakes made and corrections received
   - User preferences and workflow patterns
   - Codebase knowledge gained (architecture, gotchas, patterns)
   - Tool/library quirks discovered
   - Decisions made and their rationale
   - Friction in skill execution, orchestration, or delegation
   - Repeated manual steps that could be automated or encoded
3. **Skip** anything trivial or already captured in existing brain files
4. **Route each learning** to the right destination (see Routing below)
5. **Update `brain/index.md`** if any brain files were added or removed

## Routing

Not everything belongs in the brain. Route each learning to where it will have the most impact.

### Structural enforcement check

Before routing a learning to `brain/`, ask: can this be a lint rule, script, metadata flag, or runtime check? If yes, encode it structurally and skip the brain note. See the `principle-encode-lessons-in-structure` skill.

### Brain files (`brain/`)

In HEE v4, `brain/` has one writer, [hee4-scribe](../../agents/hee4-scribe.md) (PROTOCOL §4). Recurring workflow failures are already measured: `workflow-curator q "SELECT family, n FROM failure_modes WHERE brain_note IS NULL"` ([handoff](/mnt/storage-10tb/workflow-curator/docs/WORKFLOW_CURATOR_20261005.md)).

Codebase knowledge, principles, gotchas — anything that informs future sessions. This is the default destination. Writing conventions are in `brain/README.md`.

- One topic per file. File name = topic slug.
- Group in directories with index files using `[[wikilinks]]`.
- No inlined content in index files.

### Skill improvements (`.claude/skills/<skill>/`)

If a learning is about how a specific skill works — its process, prompts, or edge cases — update the skill directly.

### Backlog items

Follow-up work that can't be done during reflection — bugs, non-trivial rewrites, tooling gaps. File as a todo or backlog item.

## Summary

```
## Reflect Summary
- Brain: [files created/updated, one-line each]
- Skills: [skill files modified, one-line each]
- Structural: [rules/scripts/checks added]
- Todos: [follow-up items filed]
```
