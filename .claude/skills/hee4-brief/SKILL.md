---
name: hee4-brief
description: The template for ANY subagent brief in HEE v4 - the law lines every agent needs (fence, no v3 writes, pinned dispatch, stable ids, typed verdict, claim ≠ evidence, kill by PID, sed/nullglob traps, no settings edits, commit only when asked), each with its incident id. Use before dispatching an Agent/subagent or writing a roster prompt for v4 work.
---

# HEE v4 subagent brief

A rule not in the brief is not in force for the agent (Mistakes #35). Paste the **law block**
verbatim into every brief, then fill the task block. Lessons for the task's situation: the project
skill **`hee4-lessons`** (`/lessons <situation>`); corpus navigation: the user skill **`hee-v4-corpus`**.

## Dispatch
- Description carries a model pin: `[pin:opus] <3-5 words>` (or `[pin:sonnet]`, `[pin:fable]`).
  A pinned dispatch makes no Jev router call (HEE4_HANDOVER_20261001_0900, V10 (d)); v4 text must
  never reach Jev (H-8, S-9).
- Budget the brief: ≤ 20 commands for a review or recapture (memory `recapture-review-needs-a-command-budget`, K10);
  a fan-out states `planned_agents=` first (AP-35, D-15, L19).

## Law block (paste verbatim; one line each, incident in brackets)
```
LAW 1  Fence: write only under <roots>; v4 homes are ~/herdr-engineering-engine-v4, ~/hee4-evidence, the v4 vault. [D-06, V9]
LAW 2  Never write, build, gate or deploy HEE-v3 (/var/home/herdr-engineering-engine-v3, ~/.cache/hee3-*, ~/hee3-evidence, ~/.local/lib/herdr-engineering-engine-v3); read v3 only via ~/hee4-evidence/reference/. [V4-9, V4-68]
LAW 3  HOLD: no engine code until Luke says "start coding" (H-5 open). [V4-0]
LAW 4  Cite stable ids (AP-nn, EX-nn, D-nn, DC-nn, H-n, V4-nn, Ln), never file line numbers; a KEY:line cite needs `just repin KEY` after its file changes. [V4-23, V4-25]
LAW 5  End with ONE typed line: `<task> verdict=PASS|FAIL|PASS_WITH_GAPS measured=k/n …`; no prose verdicts. [L28, S-7, AP-29]
LAW 6  The claim is not the evidence: quote the printed number and the command that printed it; relay no peer/agent claim unchecked. [AP-34, L18, Mistakes #61, S-1]
LAW 7  Kill by PID from a pid file you wrote, never by pattern (pkill -f self-matches; never touch another session's process). [AP-38, Mistakes #35/#37/#55, L21]
LAW 8  No sed with a `|` delimiter on Markdown (tables hold `|`); use python or the Edit tool. [FINAL-LESSONS-REVIEW S-5]
LAW 9  nullglob traps: a bare `ls` on an empty glob lists the cwd; a bare `grep` with no file reads stdin. Count array matches first; pass </dev/null. [HEE4_HANDOVER_20261001_0900 Traps]
LAW 10 Never edit ~/.claude/settings*, ~/.claude/hooks/, user/project settings or steering files; an apply script needs an explicit root, never a $HOME default. [memory apply-root-default-live, S-6, L20]
LAW 11 A verdict comes from the command's own exit code, never a pipe's: use `gate <cmd>` or ${PIPESTATUS[0]}. [L23, Mistakes #11/#19/#21/#44/#48]
LAW 12 Do not chain a commit after its checks; never commit or push unless the brief says so (push only on Luke's word). [L22, Mistakes #36, H-1]
LAW 13 Enumerate before any blanket command (rm -rf glob, prune, find -delete). [K13, Mistakes #20]
LAW 14 Never edit a worktree a gate or plant battery is running in; never stage mid-battery. [L8, memory never-edit-a-gated-worktree]
```

## Task block (fill in)
```
TASK     <one sentence; the deliverable and its reader>
INPUTS   <exact paths / ids; what is read-only>
WRITE    <the ONLY paths it may write, e.g. its report path>
BUDGET   <≤ N commands / ≤ M minutes; every loop has a budget (AP-31)>
SOURCES  <independent sources to check against, tagged FACT / DESIGN / PROPOSAL>
RETURN   <≤ 15-line pointer return: verdict line, report path, top findings by id>
```

## Checks before sending
- Every LAW line present? (A brief missing LAW 7 once cost another session's mutation run, #35.)
- WRITE names paths, not "the repo". RETURN names the exact verdict token the reader greps for (L28).
- Same-lineage review ends PASS_WITH_GAPS at best; say so rather than overstate it (L29).
