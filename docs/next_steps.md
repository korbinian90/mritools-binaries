# Next steps — Rust ↔ Julia parity

State at `claude/audit-and-test-AMAUN` (head `ca66e3f` + Julia-runner fixes
on this branch):
- Workspace builds, **121 tests pass**, clippy + fmt clean, CI green on all 3 OSes.
- All 5 binaries dump canonical intermediates to `<out>/steps/`.
- Julia runners under `test/compare/julia/` all execute end-to-end against the
  pinned package versions (CLEARSWI 1.6.1, ROMEO 1.x, MriResearchTools 4.x);
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

a) **`qsm_core::utils::mcpc3ds_single_coil` vs `MriResearchTools.mcpc3ds`.**
   The new `phase_corrected.nii` step compare shows max diff 6.21 rad
   (~2π wrap) and correlation 0.884. The two implementations are doing
   MCPC-3D-S differently — most likely in the smoothing filter
   (Gaussian sigma units, padding, separable vs non-separable) or in
   how the phase offset is wrapped before subtraction. Concrete next
   move: dump the `phase_offset` map from both sides, compare; if it
   already differs by 2π globally the divergence is in the offset
   calculation, not the application. Sites to read:
   `qsm_core::utils::multi_echo::mcpc3ds_single_coil` vs
   `MriResearchTools/src/mcpc3ds.jl::mcpc3ds(image; TEs, ...)`.

b) **Echo-3 2π wraps.** The 6.32 rad max diff on echo 3 is consistent
   with one 2π wrap difference on a contiguous patch of voxels. Could
   be either the TE-ratio templating itself
   (`crates/romeo/src/main.rs:489-511`, the `n_wraps = round(diff/2π)`
   step) or the region-grow seed/order. ROMEO.jl uses
   `unwrapvoxel.(w, refvalue)` for the temporal step (line 86 of
   `unwrapping.jl`) — port that exact one-liner to Rust to rule it
   out.

c) **Residual 0.086 rad on echoes 1–2.** Sub-π so not a wrap. Likely
   numerical: f32 vs f64 internal precision, or different summation
   order in the weight calculation. Lowest priority.

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
