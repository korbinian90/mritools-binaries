//! Placeholder for CLEAR-SWI `--unwrapping-algorithm laplacianslice`
//! (slice-wise Laplacian phase unwrapping — 2D Laplacian applied to each
//! axial slice, then stacked).
//!
//! Julia reference: `CLEARSWI.jl/src/utility.jl` — `laplacianunwrap` with
//! per-slice application; see also `MriResearchTools.laplacianunwrap`
//! with 2D kernel.
//! qsm-core review: v0.3.3 (commit 334c6a1) exposes
//! `qsm_core::unwrap::laplacian::laplacian_unwrap` (3D) but no 2D / slice
//! variant.
//! Review date: 2026-04-17.
//!
//! See `docs/algorithm_provenance.md#clearswi-laplacianslice`.
//!
//! The main binary currently prints a warning and falls back to 3D
//! Laplacian unwrap; do not wire this function into the happy path until
//! it is implemented.

#[allow(dead_code)]
pub fn laplacian_unwrap_per_slice(
    _phase: &[f64],
    _mask: &[u8],
    _nx: usize,
    _ny: usize,
    _nz: usize,
    _vsx: f64,
    _vsy: f64,
) -> Vec<f64> {
    unimplemented!(
        "clearswi::algorithms::laplacianslice::laplacian_unwrap_per_slice is not implemented — \
         see docs/algorithm_provenance.md#clearswi-laplacianslice"
    );
}
