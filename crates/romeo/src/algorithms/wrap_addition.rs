//! Placeholder for ROMEO `--wrap-addition` (raise the phase-difference
//! tolerance used when deciding whether to unwrap a neighbouring voxel).
//!
//! Julia reference: `ROMEO.jl/src/voxelquality.jl` / `unwrapping.jl` —
//! `wrap_addition` keyword argument to `unwrap!`.
//! qsm-core review: v0.3.3 (commit 334c6a1), no equivalent found.
//! Review date: 2026-04-17.
//!
//! See `docs/algorithm_provenance.md#romeo-wrap-addition`.
//!
//! The main binary currently warns and ignores the flag; do not wire this
//! function into the happy path until it is implemented.

#[allow(dead_code)]
pub fn unwrap_with_wrap_addition(
    _phase: &mut [f64],
    _weights: &[u8],
    _mask: &mut [u8],
    _nx: usize,
    _ny: usize,
    _nz: usize,
    _wrap_addition: f64,
) {
    unimplemented!(
        "romeo::algorithms::wrap_addition::unwrap_with_wrap_addition is not implemented — \
         see docs/algorithm_provenance.md#romeo-wrap-addition"
    );
}
