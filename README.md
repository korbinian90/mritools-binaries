# mritools-binaries

(Warning: fully vibe-coded CLI support using [QSM.rs](https://github.com/astewartau/QSM.rs) with the [CompileMRI.jl](https://github.com/korbinian90/CompileMRI.jl) CLI API)

[![CI](https://github.com/korbinian90/mritools-binaries/actions/workflows/ci.yml/badge.svg)](https://github.com/korbinian90/mritools-binaries/actions/workflows/ci.yml)

Lightweight Rust CLI binaries for MRI processing — Rust ports of the Julia tools
from [korbinian90/CompileMRI.jl](https://github.com/korbinian90/CompileMRI.jl) (v4.7.1).

The binaries follow the Julia CLI interfaces closely at the **flag** level, so most
existing command lines run unchanged.

> ### ⚠ Not yet numerically equivalent to the Julia tools
>
> **Do not treat these as drop-in replacements for the Julia binaries.** The
> cross-language comparison harness (`test/compare/`) measures the current
> agreement with the Julia reference, and for some tools it is far from 1:1 - the
> CLEAR-SWI output in particular is a visibly different image, not a rounding
> difference. Results are not interchangeable with the Julia tools, and should not
> be mixed within one study or compared against Julia-produced results.
>
> The numbers below are the measured baseline on the small test dataset. See
> [`docs/algorithm_provenance.md`](docs/algorithm_provenance.md#baseline-small-dataset)
> for how they were obtained and
> [`docs/next_steps.md`](docs/next_steps.md) for what is being done about them.

## Binaries

Status columns: **CLI** is flag coverage, **Parity** is measured numerical agreement
with the Julia reference on the small test dataset.

| Binary | Description | CLI | Parity vs Julia |
|---|---|---|---|
| `romeo` | ROMEO phase unwrapping | ✅ via [QSM.rs](https://github.com/astewartau/QSM.rs) | ⚠ r ≈ 0.9999 on unwrapped/B0, but with ~2π wrap divergence and a 0.18 rad first-echo bias |
| `clearswi` | CLEAR-SWI susceptibility weighted imaging | ✅ via [QSM.rs](https://github.com/astewartau/QSM.rs) | ❌ **r ≈ 0.63** on the SWI output - largest divergence, do not use for analysis |
| `mcpc3ds` | MCPC-3D-S multi-channel phase combination | ✅ via [QSM.rs](https://github.com/astewartau/QSM.rs) | ⚠ r ≈ 0.991, with 2π offsets on a subset of voxels |
| `makehomogeneous` | Homogeneity correction for high-field MRI | ✅ via [QSM.rs](https://github.com/astewartau/QSM.rs) | ✅ r ≈ 0.99991, sub-2% max difference |
| `romeo_mask` | ROMEO quality-based brain masking | ✅ via [QSM.rs](https://github.com/astewartau/QSM.rs) | ⚠ 85% binary agreement |

Several flags are additionally parsed but ignored (`accepted-no-op`) or behave
differently (`semantic-diff`); every one of them is listed in
[`docs/cli_parity.md`](docs/cli_parity.md).

The parity harness runs in CI on every push, but is **not currently a merge gate** -
today's baseline has documented divergences, so it reports rather than blocks. Its
job is to catch regressions until absolute parity is closed.

## Installation

### Download pre-built binaries

Download the latest release for your platform from the
[Releases](https://github.com/korbinian90/mritools-binaries/releases) page.

### Build from source

Requires [Rust](https://rustup.rs/) ≥ 1.70.

```bash
git clone https://github.com/korbinian90/mritools-binaries.git
cd mritools-binaries
cargo build --release
# Binaries land in ./target/release/
```

### Docker

```bash
docker pull ghcr.io/korbinian90/mritools-binaries:latest
docker run --rm -v /path/to/data:/data ghcr.io/korbinian90/mritools-binaries:latest \
    romeo -p /data/phase.nii -m /data/mag.nii -o /data/unwrapped.nii
```

## Usage

### romeo

```
romeo --help
```

```
romeo -p phase.nii -m magnitude.nii -o unwrapped.nii
romeo -p phase.nii -m magnitude.nii -o unwrapped.nii -t "[1.5,3.0,4.5]"
romeo -p phase.nii -o unwrapped.nii -k robustmask
```

### clearswi

```
clearswi -m magnitude.nii -p phase.nii -o clearswi.nii -t "[1.5,3.0]"
clearswi -m magnitude.nii -p phase.nii -o clearswi.nii -t "[1.5,3.0]" --qsm
```

### mcpc3ds

```
mcpc3ds -p phase.nii -m magnitude.nii -o combined -t "[1.5,3.0]"
```

### makehomogeneous

```
makehomogeneous -m magnitude.nii -o homogenous.nii
```

### romeo_mask

```
romeo_mask -p phase.nii -m magnitude.nii -o mask.nii -f 0.15
```

## Documentation

- [`docs/cli_parity.md`](docs/cli_parity.md) — flag-by-flag Rust ↔ Julia CLI
  parity tables for every binary, with status codes and known
  behavioural divergences.
- [`docs/algorithm_provenance.md`](docs/algorithm_provenance.md) — per-binary,
  per-step mapping from the Julia source to the `qsm-core` function used and
  the Rust call site, plus the list of algorithms accepted by the CLI but not
  yet ported (see `crates/<tool>/src/algorithms/` stubs).
- [`test/compare/README.md`](test/compare/README.md) — how to run the
  cross-language comparison harness and use intermediate `--writesteps`
  dumps to drill into divergences.
- [`RELEASING.md`](RELEASING.md) — how to cut a release (one command
  via `scripts/release.sh`, with CI-side verification that
  `Cargo.toml` and the tag agree).

## Workspace structure

```
Cargo.toml               # workspace root
crates/
  common/                # shared NIfTI I/O helpers and utilities
  romeo/                 # ROMEO phase unwrapping
  clearswi/              # CLEAR-SWI SWI processing
  mcpc3ds/               # MCPC-3D-S phase combination
  makehomogeneous/       # Homogeneity correction
  romeo_mask/            # ROMEO quality-based masking
.github/
  workflows/
    ci.yml               # Build + test on push/PR
    release.yml          # Cross-platform binaries on tag push
Dockerfile
LICENSE                  # MIT
```

## References

- Dymerska, B., et al. (2021). "Phase unwrapping with a rapid opensource minimum
  spanning tree algorithm (ROMEO)." *MRM*, 85(4):2294-2308.
  https://doi.org/10.1002/mrm.28563
- Eckstein, K., et al. (2024). "CLEAR-SWI: Computational Efficient T2* Weighted Imaging."
  *Proc. ISMRM*.
- Eckstein, K., et al. (2018). "Computationally Efficient Combination of Multi-channel
  Phase Data From Multi-echo Acquisitions (ASPIRE)." *MRM*, 79:2996-3006.
  https://doi.org/10.1002/mrm.26963
- Eckstein, K., Trattnig, S., Robinson, S.D. (2019). "A Simple Homogeneity Correction
  for Neuroimaging at 7T." *Proc. ISMRM 27th Annual Meeting*.

## License

MIT — see [LICENSE](LICENSE).

This is a derivative of MIT-licensed Julia packages and depends on
[QSM.rs](https://github.com/astewartau/QSM.rs) for most of its algorithms; the
required upstream attributions are in [NOTICE.md](NOTICE.md), which must ship
with the binaries.

Note that MCPC-3D-S / ASPIRE (`mcpc3ds`, and `romeo`'s phase-offset correction)
is **patent-encumbered** ([US10605885B2](https://patents.google.com/patent/US10605885B2/en)):
free for scientific use, but commercial use requires a licence, and the method
may not be used for diagnosis in humans. See [NOTICE.md](NOTICE.md).
