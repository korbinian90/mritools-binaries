# Next steps — Rust ↔ Julia parity

State at `claude/rust-mri-port-testing-SDk8t` (PR #7, head `eefb9de`):
- Workspace builds, 121 tests pass, clippy + fmt clean, CI green on all 3 OSes.
- All 5 binaries dump canonical intermediates to `<out>/steps/`.
- Julia runners under `test/compare/julia/` all support `--writesteps`.
- `test/compare/julia/Manifest.toml` committed → `julia 1.10.11`, reproducible.
- `docs/cli_parity.md` and `docs/algorithm_provenance.md` are the reference for
  what is implemented, what is `accepted-no-op`, and what is `semantic-diff`.
- Five stub algorithms exist under `crates/{romeo,clearswi}/src/algorithms/`
  with `unimplemented!()` bodies and citations — intentionally not ported.

## Environment sanity check (run first in the Julia container)

```bash
julia --version                                    # expect 1.10.x
julia --project=test/compare/julia -e 'using ROMEO, CLEARSWI, MriResearchTools, NIfTI; println("ok")'
cargo test --workspace --release                   # 121 tests, all pass
test/compare/run_comparison.sh --tolerance 1e-4    # full Rust↔Julia compare
```

If the last command surfaces failures, drill into the first diverging
intermediate using the loop documented in `test/compare/README.md`
("Investigating Differences" section).

## Recommended order of work

### 1. Establish a Rust↔Julia parity baseline

Run the full comparison harness on `test/data/small/` and record, per tool, the
first intermediate that diverges and by how much. Expected from the previous
session: `phase_rescaled` matches at 1e-4, `unwrapped`/`b0` diverge by ~2π wraps
(correlation 0.9999). Capture the numbers in a one-shot table — they're the
baseline every later change is measured against.

Suggested location: append a "Baseline (small dataset)" section to
`docs/algorithm_provenance.md`.

### 2. Close the easy `semantic-diff` items

These are documented in `docs/cli_parity.md#known-semantic-differences`:

a) **`robust_mask` Otsu port.** Three sites currently use a fixed
   10%-of-max threshold (`clearswi/main.rs:779-788`,
   `mcpc3ds/main.rs:331-340`, `romeo_mask/main.rs:360-369`). Julia's
   `MriResearchTools.robustmask` uses Otsu on the magnitude histogram.
   Port it once into `mritools_common::mask::otsu_threshold` and call from
   all three sites. Expect a measurable jump in parity on masked outputs.

b) **TGV-QSM hyperparameters.** `crates/clearswi/src/main.rs:429-462`
   hard-codes `iter=800`, `b0_dir=(0,0,1)`. Expose `--tgv-iterations`,
   `--tgv-alpha-{0,1}`, `--b0-direction <x y z>` flags; default to the
   current values to keep behaviour unchanged. CompileMRI.jl exposes
   equivalents — mirror their names.

### 3. Wire the comparison harness into CI

Add an `ubuntu-latest` matrix entry to `.github/workflows/ci.yml`:

```yaml
- name: Install Julia
  uses: julia-actions/setup-julia@v2
  with:
    version: '1.10'
- name: Instantiate Julia env
  run: julia --project=test/compare/julia test/compare/julia/setup.jl
- name: Rust↔Julia comparison
  run: test/compare/run_comparison.sh --tolerance 1e-4
```

Gate it on `cargo test` passing so failures are easy to attribute. The small
dataset finishes in well under 5 min once Julia is cached.

### 4. Investigate the unwrap / B0 divergence

`romeo/main.rs:444-471` (TE-ratio temporal unwrap, marked local-algorithm in
`docs/algorithm_provenance.md`) is the most likely site. Reproduce by:

```bash
test/compare/run_comparison.sh --tools romeo --tolerance 1e-6 --verbose
julia --project=test/compare/julia test/compare/julia/compare_nifti.jl --verbose \
    /tmp/mritools_compare/rust/romeo/steps/unwrapped_echo_1.nii \
    /tmp/mritools_compare/julia/romeo/steps/unwrapped_echo_1.nii
```

The first echo should match closely (no temporal step yet); divergence appears
on echo 2+ if the TE-ratio implementation differs. Compare against
`ROMEO.jl/src/unwrapping.jl` directly.

### 5. Close Julia-runner flag-coverage gaps

`docs/cli_parity.md` lists every flag where the Julia runner doesn't yet wire
the CompileMRI.jl option (column "Julia runner flag" reads "not wired").
Mechanical work — add the flag to `run_*.jl` and forward to the Julia function.
Each closed flag widens harness reach.

Priority order based on what the test data exercises:
- `romeo`: `-k/--mask`, `-q/--write-quality`, `-Q/--write-quality-all` (so
  quality dumps become directly comparable)
- `clearswi`: `--qsm`, `--qsm-mask` (TGV path is currently Rust-only-tested)
- `romeo_mask`: `-q/--write-quality` (Julia runner currently writes a
  `NotComputed` marker)

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
