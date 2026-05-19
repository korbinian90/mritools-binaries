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

### 5. Close Julia-runner flag-coverage gaps

`docs/cli_parity.md` lists every flag where the Julia runner doesn't yet wire
the CompileMRI.jl option (column "Julia runner flag" reads "not wired").
Mechanical work — add the flag to `run_*.jl` and forward to the Julia function.
Each closed flag widens harness reach.

Priority order based on what the test data exercises:
- `romeo`: `-k/--mask`, `-q/--write-quality`, `-Q/--write-quality-all` (so
  quality dumps become directly comparable)
- `clearswi`: `--qsm`, `--qsm-mask` (TGV path is currently Rust-only-tested),
  plus a `writesteps` remap layer — CLEARSWI.jl writes
  `combined_mag.nii`, `sensitivity_corrected_mag.nii`, `unwrappedphase.nii`,
  `filteredphase.nii`, `maskforphase.nii`, `swimag.nii`, `swiphase.nii`
  under its own `Options.writesteps`; the harness needs them under the
  Rust canonical basenames (`mag_combined`, `mag_corrected`,
  `phase_unwrapped`, `phase_filtered`, `phase_mask`, etc.) to do
  step-by-step comparison. Today only `swi.nii` and `mip.nii` are written
  by the runner under matching names.
- `romeo_mask`: `-q/--write-quality` (Julia runner currently writes a
  `NotComputed` marker; ROMEO.jl provides `voxelquality` plus
  `calculateweights` — emit `quality.nii` and `quality_{x,y,z}.nii` from those).

### 6. Larger test data

`test/data/small/` is 51×51×41×3 float32. Add an opt-in fixture for clinical
sizes (e.g. 256³×5) under `test/data/extra/`, gated behind
`make download-extra` or a `--data-dir` override. Don't commit the bytes —
fetch from a stable URL (OSF, Zenodo) with a checksum.

## Out of scope for now

- Implementing any of the five stub algorithms
  (`merge_regions`, `correct_regions`, `wrap_addition`,
  `temporal_uncertain`, `laplacianslice`). They have citations and
  `unimplemented!()` bodies; the main binaries warn-and-ignore.
- Adding new binaries beyond the five CompileMRI.jl tools.
- Touching `qsm-core` upstream — pin stays at `v0.3.3` / `334c6a1`.
