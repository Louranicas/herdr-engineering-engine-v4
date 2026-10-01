---
description: Re-enter HEE v4 - read the restart note and run the restart recipe (handles its absence)
allowed-tools: Bash(hee4db recipe restart), Bash(hee4db recipes), Bash(ls -1t /var/home/Louranicas/handoffs/HEE4_*)
---
Re-enter HEE v4 work. State is a query, never a field (D-13): read what the files and the DB say now.

1. Read `~/handoffs/HEE4_RESTART.md`. If it does not exist, say `restart note: ABSENT` and read the
   newest `~/handoffs/HEE4_HANDOVER_*` instead (by mtime: `ls -1t ~/handoffs/HEE4_*`), last UPDATE first.
2. Run `hee4db recipe restart`. If it exits 2 (`verdict=USAGE`, recipe not added yet), say
   `restart recipe: ABSENT`, run `hee4db recipes` once, and use the nearest existing recipes
   (`blocks-phase P0`, `agents`, `luke-held`) instead. Never invent its output. Exit 10/20/30 is a
   verdict about the state it read (gaps / a red check / unmeasured): report which part is red.
3. Report in at most 10 lines: HOLD state (H-5), what is open for Luke, the next action the note
   names, and the verdict lines you read, each quoted with its `measured=` denominator.

Then suggest `/verify` before any edit. Do not start work the notes do not name (D-06 fence).
