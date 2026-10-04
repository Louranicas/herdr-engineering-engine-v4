# A shared CARGO_TARGET_DIR leaks build.rs output between subjects

`build.rs` bakes `HEE4_HEAD` from `git rev-parse HEAD`. The gate's `git archive` export has no `.git`, so it baked `unknown`; because the gate and the live tree shared `~/.cache/hee4-target`, the live `hee4` then reported `unknown` too, and the dispatcher correctly refused to run it (`abandon reason=head_sha unknown at build`). Measured 2026-10-05.

Rules: a gate builds in its own per-subject target dir (`~/.cache/hee4-gate-target/<sha12>`); any build-time fact the engine binds to (the head sha) is passed to the export as an env override (`HEE4_HEAD`) and `build.rs` declares `rerun-if-env-changed`. Rung-2 door: the dispatcher's refusal of an unknown head is what caught it; keep it.

Related: [[verification-spine]], [[pinned-lines-append-only]].
