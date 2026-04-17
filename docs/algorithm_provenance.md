# Algorithm provenance — Rust ↔ Julia

This document traces each pipeline step of every binary in
`mritools-binaries` to its origin. For every step it names:

1. **Julia source** — the `.jl` file and function the Rust port mirrors.
2. **`qsm-core` function** — the upstream Rust crate
   ([`astewartau/QSM.rs`](https://github.com/astewartau/QSM.rs), pinned to
   tag `v0.3.3`, commit `334c6a1`) that supplies the algorithm.
3. **Rust site** — the `crates/<tool>/src/main.rs:line-range` in this
   workspace that invokes it.
4. **Local** — `yes` means the logic lives in this workspace rather than
   in `qsm-core`; `no` means we delegate to the crate.

Every local-algorithm row is a place where Rust↔Julia parity must be
hand-verified, since `qsm-core` is not the single source of truth.

Sections are ordered so algorithm stubs (step-5) are anchored at the
end where the stub files (`crates/<tool>/src/algorithms/*.rs`) cite
them.

## Table of contents
1. [romeo](#romeo)
2. [clearswi](#clearswi)
3. [mcpc3ds](#mcpc3ds)
4. [makehomogeneous](#makehomogeneous)
5. [romeo_mask](#romeo_mask)
6. [Canonical intermediate NIfTIs (`--writesteps`)](#canonical-intermediate-niftis)
7. [Stubs — not yet implemented](#stubs)

---

## romeo

Rust binary: `crates/romeo/src/main.rs` · Julia CLI: `CompileMRI.jl/src/romeo.jl`
driving `ROMEO.unwrap` / `MriResearchTools.romeo`.

| # | Step | Julia source | qsm-core function | Rust site | Local? |
|---|---|---|---|---|---|
| 1 | Phase rescale to [-π, π] | `MriResearchTools/src/utility.jl` (`rescale`) | — | `romeo/main.rs:717-728` (`rescale_phase`) | yes (trivial) |
| 2 | Mask build (`robustmask`/`nomask`/`qualitymask`/FILE) | `MriResearchTools/src/mask.jl` (`robustmask`) | — | `romeo/main.rs:730-758` (`build_mask`) → `robust_mask` 760-770 | yes (10 %-of-max, see semantic-diff in `cli_parity.md`) |
| 3 | MCPC-3D-S phase offset correction | `MriResearchTools/src/mcpc3ds.jl` (`mcpc3ds`) | `qsm_core::utils::mcpc3ds_single_coil`, `mcpc3ds_b0_pipeline` | `romeo/main.rs:~260-330` | no |
| 4 | Bipolar correction (if requested) | `MriResearchTools/src/mcpc3ds.jl` eddy-current section | — | delegates to step 3 with `bipolar=true` arg; local fallback in `mcpc3ds/main.rs:273-315` (see mcpc3ds below) | yes |
| 5 | ROMEO weight calculation | `ROMEO.jl/src/weights.jl` (`calculateweights`) | `qsm_core::unwrap::romeo::calculate_weights_romeo`, `calculate_weights_romeo_configurable` | `romeo/main.rs:622-691` (`calculate_weights_with_config`) | no |
| 6 | Seed selection | `ROMEO.jl/src/utility.jl` (`getseed`) | — | `romeo/main.rs:772-798` (`find_seed`) | yes |
| 7 | Region-growing unwrap | `ROMEO.jl/src/grow.jl` (`growRegionUnwrap!`) | `qsm_core::region_grow::grow_region_unwrap` | `romeo/main.rs:693-715` (`unwrap_with_seeds`) | no |
| 8 | Temporal unwrap (TE-ratio) | `ROMEO.jl/src/unwrapping.jl` (`unwrap_individual` templating) | — | `romeo/main.rs:444-471` | **yes — local algorithm** |
| 9 | Quality-map flattening | `ROMEO.jl/src/utility.jl` (`getVoxelQuality`/`mean`) | — | `romeo/main.rs:800-820` (`compute_quality_map`) | **yes — local algorithm** |
| 10 | B0 fit (phase-weighted least squares) | `MriResearchTools/src/calculateB0.jl` (`calculateB0_unwrapped`) | — | `romeo/main.rs:340-375` | yes |
| 11 | NIfTI write-out | `NIfTI.jl` `niwrite` | `qsm_core::nifti_io::save_nifti` (via `mritools_common::write_nifti`) | `romeo/main.rs:527-590` | no |

Stubs (accepted-but-ignored flags): see [Stubs](#stubs).

## clearswi

Rust binary: `crates/clearswi/src/main.rs` · Julia CLI: `CompileMRI.jl/src/clearswi.jl`
driving `CLEARSWI.calculateSWI`.

| # | Step | Julia source | qsm-core function | Rust site | Local? |
|---|---|---|---|---|---|
| 1 | Magnitude combination (`SNR`/`sum-of-squares`/`average`) | `CLEARSWI.jl/src/magnitude.jl` (`combine_magnitude`) | — | `clearswi/main.rs:656-737` (`combine_magnitude`) | yes |
| 2 | Sensitivity estimation (N4-style Gaussian smoothing) | `MriResearchTools.jl/src/smoothing.jl` (`getsensitivity`) | `qsm_core::utils::get_sensitivity` | `clearswi/main.rs:~180-245` | no |
| 3 | Bias correction via sensitivity | `CLEARSWI.jl/src/magnitude.jl` (inline) | — | `clearswi/main.rs:~245-280` | yes (one divide) |
| 4 | Mask (`robustmask`) | `MriResearchTools.jl/src/mask.jl` | — | `clearswi/main.rs:789-799` (`robust_mask`) | yes (semantic-diff) |
| 5 | Phase rescale | `CLEARSWI.jl/src/phase.jl` | — | `clearswi/main.rs:776-787` (`rescale_phase`) | yes (trivial) |
| 6 | Phase unwrapping — Laplacian 3-D | `CLEARSWI.jl/src/unwrapping.jl` (`laplacian_unwrap`) | `qsm_core::unwrap::laplacian::laplacian_unwrap` | `clearswi/main.rs:376-418`, 524-535 | no |
| 6b | Phase unwrapping — ROMEO | `ROMEO.jl/src/unwrapping.jl` | `qsm_core::unwrap::romeo::calculate_weights_romeo` + `qsm_core::region_grow::grow_region_unwrap` | `clearswi/main.rs:524` → `unwrap_romeo` 739-767 | no |
| 6c | Phase unwrapping — `laplacianslice` | `CLEARSWI.jl/src/unwrapping.jl` (slice-wise) | — | `clearswi/main.rs:528-532` — **stub**, falls back to 3-D laplacian with a warning | see [Stubs](#clearswi-laplacianslice) |
| 7 | High-pass phase filter | `CLEARSWI.jl/src/phase.jl` (`high_pass_filter`) | — | `clearswi/main.rs:~420-505` | yes (local 3-D Gaussian) |
| 8 | Multi-echo QSM weighting | `CLEARSWI.jl/src/qsm.jl` (`calculateSWI` combine step) | — | `clearswi/main.rs:356-391` | **yes — local algorithm** |
| 9 | TGV-QSM inversion | — (Julia uses MRIQSM/ChiSepNet wrappers) | `qsm_core::inversion::tgv::tgv_qsm` (`TgvParams`) | `clearswi/main.rs:429-462` | no, but **hard-coded 800 iterations, `b0_dir=(0,0,1)`** — semantic-diff |
| 10 | SWI combine (phase-weighted magnitude + softplus scaling) | `CLEARSWI.jl/src/swi.jl` (`calculateSWI`) | `qsm_core::swi::{calculate_swi, softplus_scaling, PhaseScaling}` | `clearswi/main.rs:569-601` | no |
| 11 | MIP | `CLEARSWI.jl/src/mip.jl` (`mip`) | `qsm_core::swi::create_mip` | `clearswi/main.rs:640-655` | no |
| 12 | NIfTI write-out (+ `--writesteps`) | `NIfTI.jl` | `qsm_core::nifti_io` | `clearswi/main.rs:622-655` and many `write_step` calls | no |

## mcpc3ds

Rust binary: `crates/mcpc3ds/src/main.rs` · Julia CLI: `CompileMRI.jl/src/mcpc3ds.jl`
driving `MriResearchTools.mcpc3ds`.

| # | Step | Julia source | qsm-core function | Rust site | Local? |
|---|---|---|---|---|---|
| 1 | Phase rescale | `MriResearchTools.jl/src/utility.jl` | — | `mcpc3ds/main.rs:318-329` (`rescale_phase`) | yes (trivial) |
| 2 | Mask build | `MriResearchTools.jl/src/mask.jl` | — | `mcpc3ds/main.rs:331-341` (`robust_mask`) | yes (semantic-diff) |
| 3 | MCPC-3D-S single-coil | `MriResearchTools.jl/src/mcpc3ds.jl` (`mcpc3ds`) | `qsm_core::utils::mcpc3ds_single_coil` | `mcpc3ds/main.rs:205-210` | no |
| 4 | Bipolar eddy-current correction | `MriResearchTools.jl/src/mcpc3ds.jl` (bipolar branch) | — | `mcpc3ds/main.rs:273-315` (`bipolar_correction`) | **yes — local algorithm** |
| 5 | NIfTI write-out (+ `--write-phase-offsets`, `--writesteps`) | `NIfTI.jl` | `qsm_core::nifti_io` | `mcpc3ds/main.rs:~245-265` | no |

## makehomogeneous

Rust binary: `crates/makehomogeneous/src/main.rs` · Julia CLI:
`CompileMRI.jl/src/makehomogeneous.jl` driving
`MriResearchTools.makehomogeneous`.

| # | Step | Julia source | qsm-core function | Rust site | Local? |
|---|---|---|---|---|---|
| 1 | Bias field estimate + correction | `MriResearchTools.jl/src/homogeneity.jl` (`makehomogeneous`) | `qsm_core::utils::makehomogeneous` | `makehomogeneous/main.rs:106-114` (3-D), 135-143 (4-D) | no |
| 2 | Datatype conversion (output NIfTI scale) | `NIfTI.jl` header handling | — | `makehomogeneous/main.rs:175-…` (`apply_datatype_conversion`) | yes |
| 3 | NIfTI write-out | `NIfTI.jl` | `qsm_core::nifti_io` | `makehomogeneous/main.rs:128, 159` | no |

No locally-implemented algorithms.

## romeo_mask

Rust binary: `crates/romeo_mask/src/main.rs` · Julia CLI:
`CompileMRI.jl/src/romeo_mask.jl` driving `ROMEO.robustmask` or
`MriResearchTools.qualitymask`.

| # | Step | Julia source | qsm-core function | Rust site | Local? |
|---|---|---|---|---|---|
| 1 | Phase rescale | `MriResearchTools.jl/src/utility.jl` | — | `romeo_mask/main.rs:347-358` (`rescale_phase`) | yes (trivial) |
| 2 | Initial mask (`robustmask`) | `MriResearchTools.jl/src/mask.jl` | `qsm_core::utils::otsu_threshold` (optional) | `romeo_mask/main.rs:360-370` (`robust_mask`) | yes (semantic-diff — 10 % threshold, Julia uses Otsu) |
| 3 | ROMEO weight calculation | `ROMEO.jl/src/weights.jl` | `qsm_core::unwrap::romeo::{calculate_weights_romeo, calculate_weights_romeo_configurable}` | `romeo_mask/main.rs:292-344` | no |
| 4 | Quality-map flattening | `ROMEO.jl/src/utility.jl` | — | `romeo_mask/main.rs:372-392` (`compute_quality_map`) | **yes — local algorithm** |
| 5 | Threshold + write mask | `MriResearchTools.jl/src/mask.jl` (`qualitymask`) | — | `romeo_mask/main.rs:~225-260` | yes |

---

## Canonical intermediate NIfTIs

For `--writesteps <dir>` both Rust and Julia write the following file
basenames inside `<dir>/` (all `*.nii`, float64 on Julia side, mapped to
the input datatype on Rust side). `test/compare/run_comparison.sh`
recurses into these directories and diffs matching pairs.

| Tool | `steps/*.nii` basenames |
|---|---|
| romeo | `phase_rescaled`, `phase_offset`, `phase_corrected`, `b0`, `quality`, `quality_x`, `quality_y`, `quality_z`, `unwrapped_echo_<i>`, `unwrapped` |
| clearswi | `mag_combined`, `sensitivity`, `mag_corrected`, `phase_rescaled`, `phase_unwrapped`, `phase_filtered`, `phase_mask`, `qsm`, `swi_unscaled`, `swi`, `mip` |
| mcpc3ds | `input_phases`, `phase_offset`, `corrected`, `corrected_bipolar` |
| makehomogeneous | `bias_field`, `homogeneous` |
| romeo_mask | `quality`, `quality_x`, `quality_y`, `quality_z`, `mask` |

Only files that actually exist on both sides are diffed; the Julia
runners emit `NotComputed: …` markers when a step was skipped (no
magnitude supplied, single-echo data, flag not set, etc.).

---

## Stubs

Flags accepted by the Rust CLI but whose algorithm is not yet ported
from Julia. Each stub lives in its own file under
`crates/<tool>/src/algorithms/` so the path to implementing them is
obvious; the bodies are `unimplemented!()` and the main binary still
only warns and ignores the flag. Once implemented, wire the function
into `main.rs` and remove the warning there (do **not** delete the
citation-carrying docstring).

### romeo-merge-regions
- Flag: `--merge-regions`
- Julia: `ROMEO.jl/src/merging.jl` `merge_regions!`
- qsm-core v0.3.3 (334c6a1): no equivalent found.
- Review date: 2026-04-17
- Rust stub: `crates/romeo/src/algorithms/merge_regions.rs`
- Purpose: after region-grow unwrap, merge adjacent regions whose phase
  offsets are near-integer multiples of 2π to remove residual
  inter-region jumps.

### romeo-correct-regions
- Flag: `--correct-regions`
- Julia: `ROMEO.jl/src/merging.jl` `correct_regions!`
- qsm-core v0.3.3: no equivalent.
- Review date: 2026-04-17
- Rust stub: `crates/romeo/src/algorithms/correct_regions.rs`
- Purpose: shift each unwrapped region by whole 2π so its median ends up
  near 0 rad (avoids arbitrary global offsets between regions).

### romeo-wrap-addition
- Flag: `--wrap-addition`
- Julia: `ROMEO.jl/src/weights.jl` wrap-addition weighting term.
- qsm-core v0.3.3: no equivalent.
- Review date: 2026-04-17
- Rust stub: `crates/romeo/src/algorithms/wrap_addition.rs`
- Purpose: add a small penalty to the ROMEO weight graph for neighbouring
  voxels whose phase difference lies near ±π (reduces wrap-site path
  preferences). Julia default is 0 (off).

### romeo-temporal-uncertain
- Flag: `--temporal-uncertain-unwrapping`
- Julia: `ROMEO.jl/src/unwrapping.jl` — temporal uncertainty branch that
  refuses to unwrap echoes where the temporal weight is below a
  threshold.
- qsm-core v0.3.3: no equivalent.
- Review date: 2026-04-17
- Rust stub: `crates/romeo/src/algorithms/temporal_uncertain.rs`
- Purpose: mark echoes whose inter-echo unwrap confidence is below a
  threshold as NaN so downstream B0 fit rejects them.

### clearswi-laplacianslice
- Flag: `--unwrapping-algorithm laplacianslice`
- Julia: `CLEARSWI.jl/src/unwrapping.jl` slice-wise laplacian path.
- qsm-core v0.3.3: only 3-D laplacian (`unwrap::laplacian`) is
  available.
- Review date: 2026-04-17
- Rust stub: `crates/clearswi/src/algorithms/laplacianslice.rs`
- Current behaviour: falls back to 3-D laplacian unwrap with a WARNING
  print — differs from Julia, so flag this when reporting comparison
  results.
- Purpose: apply 2-D laplacian unwrap per axial slice; useful for data
  with thick slices or highly anisotropic voxel spacing where the 3-D
  laplacian blurs across slices.
