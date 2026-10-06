# Parallel slices and test runs share no path and no process

**Seen 2026-10-05/06.** These parallel runs broke each other through a name they
shared:
- task-family-actions: an e2e test used a fixed `/dev/shm` path and `pkill -KILL -f` on it.
  Concurrent runs deleted each other's dirs and killed each other's servers (1 of 3 trials
  failed).
- h5-tools-tests-runner, wave 12: `test_check_deployed` shared the `~/.cache/hee4-host` control
  root. Commit c9a3f1b gave each control run its own root. The tool's sweep still treats a
  `cd-control-*` dir without a pid file as dead, so a concurrent control can lose its dir
  between `mkdtemp` and the pid write (`tools/check-deployed:692-706`).
- Same slice: the `tools/gate` log dir is chosen by `exists()` then `makedirs()`
  (`tools/gate:103-107`, still on main at 89156bd). The refuter measured 7 of 60 concurrent
  same-sha runs failing with `FileExistsError`. The nonce in commit fb66249 hides this from the
  suite only. Out of SCOPE for that slice, so it is open.
- cut-recipe-watch-prune and c5-cut-apparatus-gaps: `prune --apply` removed a target dir that a
  concurrent build was using (a TOCTOU between plan and rmtree).
- h5-k0-redir-echo: a shared `CARGO_TARGET_DIR` was poisoned across mutation copies
  ([[shared-target-dir-leaks-build-rs]]).

**Rule.** Name every concurrent actor before writing a path or killing a process. Each slice,
test and run creates its own root (`mkdtemp`, a per-slice `~/.cache/<tool>-<slice>`), and it
kills only the pids it started. A shared name that must exist is created atomically
(`makedirs(exist_ok=False)` in a retry loop, `O_EXCL`, rename), never by check-then-create. The
ACCEPTANCE for anything that touches a shared root includes a concurrent round.

- Failure family: `concurrency_race` (n=60). MEASURED:
  `workflow-curator q "SELECT n FROM failure_modes WHERE family='concurrency_race'"`. The family
  regex also matches "block" and "clock". Re-run with `\block` over `wf_gaps` on 2026-10-06
  (43 rows, excluding `brief:verify`), every row read: 26 are real races, and at least 13 of
  those are a shared path or process as above. Classification INFERRED (by reading).
- Example key: `wf_f19f80fc-69a/journal.jsonl#key=v2:a967e9e4…&origin=refuter&idx=3` (slice
  h5-tools-tests-runner, the gate log-dir TOCTOU).
- Rung: checked, by concurrent rounds in ACCEPTANCE. A per-run root type would raise it to
  impossible.

Related: [[tmp-is-ram-keep-builds-in-cache]], [[tests-run-in-a-temp-home]],
[principle-separate-before-serializing-shared-state](../.claude/skills/principle-separate-before-serializing-shared-state/SKILL.md).
