# Unix socket paths have a 107-byte limit; put sockets in the runtime dir

**Seen 2026-10-05.** The first cut-tier run with a live model failed one test the live tree
had passed: under the gate's per-subject target dir the door socket became
`~/.cache/hee4-gate-target/<sha12>/tmp/e2e-live/work/<task>.model.sock`, 109 bytes, and
`bind(2)` refused it (`path must be shorter than SUN_LEN`). The attempt died mid-flight as an
io error, the task ended `failed`, and nothing named the cause until the log was read.
Production's path was 77 bytes, so the deployed unit never saw it.

**Rule.** `sun_path` is 108 bytes with its NUL, so a socket path is at most 107 bytes. Serve
sockets from `$XDG_RUNTIME_DIR/<app>/`, never from a work or cache tree whose depth the
caller controls. Refuse an over-long path by name at plan time (`WorkerError::DoorPath{len,
max}`, rung 2) rather than letting bind fail inside the attempt (rung 3 at best).

**Why the gate caught it.** The gate judges a `git archive` export at a sha with its own
target dir, so its paths are deeper than the live tree's. A test that passes in place and
fails under the gate is the gate doing its job; see [[gate-base-is-relative-to-the-subject]]
and [[compile-time-paths-break-in-cached-exports]] for the other two of this family.
