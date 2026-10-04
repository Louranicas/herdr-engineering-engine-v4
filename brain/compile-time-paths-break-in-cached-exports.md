# env!() paths break when a gate reuses a target dir across exports

`env!("CARGO_MANIFEST_DIR")` bakes the export's temp path into the test binary. The gate caches builds per subject sha; a later export of the same sha reuses the binary, whose baked path is gone → `NotFound` (measured 2026-10-05, `one_door` in the cut tier). Rule: read `CARGO_MANIFEST_DIR` at runtime with `std::env::var`; never `env!()` a path. Same family as [[shared-target-dir-leaks-build-rs]].
