//! Placeholder module for ROMEO algorithms that are not yet available in
//! the `qsm-core` Rust dependency.
//!
//! Each submodule corresponds to a single CLI flag from the Julia ROMEO
//! binary that is currently accepted-but-unimplemented in this port.
//! The main binary prints a `WARNING: … NOT IMPLEMENTED` message and
//! ignores the flag; these files exist so that future implementers have
//! an obvious, per-algorithm home — not to be silently wired into the
//! happy path.
//!
//! See `docs/algorithm_provenance.md` for the mapping of each stub to its
//! Julia reference and the `qsm-core` v0.3.3 review notes.

pub mod correct_regions;
pub mod merge_regions;
pub mod temporal_uncertain;
pub mod wrap_addition;
