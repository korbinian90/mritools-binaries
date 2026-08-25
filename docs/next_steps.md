# Next steps — Rust ↔ Julia parity

State at `claude/audit-and-test-AMAUN` (head `ca66e3f` + Julia-runner fixes
on this branch):
- Workspace builds, **121 tests pass**, clippy + fmt clean, CI green on all 3 OSes.
- All 5 binaries dump canonical intermediates to `<out>/steps/`.
- Julia runners under `test/compare/julia/` all execute end-to-end against the
  pinned package versions (CLEARSWI 1.6.1, ROMEO 1.x, MriResearchTools 3.x);
  on Julia 1.12.6 the Manifest's `1.10.11` resolve warning is harmless for this
  set of APIs.
- `docs/cli_parity.md` and `docs/algorithm_provenance.md` are the reference for
  what is implemented, what is `accepted-no-op`, and what is `semantic-diff`.
- **Parity baseline captured** —
  see `docs/algorithm_provenance.md#baseline-small-dataset` for the numbers.
- Five stub algorithms exist under `crates/{romeo,clearswi}/src/algorithms/`
  with `unimplemented!()` bodies and citations — intentionally not ported.

## Environment sanity check (run first in the Julia container)

```bash
julia --version                                    # 1.10.x or 1.12.x both work
julia --project=test/compare/julia -e 'using ROMEO, CLEARSWI, MriResearchTools, NIfTI; println("ok")'
cargo test --workspace --release                   # 121 tests, all pass
test/compare/run_comparison.sh --tolerance 1e-4    # full Rust↔Julia compare
```

If the last command surfaces failures, drill into the first diverging
intermediate using the loop documented in `test/compare/README.md`
("Investigating Differences" section).

## Recommended order of work

### 1. ~~Establish a Rust↔Julia parity baseline~~ — done

The baseline lives under
`docs/algorithm_provenance.md#baseline-small-dataset`. Headlines:

- `romeo phase_rescaled` matches at 1e-4 (correlation 1.000000).
- `romeo unwrapped` and `B0` diverge by ~2π wraps with correlation 0.9999;
  first echo already has a 0.18-rad bias — so the divergence is not just
  TE-ratio templating but starts inside the per-echo unwrap path.
- `clearswi swi` correlation 0.63 (largest divergence, three semantic-diffs
  layered on top of each other).
- `mcpc3ds output` correlation 0.991, ~2π offsets on a subset of voxels.
- `makehomogeneous homogeneous` correlation 0.99991, sub-2% max diff.
- `romeo_mask mask` 85% binary agreement; correlation 0 by construction
  (binary vs binary). Otsu port should move this materially above 85%.

### 2. Close the easy `semantic-diff` items

These are documented in `docs/cli_parity.md#known-semantic-differences`:

a) ~~**`robust_mask` Otsu port.**~~ — **done**, with a correction:
   the Julia `MriResearchTools.robustmask` is **not** Otsu, it's a
   quantile rule (`high_intensity = mean(w[q80..q99])`, `noise =
   mean(w[w<=q15])` with a `q05` fallback,
   `threshold = max(5*noise, high_intensity/5)`) plus morphological
   cleanup (smooth-and-threshold at 0.4, hole-fill,
   second smooth-and-threshold at 0.6). All four call sites
   (`clearswi`, `mcpc3ds`, `romeo`, `romeo_mask`) now route through
   `mritools_common::robust_mask` (re-export of
   `qsm_core::utils::robust_mask`, which is already a port of the
   Julia algorithm). The synthetic `test/data/small` dataset has no
   noise corner so both algorithms produce the same all-ones mask
   on it — see baseline section in `docs/algorithm_provenance.md`.
   The win shows up on clinical data with a real background.

b) ~~**TGV-QSM hyperparameters.**~~ — **done.**
   `clearswi` now accepts `--tgv-iterations` (default 800),
   `--tgv-alpha-1` (default 0.003), `--tgv-alpha-0` (default 0.002),
   `--tgv-erosions` (default 0), and `--b0-direction <x y z>`
   (default `0 0 1`, normalised on parse). Defaults preserve the
   previous hard-coded behaviour, so existing pipelines are unaffected.
   Note: CLEARSWI.jl's CLI doesn't expose these (they live in MRIQSM.jl
   one layer down), so this is Rust-only configurability rather than
   a parity item — the Julia comparison harness can't exercise the
   non-default values.

### 3. ~~Wire the comparison harness into CI~~ — done (informational)

