---
description: Run `just verify` (every HEE v4 check, one verdict) and report the verdict line
allowed-tools: Bash(just verify)
---
Run `just verify` from the repo root with the Bash tool (it prints one `── step i/N` block per check
and ends with `verify verdict=PASS|FAIL steps=k/N head=… logs=…`).

Report, in at most 6 lines:
1. the final `verify verdict=…` line, quoted verbatim (never a paraphrase, never "looks good");
2. for each failed step: its name, `rc=`, its own verdict line, and the `log=` path;
3. whether the failing step is inside the area this session touched.

Do not re-run a failed step piecemeal and report that as the verdict: the recipe is the one verdict.
A step reported only by exit code with no printed number measured nothing (`~/CLAUDE.md` §5).
