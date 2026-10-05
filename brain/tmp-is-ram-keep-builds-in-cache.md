# /tmp is RAM: build output goes under ~/.cache, sockets under the runtime dir

**Seen 2026-10-05.** On this host /tmp is a 48G RAM-backed tmpfs, and it filled twice in one day.
- Space: agents' Cargo target dirs in a session scratchpad under /tmp reached 12G.
- Inodes: leaked tool-test fixtures reached 12,893 dirs and 100% of /tmp's inodes. Every shell on
  the machine then failed. The tests were fixed at 4db540e: one run dir that is removed at exit.

**Rules.**
- Build output (a `CARGO_TARGET_DIR`, a gate or cold-clone export, a crash run) lives under
  `~/.cache/<tool>-<slice or sha12>`, never in /tmp or a session scratchpad. `tools/gate` and
  `tools/cold-clone` default there already; set it by hand for every agent build.
- Small throwaway fixtures may use `tempfile`, and they must be removed. `test_habitat_backup` keeps
  a /tmp hop because it needs a second device.
- Sockets are served from `$XDG_RUNTIME_DIR/<app>/`, never from a cache or work tree; see
  [[unix-socket-paths-have-a-107-byte-limit]].

**Door.** `tools/doctor` prints `check=tmp_usage` (space%, inode%, the user's three largest entries).
The row is advisory above 70%: a full /tmp is the operator's to clear, not a readiness failure.
Related: [[shared-target-dir-leaks-build-rs]].
