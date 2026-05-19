//! Placeholder for ROMEO `--correct-regions` (per-region median → 0 shift
//! applied after spatial region merging).
//!
//! Julia reference: `ROMEO.jl/src/merging.jl` — `correct_regions!`.
//! qsm-core review: v0.3.3 (commit 334c6a1), no equivalent found.
//! Review date: 2026-04-17.
//!
//! See `docs/algorithm_provenance.md#romeo-correct-regions`.
//!
//! The main binary currently warns and ignores the flag; do not wire this
//! function into the happy path until it is implemented.

#[allow(dead_code)]
pub fn correct_regions_to_zero_median(
    _unwrapped: &mut [f64],
    _region_labels: &[u32],
    _mask: &[u8],
) {
    unimplemented!(
        "romeo::algorithms::correct_regions::correct_regions_to_zero_median is not implemented — \
         see docs/algorithm_provenance.md#romeo-correct-regions"
    );
}
