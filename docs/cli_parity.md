# CLI parity — Rust ↔ Julia (`CompileMRI.jl` v4.7.1)

This document records, per binary, every Rust CLI flag in this workspace
and the corresponding option exposed by the Julia source the port is
tracking (`CompileMRI.jl` CLI wrappers, and the underlying packages
`ROMEO.jl`, `CLEARSWI.jl`, `MriResearchTools.jl`). The Julia-runner
column refers to the wrappers in `test/compare/julia/` that drive the
live cross-language comparison.

Status codes:
- `parity` — same flag name/semantic both sides
- `rust-only` — Rust accepts it; Julia CLI does not expose it
- `julia-only` — Julia CLI has it; Rust has no equivalent (flag missing on our runner too, flagged below)
- `accepted-no-op` — Rust parses the flag but warns and ignores (stubs live under `crates/<tool>/src/algorithms/`)
- `semantic-diff` — both sides accept it but behaviour differs (documented per row)

## Table of contents
1. [romeo](#romeo)
2. [clearswi](#clearswi)
3. [mcpc3ds](#mcpc3ds)
4. [makehomogeneous](#makehomogeneous)
5. [romeo_mask](#romeo_mask)
6. [Known semantic differences](#known-semantic-differences)

---

## romeo

Source: `crates/romeo/src/main.rs:32-139` · Julia runner: `test/compare/julia/run_romeo.jl` · Julia lib: `ROMEO.jl` via `MriResearchTools`.

| Rust flag | Default | Julia runner flag | `romeo()` kwarg | CompileMRI.jl `romeo` | Status |
|---|---|---|---|---|---|
| `-p / --phase` | (required) | `--phase / -p` | first positional | `-p` | parity |
| `-m / --magnitude` | none | `--magnitude / -m` | `mag` | `-m` | parity |
| `-o / --output` | `unwrapped.nii` | `--output / -o` | — | `-o` | parity |
| `-t / --echo-times` | none | `--echo-times / -t` | `TEs` | `-t` | parity |
| `-k / --mask <nomask\|robustmask\|qualitymask\|FILE>` | `robustmask` | not wired | — | `-k` | julia-only flag on runner — add in follow-up |
| `-u / --mask-unwrapped` | off | not wired | — | `-u` | julia-only flag on runner |
| `-e / --unwrap-echoes <range>` | `:` | not wired | — | `-e` | julia-only flag on runner |
| `-w / --weights <romeo\|romeo2…\|bestpath>` | `romeo` | `--weights / -w` | `weights` (Symbol) | `-w` | parity |
| `-B / --compute-B0 [NAME]` | off | `--compute-B0 / -B` | `MriResearchTools.calculateB0_unwrapped` | `-B` | parity (Julia always writes `B0.nii` — see semantic-diff below) |
| `--B0-phase-weighting <phase_snr\|phase_var\|average\|TEs\|mag\|simulated_mag>` | `phase_snr` | not wired | — | `--B0-phase-weighting` | julia-only flag on runner |
| `--phase-offset-correction <on\|off\|bipolar>` | `on` (if ≥2 echoes) | `--phase-offset-correction` | `phase_offset_correction` (Symbol) | `--phase-offset-correction` | parity |
| `--phase-offset-smoothing-sigma-mm <x y z>` | `[7,7,7]` | not wired | `sigma` | `--phase-offset-smoothing-sigma-mm` | julia-only flag on runner |
| `--write-phase-offsets` | off | not wired | `po` (output array) | `--write-phase-offsets` | julia-only flag on runner |
| `-i / --individual-unwrapping` | off | `--individual` | `individual` | `-i` | parity (flag name differs on runner) |
| `--template <N>` | `1` | `--template` | `template` | `--template` | parity |
| `--no-phase-rescale` / `--no-rescale` | off (rescale on) | `--no-rescale` | — (done before `romeo()`) | `--no-phase-rescale` | parity |
| `--fix-ge-phase` | off | not wired | — | `--fix-ge-phase` | julia-only flag on runner |
| `--threshold <float>` | ∞ | not wired | — | `--threshold` | julia-only flag on runner |
| `-g / --correct-global` | off | not wired | — | `-g` | julia-only flag on runner |
| `-q / --write-quality` | off | not wired | use `ROMEO.voxelquality` | `-q` | julia-only flag on runner |
| `-Q / --write-quality-all` | off | not wired | use `ROMEO.calculateweights` | `-Q` | julia-only flag on runner |
| `-s / --max-seeds <N>` | `1` | not wired | `maxseeds` | `-s` | julia-only flag on runner |
| `--merge-regions` | off | n/a | `merge_regions` | `--merge-regions` | accepted-no-op → `crates/romeo/src/algorithms/merge_regions.rs` |
| `--correct-regions` | off | n/a | `correct_regions` | `--correct-regions` | accepted-no-op → `crates/romeo/src/algorithms/correct_regions.rs` |
| `--wrap-addition <0..π>` | `0.0` | n/a | `wrap_addition` | `--wrap-addition` | accepted-no-op → `crates/romeo/src/algorithms/wrap_addition.rs` |
| `--temporal-uncertain-unwrapping [T]` | `0.0` | n/a | `temporal_uncertain_unwrapping` | `--temporal-uncertain-unwrapping` | accepted-no-op → `crates/romeo/src/algorithms/temporal_uncertain.rs` |
| `-v / --verbose` | off | — | — | — | rust-only (diagnostic) |

Unwrapping seed selection, temporal propagation via TE-ratio, and the
collapsed per-voxel quality map are Rust-local; see
`docs/algorithm_provenance.md#romeo`.

---

## clearswi

Source: `crates/clearswi/src/main.rs:32-112` · Julia runner:
`test/compare/julia/run_clearswi.jl` · Julia lib: `CLEARSWI.jl`.

| Rust flag | Default | Julia runner flag | Julia API | CompileMRI.jl `clearswi` | Status |
|---|---|---|---|---|---|
| `-m / --magnitude` | (required) | `--magnitude / -m` | `Data.mag` | `-m` | parity |
| `-p / --phase` | optional | `--phase / -p` | `Data.phase` | `-p` | parity |
| `-o / --output` | `clearswi.nii` | `--output / -o` | — | `-o` | parity |
| `-t / --echo-times` | none | `--echo-times / -t` | `Data.TEs` | `-t` | parity |
| `-s / --mip-slices <N>` | `7` | `--mip-slices / -s` | `createMIP(..., N)` | `-s` | parity |
| `--qsm` | off | not wired | `Options.phase.scaling.type = :qsm` | `--qsm` | julia-only flag on runner; Rust uses `qsm_core::inversion::tgv::tgv_qsm` |
| `--qsm-input FILE` | none | not wired | pre-computed chi map | `--qsm-input` | julia-only flag on runner |
| `--qsm-mask FILE` | none | not wired | mask for TGV | `--qsm-mask` | julia-only flag on runner |
| `--mag-combine <SNR\|average\|echo N\|SE TE>` | `SNR` | `--mag-combine` | `Options.mag.combine_echoes` | `--mag-combine` | parity (only SNR/average wired in Julia runner today) |
| `--mag-sensitivity-correction <on\|off\|FILE>` | `on` | `--mag-sensitivity-correction` | `Options.mag.correct_sensitivity` | `--mag-sensitivity-correction` | parity |
| `--mag-softplus-scaling <on\|off>` | `on` | not wired | `Options.mag.softplus` | `--mag-softplus-scaling` | julia-only flag on runner |
| `--unwrapping-algorithm <laplacian\|romeo\|laplacianslice>` | `laplacian` | `--unwrapping-algorithm` | `Options.unwrapping` | `--unwrapping-algorithm` | `laplacianslice` is accepted-no-op → `crates/clearswi/src/algorithms/laplacianslice.rs` |
| `--filter-size <x y z>` | `[4,4,0]` | `--filter-size` | `Options.phase.hp_sigma` | `--filter-size` | parity |
| `--phase-scaling-type <tanh\|negativetanh\|positive\|negative\|triangular>` | `tanh` | `--phase-scaling-type` | `Options.phase.scaling.type` | `--phase-scaling-type` | parity |
| `--phase-scaling-strength <float>` | `4` | `--phase-scaling-strength` | `Options.phase.scaling.strength` | `--phase-scaling-strength` | parity |
| `-e / --echoes <range>` | `:` | not wired | — | `-e` | julia-only flag on runner |
| `--no-phase-rescale` / `--no-rescale` | off | `--no-rescale` | — | `--no-phase-rescale` | parity |
| `--fix-ge-phase` | off | not wired | — | `--fix-ge-phase` | julia-only flag on runner |
| `--writesteps DIR` | none | not wired today — added by this port | `savesteps=...` (internal) | — | parity planned (see `docs/algorithm_provenance.md#clearswi`) |
| `--tgv-iterations <N>` | `800` | n/a | `TgvParams.iterations` | not exposed by CLEARSWI.jl | rust-only (TGV knob) |
| `--tgv-alpha-1 <float>` | `0.003` | n/a | `TgvParams.alpha1` | not exposed by CLEARSWI.jl | rust-only (TGV knob) |
| `--tgv-alpha-0 <float>` | `0.002` | n/a | `TgvParams.alpha0` | not exposed by CLEARSWI.jl | rust-only (TGV knob) |
| `--tgv-erosions <N>` | `0` | n/a | `TgvParams.erosions` | not exposed by CLEARSWI.jl | rust-only (TGV knob) |
| `--b0-direction <x y z>` | `0 0 1` | n/a | `b0_dir` (4th positional to `tgv_qsm`) | not exposed by CLEARSWI.jl | rust-only (TGV knob) |
| `-v / --verbose` | off | — | — | — | rust-only |

TGV-QSM hyperparameters (α₁, α₀, iterations, erosions, `b0_dir`) are
exposed on the Rust CLI via `--tgv-*` and `--b0-direction`. CLEARSWI.jl
itself doesn't surface these (they live one layer deeper inside
MRIQSM.jl), so the Rust port is strictly more configurable on this axis.

---

## mcpc3ds

Source: `crates/mcpc3ds/src/main.rs:22-72` · Julia runner:
`test/compare/julia/run_mcpc3ds.jl` · Julia lib:
`MriResearchTools.mcpc3ds`.

| Rust flag | Default | Julia runner flag | `mcpc3ds()` kwarg | CompileMRI.jl `mcpc3ds` | Status |
|---|---|---|---|---|---|
| `-p / --phase` | (required) | `--phase / -p` | `phase` | `-p` | parity |
| `-m / --magnitude` | optional | `--magnitude / -m` | `mag` | `-m` | parity |
| `-o / --output` | `output` | `--output / -o` | — | `-o` | parity |
| `-t / --echo-times` | none | `--echo-times / -t` | `TEs` | `-t` | parity |
| `-s / --smoothing-sigma <x y z>` | `[10,10,5]` | `--smoothing-sigma / -s` | `sigma` | `-s` | parity |
| `-b / --bipolar` | off | `--bipolar / -b` | `bipolar_correction` | `-b` | parity |
| `--write-phase-offsets` | off | `--write-phase-offsets` | `po` out arg | `--write-phase-offsets` | parity |
| `--no-phase-rescale` / `--no-rescale` | off | `--no-rescale` | — | `--no-phase-rescale` | parity |
| `--fix-ge-phase` | off | not wired | — | `--fix-ge-phase` | julia-only flag on runner |
| `--writesteps DIR` | none | added by this port | `savesteps=...` | — | parity planned |
| `-v / --verbose` | off | — | — | — | rust-only |

---

## makehomogeneous

Source: `crates/makehomogeneous/src/main.rs:18-48` · Julia runner:
`test/compare/julia/run_makehomogeneous.jl` · Julia lib:
`MriResearchTools.makehomogeneous`.

| Rust flag | Default | Julia runner flag | `makehomogeneous()` kwarg | CompileMRI.jl | Status |
|---|---|---|---|---|---|
| `-m / --magnitude` | (required) | `--magnitude / -m` | first positional | `-m` | parity |
| `-o / --output` | `homogenous` | `--output / -o` | — | `-o` | parity |
| `-s / --sigma-bias-field` | `7.0` | `--sigma / -s` | `sigma_mm` | `-s` | parity (flag name differs on runner: `--sigma` vs `--sigma-bias-field`) |
| `-n / --nbox` | `15` | `--nbox / -n` | `nbox` | `-n` | parity |
| `-d / --datatype <Float32\|Float64\|Int16\|Int32\|UInt8\|UInt16>` | same as input | not wired | — | `-d` | julia-only flag on runner; Rust emulates by clamp+round |
| `-v / --verbose` | off | — | — | — | rust-only |

---

## romeo_mask

Source: `crates/romeo_mask/src/main.rs:23-77` · Julia runner:
`test/compare/julia/run_romeo_mask.jl` · Julia lib: `ROMEO.create_mask`.

| Rust flag | Default | Julia runner flag | `create_mask()` arg | CompileMRI.jl `romeo_mask` | Status |
|---|---|---|---|---|---|
| `-p / --phase` | (required) | `--phase / -p` | `phase` | `-p` | parity |
| `-m / --magnitude` | optional | `--magnitude / -m` | — | `-m` | parity |
| `-o / --output` | `unwrapped.nii` | `--output / -o` | — | `-o` | parity |
| `-f / --factor <0..1>` | `0.1` | `--factor / -f` | `threshold` | `-f` | parity |
| `-t / --echo-times` | none | `--echo-times / -t` | — | `-t` | parity |
| `-e / --unwrap-echoes <range>` | `:` | not wired | — | `-e` | julia-only flag on runner |
| `-w / --weights <romeo\|…>` | `romeo` | `--weights / -w` | — | `-w` | parity |
| `--no-phase-rescale` / `--no-rescale` | off | `--no-rescale` | — | `--no-phase-rescale` | parity |
| `--fix-ge-phase` | off | not wired | — | `--fix-ge-phase` | julia-only flag on runner |
| `-q / --write-quality` | off | `--write-quality / -q` (API-dependent) | — | `-q` | semantic-diff (Julia runner currently no-op) |
| `-Q / --write-quality-all` | off | not wired | — | `-Q` | julia-only flag on runner |
| `-v / --verbose` | off | — | — | — | rust-only |

---

## Known semantic differences

These behaviours are identical on both Rust and Julia *runners* today,
but diverge from the Julia packages they wrap. Document-only — not
fixed in this pass.

1. ~~**Robust mask (all phase/magnitude-based masks)**~~ — **resolved.**
   The four call sites in `clearswi`, `mcpc3ds`, `romeo` and `romeo_mask`
   now go through `mritools_common::robust_mask` (re-export of
   `qsm_core::utils::robust_mask`), which is a port of
   `MriResearchTools.robustmask`: quantile-based threshold
   (`high_intensity = mean(w[q80..q99])`, `noise = mean(w[w<=q15])` with
   a `q05` fallback, `threshold = max(5*noise, high_intensity/5)`) plus
   morphological cleanup (gaussian smooth-and-threshold at 0.4, fill
   holes, second smooth-and-threshold at 0.6). Note: despite the name
   in `next_steps.md`, the Julia function is **not** Otsu — it's a
   hand-tuned quantile rule with explicit MRI-specific assumptions
   (one corner is noise-only). Otsu is still used for the
   quality-map-based threshold in `romeo_mask` (`romeo_mask/main.rs:233`)
   via `qsm_core::utils::otsu_threshold` — that's a different and
   correct application.
2. ~~**TGV-QSM parameters hard-coded**~~ — **resolved.**
   Exposed on `clearswi` as `--tgv-iterations` (default 800),
   `--tgv-alpha-1` (default 0.003), `--tgv-alpha-0` (default 0.002),
   `--tgv-erosions` (default 0), and `--b0-direction` (default `0 0 1`,
   normalised to unit length). Defaults preserve the previous
   hard-coded behaviour. Underlying `qsm_core::inversion::tgv::TgvParams`
   has more knobs (step_size, fieldstrength, tol) not yet surfaced —
   add them on demand. Note: CLEARSWI.jl itself doesn't expose these
   either (CLEARSWI's `--qsm` is a boolean and TGV params live in the
   wrapped MRIQSM.jl); the Rust port is now strictly more configurable
   than the Julia upstream on this axis.
3. **TE-ratio temporal unwrap** —
   `crates/romeo/src/main.rs:444-471` implements a local simplification
   of ROMEO.jl's temporal unwrap: each non-template echo is
   `phase - round((phase - template*TE_ratio) / 2π) * 2π`. ROMEO.jl's
   `unwrap!` uses seed-guided region growth across echoes in one pass.
4. **Multi-echo QSM combination** —
   `crates/clearswi/src/main.rs:356-391` unwraps each echo with the
   Laplacian and combines them with a `mag²·TE²` weighted average
   before feeding a single phase map to TGV. Julia CLEARSWI uses
   `MriResearchTools.romeo` + `calculateB0_unwrapped` or the
   `Options.phase.scaling.type = :qsm` path, which differ in scaling.
5. **Bipolar eddy-current correction** —
   `crates/mcpc3ds/src/main.rs:273-315` averages the odd-echo deviation
   from the mean of even neighbours and subtracts from odd echoes.
   Julia's `mcpc3ds(..., bipolar_correction=true)` uses a different
   interpolation across echoes.
6. **Quality map flattening** —
   `crates/romeo/src/main.rs:798-818` and
   `crates/romeo_mask/src/main.rs:372-392` average the three directional
   weights into a single per-voxel quality. Julia's
   `ROMEO.voxelquality` uses a geometric/product combination.
7. **`-B / --compute-B0` output filename** — Rust writes to whatever
   path the user passes (default `B0.nii`). The Julia runner currently
   always writes `B0.nii` next to the primary output; see
   `test/compare/julia/run_romeo.jl:130`. Keep this in mind when
   scripting custom `-B` names.

Additional runner coverage gaps (rows marked `julia-only flag on
runner`) are candidates for follow-up work — they do not affect
algorithm parity, only the default-exposed surface of
`run_*.jl` scripts when driving the comparison harness.