`.github/workflows/ci.yml` now has a `compare-julia` job (ubuntu-only)
that installs Julia 1.10, instantiates the env, builds the Rust release
binaries, runs `test/compare/run_comparison.sh --tolerance 1e-4`, and
uploads the comparison log and per-tool output trees as an artifact
`mritools-compare-<sha>` (14-day retention).

The job is marked `continue-on-error: true` so it does **not** gate PRs
today — the small-dataset baseline has documented divergences
(`docs/algorithm_provenance.md#baseline-small-dataset`). Its role is
drift detection: when a parity item closes (item 4 below, for example),
tighten the gate by removing `continue-on-error` and (if needed)
narrowing the tolerance.

### 4. Investigate the unwrap / B0 divergence — partial

Step 1 done: `run_romeo.jl` was missing the MCPC-3D-S phase offset
correction. `ROMEO.unwrap!` does NOT apply it internally; only the
CompileMRI.jl / RomeoApp wrapper does, which the Rust port mirrors.
After wiring `mcpc3ds(phase, mag; TEs)` into the Julia runner the
unwrapped-echo-1 bias halved (0.176 → 0.086 rad).

What's left, in order of size:

a) **`qsm_core::utils::mcpc3ds_single_coil` vs `MriResearchTools.mcpc3ds`
   — root cause identified, fix lives upstream in qsm-core.**

   Two concrete implementation differences:

   1. **Smoothing kernel.** `qsm_core::utils::gaussian_smooth_3d_phase`
      applies a truncated true-Gaussian kernel (separable convolution)
      with mask-aware reweighting. MriResearchTools.gaussiansmooth3d_phase
      applies 3 passes of a box filter, with box sizes computed to
      approximate the Gaussian (Wells's method, `getboxsizes` in
      `smoothing.jl`). These produce subtly different smoothed phase
      offsets — sub-radian locally, but enough to flip 2π wraps in
      `wrap_to_pi(phase[e] - phase_offset_smoothed)` for any voxel
      where that subtraction lands near ±π.

   2. **Seed selection for the HIP unwrap.** Rust's `find_seed_point`
      picks the centre-of-mass of the input mask. ROMEO.jl's
      `findseed!` walks a priority queue ordered by ROMEO weight
      (highest-quality voxel first). On data with regional wrap
      structure the seeds can be in different connected components,
      producing different regional 2π offsets in `unwrapped_hip`.

   Per-echo `phase_corrected` diff on `test/data/small`:
   ```
   echo 1: max 6.15 rad, mean 0.15, voxels with |d|>π: 239
   echo 2: max 6.19 rad, mean 0.27, voxels with |d|>π: 2359
   echo 3: max 6.21 rad, mean 0.26, voxels with |d|>π: 2148
   ```
   The 2.2% of voxels with |d|>π are the wrap-boundary set.

   Concrete next move: either offer a box-filter mode on
   `qsm_core::utils::gaussian_smooth_3d_phase` (cheap — qsm-core
   already has `gaussian_smooth_3d_boxsizes`; just expose a
   `gaussian_smooth_3d_phase_boxsizes` variant), or port Julia's
   `getseedfunction` / `findseed!` priority-queue seed selection
   into qsm-core. Picking either fix would close items 4a and 4c at
   once. Both fixes are out of scope for `mritools-binaries`.

b) **Echo-3 2π wraps in `unwrapped_echo_3.nii`** — local fixes
   applied, residual cause is (a).

   Two issues in `crates/romeo/src/main.rs` were fixed this session:

   1. **Wrong chain reference.** The old code used the *template*
      echo's unwrapped phase as the reference for *every* non-template
      echo (`unwrapped_volumes[template_idx][i] * te_ratio` with
      `te_ratio = tes[e] / tes[template_idx]`). ROMEO.jl propagates
      one echo at a time: echo `e`'s reference is its neighbour
      toward the template (`iref = e±1`). For echo 3 with template 1,
      Rust used ratio 3.0 (against echo 1), Julia uses ratio 1.5
      (against echo 2). Different ratios → different
      `round(diff/2π)` outcomes on wrap-boundary voxels.

   2. **Half-tie rounding rule.** Rust's `f64::round()` rounds half
      away from zero; Julia's default `round(::Float64)` uses
      `RoundNearest` (half to even). For `diff/(2π)` exactly on a
      half-integer the two flip one 2π wrap. Switched to
      `round_ties_even` (stable since Rust 1.77).

   Both fixes are merged. Neither moves the needle on
   `test/data/small` because the dominant echo-3 divergence is the
   wrap-boundary set inherited from `phase_corrected` (item a) — the
   temporal unwrap re-wraps those voxels but `expected` shifts
   correspondingly, so the integer `round()` lands on the same
   side and 2π propagates. On data without that mcpc3ds drift the
   chain + banker's-rounding fixes are net improvements.

