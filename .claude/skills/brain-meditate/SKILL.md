---
name: brain-meditate
description: >-
  HEE v4 build-memory variant: Audit and evolve the brain vault — prune outdated content, discover cross-cutting principles, review skills for structural encoding opportunities. Triggers: "meditate", "audit the brain".
---

> Ported note: from brainmaxxing (`meditate`), renamed `brain-meditate` because pstack ships its own `reflect`. The brain is `$HEE4_BRAIN` (default `<repo>/brain/`), the HEE v4 **build memory**: what Claude learned while building v4. It is not the ops DB and not the design vault (`/mnt/storage-10tb/fedora-obsidian-vaults/herdr-engineering-engine-v4.vault`); link to those, never copy. Conventions: `brain/README.md`.

# Meditate

**Quality bar:** A note earns its place by being **high-signal** (Claude would reliably get this wrong without it), **high-frequency** (comes up in most sessions or most tasks of a type), or **high-impact** (getting it wrong causes significant damage or wasted work). Everything else is noise. A lean, precise brain outperforms a comprehensive but bloated one.

## Process

### 1. Build snapshots

```bash
sh .claude/skills/brain-meditate/scripts/snapshot.sh brain/ /tmp/brain-snapshot.md
sh .claude/skills/brain-meditate/scripts/snapshot.sh .claude/skills/ /tmp/skills-snapshot.md
```

Files are delimited with `=== path/to/file.md ===` headers. Also locate the auto-memory directory (`~/.claude/projects/<project>/memory/`).

### 2. Auditor (blocking — its report feeds step 3)

Spawn one `Agent` (`subagent_type: general-purpose`). See `references/agents.md` for the full prompt spec. Inputs: brain snapshot, auto-memory path, CLAUDE.md path.

Audits brain notes, CLAUDE.md, and auto-memory for staleness, redundancy, low-value content, verbosity, and orphans. Returns a categorized report.

**Early-exit gate:** If the auditor finds fewer than 3 actionable items, skip step 3 and go directly to step 4.

### 3. Reviewer (after auditor completes)

Spawn one `Agent` (`subagent_type: general-purpose`). See `references/agents.md` for the full prompt spec. Inputs: brain snapshot, skills snapshot, auditor report, and the principles in `.claude/skills/principle-*/SKILL.md` (in HEE v4 they are pstack skills, not a `brain/principles.md`; see `.claude/skills/pstack/PROVENANCE.md`). Also the curator's failure-mode table ([playbook.md](/mnt/storage-10tb/workflow-curator/data/playbook.md)): a family with n of 5 or more and no brain note is distillation evidence.

Combines three concerns in a single pass:
- **Synthesis**: Proposes missing wikilinks, flags principle tensions, suggests clarifications.
- **Distillation**: Identifies recurring patterns that reveal unstated principles. New principles must be (1) independent, (2) evidenced by 2+ notes, (3) actionable.
- **Skill review**: Cross-references skills against brain principles. Finds contradictions, missed structural enforcement, redundant instructions.

### 4. Review reports

Present the user with a consolidated summary. See `references/agents.md` for the report format.

### 5. Route skill-specific learnings

Check all reports for findings that belong in skill files, not `brain/`. Update the skill's SKILL.md or references/ directly. Read the skill first to avoid duplication.

### 6. Apply changes

Apply all changes directly. The user reviews the diff.

- **Outdated notes**: Update or delete
- **Redundant notes**: Merge into the stronger note, delete the weaker
- **Low-value notes**: Delete
- **Verbose notes**: Condense in place
- **New connections**: Add `[[wikilinks]]`
- **Tensions**: Reword to clarify boundaries
- **New principles**: Only from the distillation section, only if genuinely independent. Propose each as a new `principle-*` skill (a skill edit, outside `brain/`'s one writer); record the evidence notes in `brain/`
- **Merge principles**: Look for principles that are subsets or specific applications of each other — merge the narrower into the broader
- **CLAUDE.md issues**: Rewrite or delete
- **Stale memories**: Delete or rewrite

### 7. Housekeep

Update `brain/index.md` for any files added or removed.

## Summary

```
## Meditate Summary
- Pruned: [N notes deleted, M condensed, K merged]
- Extracted: [N new principles, with one-line + evidence count each]
- Skill review: [N findings, M applied]
- Housekeep: [state files cleaned]
```
