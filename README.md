# mritools-binaries

(Warning: fully vibe-coded CLI support using [QSM.rs](https://github.com/astewartau/QSM.rs) with the [CompileMRI.jl](https://github.com/korbinian90/CompileMRI.jl) CLI API)

[![CI](https://github.com/korbinian90/mritools-binaries/actions/workflows/ci.yml/badge.svg)](https://github.com/korbinian90/mritools-binaries/actions/workflows/ci.yml)

Lightweight Rust CLI binaries for MRI processing — Rust ports of the Julia tools
from [korbinian90/CompileMRI.jl](https://github.com/korbinian90/CompileMRI.jl) (v4.7.1).

The binaries aim to closely follow the Julia CLI interfaces so they can be used as
drop-in replacements in most existing pipelines, but there may still be minor differences.

## Binaries

| Binary | Description | Status |
|---|---|---|
| `romeo` | ROMEO phase unwrapping | ✅ Implemented via [QSM.rs](https://github.com/astewartau/QSM.rs) |
| `clearswi` | CLEAR-SWI susceptibility weighted imaging | ✅ Implemented via [QSM.rs](https://github.com/astewartau/QSM.rs) |
| `mcpc3ds` | MCPC-3D-S multi-channel phase combination | ✅ Implemented via [QSM.rs](https://github.com/astewartau/QSM.rs) |
| `makehomogeneous` | Homogeneity correction for high-field MRI | ✅ Implemented via [QSM.rs](https://github.com/astewartau/QSM.rs) |
| `romeo_mask` | ROMEO quality-based brain masking | ✅ Implemented via [QSM.rs](https://github.com/astewartau/QSM.rs) |

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
