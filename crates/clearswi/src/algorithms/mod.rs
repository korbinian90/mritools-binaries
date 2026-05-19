//! Placeholder module for CLEAR-SWI algorithms that are not yet available
//! in the `qsm-core` Rust dependency.
//!
//! Each submodule corresponds to a single CLI option value from the Julia
//! CLEARSWI binary that is currently accepted-but-unimplemented in this
//! port. The main binary silently falls back to the default (`laplacian`)
//! for the missing options; these files exist so that future implementers
//! have an obvious, per-algorithm home — not to be silently wired into
//! the happy path.
//!
//! See `docs/algorithm_provenance.md` for the mapping of each stub to its
//! Julia reference and the `qsm-core` v0.3.3 review notes.

pub mod laplacianslice;
