//! Placeholder for ROMEO `--merge-regions` (spatial region merging after
//! unwrapping).
//!
//! Julia reference: `ROMEO.jl/src/merging.jl` — `merge_regions!`.
//! qsm-core review: v0.3.3 (commit 334c6a1), no equivalent found.
//! Review date: 2026-04-17.
//!
//! See `docs/algorithm_provenance.md#romeo-merge-regions`.
//!
//! The main binary currently warns and ignores the flag; do not wire this
//! function into the happy path until it is implemented.

#[allow(dead_code)]
pub fn merge_neighbouring_regions(
    _unwrapped: &mut [f64],
    _mask: &[u8],
    _nx: usize,
    _ny: usize,
    _nz: usize,
) {
    unimplemented!(
        "romeo::algorithms::merge_regions::merge_neighbouring_regions is not implemented — \
         see docs/algorithm_provenance.md#romeo-merge-regions"
    );
}
