---
description: Load the HEE v4 lessons (anti-patterns, exemplars, drift controls, spells, mistakes) for a situation
argument-hint: <situation, e.g. "store door", "test double", "a loop", "a gate", "a brief", "a migration">
---
Situation: `$ARGUMENTS` (if empty, show the situation column of the triggers table and ask).

1. Read `.claude/skills/hee4-lessons/reference/triggers.md` and pick the row(s) whose situation matches.
2. For each id that row names, read ONLY its line in the matching reference file
   (`antipatterns.md`, `exemplars.md`, `spells.md`, `mistakes.md`), then open its home
   (the AP/EX/D row in `docs/`, the L/K row in PROCESS-LEARNINGS, the diary note) for the detail.
3. Answer in ≤ 15 lines: the 3-7 lessons that bite here, each as `id - what to do - home`.

The skill is an index: never paste a home's content into the answer at length; point at it.
