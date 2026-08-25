# Third-party notices

These binaries are Rust ports of, and link against, third-party work. The MIT
License requires the copyright notices below to accompany all copies and
substantial portions of the software, **including compiled binary
distributions** - so this file ships with the release artefacts, not only with
the source tree.

## Ported from (MIT)

The algorithms implemented here are ports of the Julia tools bundled as
`mritools` by [CompileMRI.jl](https://github.com/korbinian90/CompileMRI.jl)
(v4.7.1). See [`docs/algorithm_provenance.md`](docs/algorithm_provenance.md)
for the per-step mapping.

- **[ROMEO.jl](https://github.com/korbinian90/ROMEO.jl)** - Copyright (c) 2019
  Korbinian Eckstein, Barbara Dymerska
- **[CLEARSWI.jl](https://github.com/korbinian90/CLEARSWI.jl)** - Copyright (c)
  2019 Korbinian Eckstein
- **[MriResearchTools.jl](https://github.com/korbinian90/MriResearchTools.jl)** -
  Copyright (c) 2019 Korbinian Eckstein
- **[CompileMRI.jl](https://github.com/korbinian90/CompileMRI.jl)** - Copyright
  (c) 2020-2022 Korbinian Eckstein

Each of the above is distributed under the MIT License, reproduced in
[`LICENSE`](LICENSE).

## Rust dependency supplying the algorithms

- **[QSM.rs](https://github.com/astewartau/QSM.rs)** (`qsm-core`, pinned to tag
  `v0.3.3`) - Alex Stewart. Most pipeline steps delegate to this crate; see the
  `qsm-core function` column in `docs/algorithm_provenance.md`. Its licence
  terms bind any distribution of these binaries and must be reproduced here
  before a release is published outside this repository.

## Patent notice

**MCPC-3D-S / ASPIRE is covered by a patent
([US10605885B2](https://patents.google.com/patent/US10605885B2/en)).** The
`mcpc3ds` binary implements this method, and `romeo` invokes it for phase-offset
correction on multi-echo input.

Per the [upstream ASPIRE repository](https://github.com/korbinian90/ASPIRE):
a licence is required for **commercial use**, and the method is not a medical
product, so it may not be used for diagnosis in humans. For scientific purposes
no licence is required and the method can be applied free of charge.

An MIT licence grants copyright permissions only - it does not grant patent
rights. Anyone distributing or using these binaries commercially needs to
resolve the patent question separately.
