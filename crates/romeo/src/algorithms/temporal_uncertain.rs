//! Placeholder for ROMEO `--temporal-uncertain-unwrapping` (spatially
//! unwrap low-quality voxels after temporal unwrapping for vein-imaging
//! scenarios).
//!
//! Julia reference: `ROMEO.jl/src/unwrapping.jl` —
//! `temporal_uncertain_unwrapping` keyword argument.
//! qsm-core review: v0.3.3 (commit 334c6a1), no equivalent found.
//! Review date: 2026-04-17.
//!
//! See `docs/algorithm_provenance.md#romeo-temporal-uncertain`.
//!
//! The main binary currently warns and ignores the flag; do not wire this
//! function into the happy path until it is implemented.

#[allow(dead_code)]
pub fn temporal_uncertain_unwrap(
    _unwrapped_4d: &mut [Vec<f64>],
    _quality: &[f64],
    _mask: &[u8],
    _nx: usize,
    _ny: usize,
    _nz: usize,
    _threshold: f64,
) {
    unimplemented!(
        "romeo::algorithms::temporal_uncertain::temporal_uncertain_unwrap is not implemented — \
         see docs/algorithm_provenance.md#romeo-temporal-uncertain"
    );
}
