# Rust vs Julia Comparison Test Infrastructure

Compare NIfTI outputs between the Rust `mritools-binaries` and the original Julia implementations ([ROMEO.jl](https://github.com/korbinian90/ROMEO.jl), [CLEARSWI.jl](https://github.com/korbinian90/CLEARSWI.jl), [MriResearchTools.jl](https://github.com/korbinian90/MriResearchTools.jl)) to verify 1:1 reproducibility.

## Quick Start

```bash
# Run full comparison on the small test data (default)
./test/compare/run_comparison.sh

# Compare a single tool
./test/compare/run_comparison.sh --tools romeo

# Use custom data
./test/compare/run_comparison.sh \
    --data-dir /path/to/your/data \
    --phase phase.nii \
    --mag magnitude.nii \
    --echo-times 4 8 12

# Compare with relaxed tolerance
./test/compare/run_comparison.sh --tolerance 1e-3

# Only compare NIfTI files directly (no Rust/Julia run)
julia --project=test/compare/julia test/compare/julia/compare_nifti.jl rust_output.nii julia_output.nii
julia --project=test/compare/julia test/compare/julia/compare_nifti.jl --dir rust_output_dir/ julia_output_dir/
```

## Prerequisites

### Rust
The Rust binaries are built automatically by `run_comparison.sh`. Or build manually:
```bash
cargo build --workspace --release
```

### Julia
Install [Julia](https://julialang.org/downloads/) (≥ 1.9), then install the required packages:
```bash
julia --project=test/compare/julia test/compare/julia/setup.jl
```

The Julia project (`test/compare/julia/Project.toml`) depends on:
- `ROMEO.jl` — phase unwrapping (romeo, romeo_mask)
- `CLEARSWI.jl` — susceptibility weighted imaging
- `MriResearchTools.jl` — mcpc3ds, makehomogeneous, NIfTI I/O
- `NIfTI.jl` — NIfTI file reading
- `ArgParse.jl` — CLI argument parsing
- `JSON3.jl` — JSON output for comparison results

## Files

| File | Description |
|------|-------------|
| `run_comparison.sh` | Main orchestration script — builds, runs, compares |
| `julia/compare_nifti.jl` | NIfTI file comparison utility using NIfTI.jl |
| `julia/Project.toml` | Julia package dependencies |
| `julia/run_romeo.jl` | Julia runner for ROMEO phase unwrapping |
| `julia/run_clearswi.jl` | Julia runner for CLEARSWI |
| `julia/run_mcpc3ds.jl` | Julia runner for MCPC-3D-S |
| `julia/run_makehomogeneous.jl` | Julia runner for makehomogeneous |
| `julia/run_romeo_mask.jl` | Julia runner for romeo_mask |

## Comparison Options

### `run_comparison.sh`

| Option | Default | Description |
|--------|---------|-------------|
| `--data-dir` | `test/data/small` | Input data directory |
| `--phase` | `Phase.nii` | Phase filename in data dir |
| `--mag` | `Mag.nii` | Magnitude filename in data dir |
| `--echo-times` | `1 2 3` | Echo times in ms (space-separated) |
| `--tools` | all | Comma-separated: `romeo,clearswi,mcpc3ds,makehomogeneous,romeo_mask` |
| `--tolerance` | `1e-6` | Max absolute difference for PASS |
| `--rust-bin-dir` | auto | Path to Rust release binaries |
| `--julia` | `julia` | Julia executable path |
| `--output-dir` | `/tmp/mritools_compare` | Output directory for both Rust and Julia |
| `--skip-build` | off | Skip Rust compilation |
| `--skip-rust` | off | Reuse previous Rust output |
| `--skip-julia` | off | Reuse previous Julia output |
| `--verbose` | off | Show detailed output and diff distribution |

### `compare_nifti.jl`

```bash
# Compare two files
julia --project=test/compare/julia test/compare/julia/compare_nifti.jl file1.nii file2.nii [--tolerance 1e-6] [--verbose] [--json]

# Compare all matching NIfTI files in two directories
julia --project=test/compare/julia test/compare/julia/compare_nifti.jl --dir dir1/ dir2/ [--tolerance 1e-6]
```

**Output metrics:**
- Max absolute difference
- Mean absolute difference
- RMSE and Normalized RMSE
- Pearson correlation
- Exact match count / percentage
- Within-tolerance count / percentage
- Difference distribution across thresholds

**Exit codes:** `0` = PASS, `1` = FAIL, `2` = Error

## Tool Mapping: Rust → Julia

| Rust Binary | Julia Package | Julia Function |
|-------------|---------------|----------------|
| `romeo` | ROMEO.jl | `romeo()` |
| `clearswi` | CLEARSWI.jl | `clearswi()` / `calculateSWI()` |
| `mcpc3ds` | MriResearchTools.jl | `mcpc3ds()` |
| `makehomogeneous` | MriResearchTools.jl | `makehomogeneous()` |
| `romeo_mask` | ROMEO.jl | `create_mask()` |

## Using Custom Data

To compare with your own MRI data:

```bash
./test/compare/run_comparison.sh \
    --data-dir /path/to/data \
    --phase my_phase.nii \
    --mag my_magnitude.nii \
    --echo-times 4.5 9.0 13.5
```

The data directory should contain at minimum:
- A phase NIfTI file (3D or 4D with multiple echoes)
- A magnitude NIfTI file (same dimensions as phase)

## Investigating Differences

When FAIL is reported, use detailed output to diagnose:

```bash
# Verbose comparison with diff distribution
julia --project=test/compare/julia test/compare/julia/compare_nifti.jl \
    /tmp/mritools_compare/rust/romeo/unwrapped.nii \
    /tmp/mritools_compare/julia/romeo/unwrapped.nii \
    --verbose

# JSON output for programmatic analysis
julia --project=test/compare/julia test/compare/julia/compare_nifti.jl \
    /tmp/mritools_compare/rust/romeo/unwrapped.nii \
    /tmp/mritools_compare/julia/romeo/unwrapped.nii \
    --json

# Run a single tool with verbose output for debugging
./test/compare/run_comparison.sh --tools romeo --verbose --tolerance 0
```

## Notes

- **Phase rescaling**: Both Rust and Julia scripts apply the same linear rescaling from `[min, max]` to `[-π, π]` by default. Use `--no-rescale` to disable.
- **Echo times**: The default `1 2 3` matches the test data (3 echoes). Use realistic echo times for real data.
- **Julia startup**: First Julia run will be slow due to package compilation. Subsequent runs are faster.
- **Tolerance guidance**:
  - `0` — Exact bit-for-bit match
  - `1e-10` — Numerical precision differences only
  - `1e-6` — Typical for f32/f64 mixed precision
  - `1e-3` — Algorithmic differences likely present
