# mritools-binaries - issue list

Working notes from the architecture review of 2026-08-19..21. Uncommitted on
purpose. Cross-repository items are in `issues-stack.md` (X0..X7). Full
evidence: https://claude.ai/code/artifact/1dcb7a4a-4523-46fc-af23-7c5b0ecc3688

Measured on this machine unless marked *unverified*. State refers to branch
`claude/julia-repos-architecture-review-tj1zzz`. Suite here: 125/125,
`clippy -D warnings` and `fmt` clean, 3m40.

A genuinely well-organised Rust workspace. The problem is not the engineering,
it is the gap between what the documents claim and what the numbers show.

---

## Done on this branch

- **F5** README no longer describes the binaries as drop-in replacements with
  "minor differences". The parity numbers are now measured here rather than
  quoted from `docs/next_steps.md`, and two new intermediates were added that
  localise the divergence.
- **F8** every Julia path cited by `docs/algorithm_provenance.md` now resolves
  to a real file, checked mechanically against the working trees. Nine or more
  were wrong (`CLEARSWI/src/{swi,mip,magnitude,phase,qsm,unwrapping}.jl`,
  `MriResearchTools/src/{mask,calculateB0}.jl`, `ROMEO/src/grow.jl`; the real
  files are `functions.jl`, `magnitude_processing.jl`, `masking.jl`,
  `algorithm.jl`). The phantom "MriResearchTools 4.x", which has never existed,
  is gone from `next_steps.md`. A traceability document that cannot be followed
  is worse than none, because it reads as verified.
- **F12** `NOTICE.md` added and packaged into the release archives and the
  Docker image, crediting the upstream MIT packages and `qsm-core` from
  astewartau/QSM.rs. This is what MIT actually requires.
- **P6** `crates/common/src/provenance.rs`: a `Method` enum plus a `Provenance`
  builder mirroring the Julia design, with citations selected by execution path
  and the same ASPIRE patent notice. Records state that this is the port and is
  not yet numerically equivalent, and point at the parity table: a provenance
  record that does not say which implementation produced the numbers is not much
  of a record. The old one-line `save_settings` is removed rather than left as a
  weaker alternative.

## Measured parity, against Julia 1.11.5

`test/data/small`, echo times `1 2 3`, tolerance 1e-4.

| Output | r | Reading |
|---|---|---|
| `clearswi` swi | 0.634 | A different image |
| `clearswi` mip | 0.675 | Inherits the SWI divergence |
| `clearswi` mag_combined | 1.000 | Magnitude path is fine ... |
| `clearswi` sensitivity | 0.949 | ... so the phase path is where SWI goes wrong |
| `romeo` unwrapped | 0.99997 | but max difference 6.31 rad, a whole wrap |
| `romeo` B0 | 0.99994 | same, max difference 130.6 Hz |
| `romeo` quality | 0.656 | the weakest link |
| `romeo_mask` quality | 0.708 | mask is a threshold on this, so 85% binary agreement is a symptom |
| `mcpc3ds` | 0.991 | 2pi offsets on a voxel subset |
| `makehomogeneous` | 0.9999 | genuinely close |

Caveat on all of it: `1 2 3` is the harness default and is not physically
realistic. Another reason X3 matters.

## Open

### B1. The quality map is the lever
r = 0.656 on `romeo` quality and 0.708 on `romeo_mask` quality are the lowest
numbers in the table and everything mask-shaped downstream inherits them. Fix
the quality map and the 85% mask agreement should follow. Start here, not at the
SWI end.

### B2. `clearswi phase_unwrapped` cannot be compared at all
The Julia and Rust `steps/` dumps disagree on its shape, so the largest
divergence in the table cannot currently be drilled into. Blocked on X4, the
`writesteps` contract.

### B3. The parity job is not a gate
`.github/workflows/ci.yml:96` carries `continue-on-error: true` and is
documented as informational. Once X3 exists, make it a gate with a per-output
threshold, so the 0.63 is a red build rather than a footnote.

### B4. F12 remainder - the LICENSE holder is a placeholder
`LICENSE` reads "Copyright (c) 2024 mritools-binaries contributors". Left for
the owner to set to a real name.

### B5. F11 - a fourth copy of `test/data/small`
Fold into the conformance artefact (X3).

### B6. The strategic question (X7.4)
What the port is for: retire, keep explicitly experimental, or promote to the
product runtime. Only answerable after X3. Note that `juliac --trim=safe`
reached 2.29 MB and 39 ms on the ROMEO kernel, which is the same class as the
1.5 MB Rust `romeo`, so the size-and-startup argument for a second
implementation is weaker than it was.
