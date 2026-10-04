//! `hee4-evidence` (K4): [`decide`], the one verdict authority, and the deep-diff-forge
//! observation adapter [`ddf`]. See `FLOW.md` for the lattice table.

pub mod ddf;
mod decide;

pub use decide::{
    Identities, Identity, SealError, Source, Subject, Why, decide, decide_and_seal, observation_id,
};
