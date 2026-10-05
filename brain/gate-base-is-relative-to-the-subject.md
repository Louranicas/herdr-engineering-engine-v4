# The gate's base is relative to the subject, not to HEAD

**Seen 2026-10-05.** `just gate cut` at subject 65876e8 reported `ddf rc=7 refused: 0 files`
while a commit landed on main mid-run. The runner resolved `[gate].base = "HEAD~1"` lazily,
at the ddf step, against the checkout's HEAD, which by then was the new commit, so
`HEAD~1..<subject>` was empty.

**Rule.** A rev in gate config is relative to the subject sha, resolved to a sha once before
any step runs. A base equal to the subject is allowed through: the diff is empty and the diff tool refuses it by name (`deep-diff-forge` rc 7 "0 files"), so the ddf step is FAIL with the reason rather than a silent green; `tools/tests/test_gate.py::test_ddf_rc7_is_fail_with_reason` pins that. Same family as
[[compile-time-paths-break-in-cached-exports]] and [[shared-target-dir-leaks-build-rs]]: the
gate judges an export at a sha, and every input it reads must be pinned to that sha, not to
whatever the live checkout is doing.

**Also.** `tools/doctor` read its head with `git rev-parse` in the cwd; in the export there is
no `.git`, so `binary_sha` was UNMEASURED under the gate even with the unit healthy. It now
takes `HEE4_HEAD` first, the same sha `build.rs` bakes.
