# Extra test data (opt-in, fetched on demand)

`test/data/small/` is a 51×51×41×3 float32 fixture, small enough to ship
in-repo. It's enough to exercise every CLI flag but doesn't show the
algorithmic divergences that only appear at clinical resolution (see
`docs/algorithm_provenance.md#baseline-small-dataset`).

This directory is the staging area for larger fixtures, fetched from a
stable URL with a sha256 checksum. Nothing here is committed — the
`.gitignore` excludes `*.nii` and `*.nii.gz`.

## How to use

1. Create a manifest file `manifest.tsv` in this directory (one entry per
   line, tab-separated):

   ```
   # Lines starting with # are skipped.
   # <relative_path>  <url>  <sha256>
   Phase.nii  https://example.org/path/Phase.nii  3a2b1c...
   Mag.nii    https://example.org/path/Mag.nii    9d8e7f...
   ```

   Stable hosts that have worked for similar projects: OSF
   (`osf.io/<token>/download`), Zenodo (`zenodo.org/record/<id>/files/<name>`),
   GitHub Releases (`github.com/<org>/<repo>/releases/download/<tag>/<file>`).

2. Run the fetcher:

   ```bash
   test/data/extra/fetch.sh
   ```

   The script downloads each entry, verifies the sha256, and refuses to
   overwrite a file whose existing hash already matches. A mismatch
   deletes the file and exits non-zero.

3. Drive the harness against the new tree:

   ```bash
   test/compare/run_comparison.sh \
       --data-dir test/data/extra \
       --phase Phase.nii \
       --mag Mag.nii \
       --echo-times "4 8 12 16"   # whatever your TEs actually are
   ```

The harness already supports `--data-dir`, `--phase`, `--mag`, and
`--echo-times` — no harness changes were needed.

## Choosing a dataset

For the divergences documented in
`docs/algorithm_provenance.md#baseline-small-dataset` to actually
manifest you need data with:

- A real noise corner (otherwise `robust_mask` produces all-ones on
  both sides — exactly what we saw on the small dataset).
- ≥3 echoes (so the multi-echo combine path in `clearswi` and the
  temporal unwrap in `romeo` get exercised).
- Realistic dynamic range (not bias-corrected and rescaled to a narrow
  band).

Any public 3T GRE acquisition with multi-echo phase + magnitude works.
Examples used in the QSM literature: BIDS Open Datasets ds003523,
ds002785, ds001785. Pick one with a permissive licence and stable
mirror, hash the files you actually downloaded, and put those into
`manifest.tsv`.
