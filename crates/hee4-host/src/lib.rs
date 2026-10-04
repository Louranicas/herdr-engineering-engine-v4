//! hee4-host: the host doors the skeleton needs (spawn behind a permit, a clock,
//! a local-model client). No policy lives here: callers say what may run.
//!
//! See `modules/hee4-host/{spawn,clock}/MODULE.md` and `FLOW.md`.

pub mod clock;
pub mod model;
pub mod spawn;