c) **Residual 0.086 rad on echoes 1–2** — same source as (a).
   The smoothing-kernel difference produces a sub-π per-voxel error
   in `phase_offset_smoothed` that propagates uniformly into each
   echo's `wrap_to_pi(phase[e] - phase_offset_smoothed)`. Same fix
   path: box-filter smoothing mode in qsm-core.

### 5. Close Julia-runner flag-coverage gaps — mostly done

What was wired this session:
- `clearswi`: `--qsm` (`Options.qsm = true`), `--qsm-mask`
  (`Options.qsm_mask` parsed from a NIfTI file), plus a writesteps
  rename layer that maps CLEARSWI.jl's `combined_mag`,
  `sensitivity_corrected_mag`, `maskforphase`, `unwrappedphase`,
  `combinedphase` to the Rust canonical names (`mag_combined`,
  `mag_corrected`, `phase_mask`, `phase_unwrapped`, `phase_combined`).
  `swimag`, `swiphase`, and `filteredphase` stay under the Julia
  names — they don't have direct Rust counterparts.
- `romeo`: `quality.nii` step dump derived from
  `MriResearchTools.voxelquality(phase; mag, TEs)`. Compares directly
  against the Rust `quality.nii`.
- `romeo_mask`: `quality.nii` step dump replaces the `NotComputed`
  marker. `--write-quality` writes `<output>_quality.nii` next to the
  primary mask.

What's still on the table (lower-impact, mechanical):
- `romeo`: `-k/--mask` (so `qualitymask <threshold>` and
  `<file>.nii` masks can be compared), `-Q/--write-quality-all`
  (CLEARSWI.jl-style per-direction quality maps — would need to
  call `calculateweights` directly and dump three volumes).
- `clearswi`: `--qsm-input` (pre-computed χ map bypassing the
  TGV step — currently the Rust path only, easy to add to Julia
  via `Options.qsm = false` and a side-load).

What is **not** worth wiring:
- `clearswi/steps/phase_rescaled.nii` — CLEARSWI.jl doesn't dump an
  equivalent step; the rescale is folded into `unwrappedphase`.
- `mcpc3ds/steps/phase_offset.nii` — `MriResearchTools.mcpc3ds`
  doesn't return the offset map separately.
- `romeo/steps/quality_{x,y,z}.nii` — `voxelquality` already
  reduces the three directional weights into a single map.

### 6. Larger test data — scaffold done, dataset selection open

`test/data/extra/` now contains a fetch scaffold:
- `fetch.sh` reads a TSV manifest of `(path, url, sha256)` tuples,
  downloads each entry (curl or wget), and verifies the sha256.
  Re-runs skip files whose checksum already matches; mismatches delete
  the file and exit non-zero.
- `.gitignore` excludes `*.nii` and `*.nii.gz` so the bytes are never
  committed.
- `README.md` documents how to author a manifest, how to drive the
  harness against the fetched data (`run_comparison.sh --data-dir
  test/data/extra ...`), and what dataset properties actually expose
  the documented divergences (noise corner, ≥3 echoes, realistic
  dynamic range).

What's still open: choosing the dataset itself. The fetch script is
intentionally URL-agnostic because the right choice depends on the
licence the project wants to accept and on the host's long-term
availability. Likely candidates: a multi-echo GRE from BIDS Open
Datasets (ds003523, ds002785, ds001785) or one of the QSM Reconstruction
Challenge fixtures on Zenodo. Once one is picked, add a default
`manifest.tsv` and the harness can be exercised against it both
locally and (optionally) on the CI matrix.

## Out of scope for now

- Implementing any of the five stub algorithms
  (`merge_regions`, `correct_regions`, `wrap_addition`,
  `temporal_uncertain`, `laplacianslice`). They have citations and
  `unimplemented!()` bodies; the main binaries warn-and-ignore.
- Adding new binaries beyond the five CompileMRI.jl tools.
- Touching `qsm-core` upstream — pin stays at `v0.3.3` / `334c6a1`.
