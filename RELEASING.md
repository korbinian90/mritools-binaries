# Releasing

The release flow is designed so a tag and `Cargo.toml`'s workspace
version can never disagree.

## Cutting a release

From a clean `main`:

```bash
scripts/release.sh 0.3.0
```

The script:

1. Refuses to run unless you're on `main` with a clean working tree.
2. Refuses to reuse an existing tag (local or remote).
3. Bumps `workspace.package.version` in `Cargo.toml`.
4. Refreshes `Cargo.lock`.
5. Runs `cargo test --release`, `cargo clippy -- -D warnings`,
   `cargo fmt --check`. Aborts on any failure.
6. Creates a single `Release v0.3.0` commit + annotated tag `v0.3.0`.

It does **not** push. Review the diff (`git show v0.3.0`) and then:

```bash
git push origin main
git push origin v0.3.0
```

The tag push triggers `.github/workflows/release.yml`, which:

1. Runs a `verify-version` job that compares the tag (e.g. `v0.3.0`)
   against `workspace.package.version` in `Cargo.toml`. Fails the
   release if they disagree — this is the floor that catches any
   bypass of the script.
2. Cross-builds release binaries for five targets:
   - `x86_64-unknown-linux-gnu`
   - `aarch64-unknown-linux-gnu` (via `cross`)
   - `x86_64-pc-windows-msvc`
   - `x86_64-apple-darwin`
   - `aarch64-apple-darwin`
3. Packages each as `mritools-v0.3.0-<target>.{tar.gz|zip}` and uploads
   to the GitHub Release at the tag.

## After the release runs

`softprops/action-gh-release@v2` creates the release with an empty
body. Add release notes via:

```bash
gh release edit v0.3.0 --notes-file - <<'EOF'
- Notable change 1
- Notable change 2
EOF
```

Or in the GitHub UI under the release entry.

## If the verify-version job fails

The tag and `Cargo.toml` disagree. Two safe ways out:

1. **Re-tag at the right commit** (you forgot to bump):
   ```bash
   git tag -d v0.3.0
   git push origin :refs/tags/v0.3.0   # delete remote tag
   scripts/release.sh 0.3.0            # bumps + re-tags atomically
   git push origin main && git push origin v0.3.0
   ```

2. **If you really want to ship the existing commit** (rare):
   bump `Cargo.toml` to match the tag in a follow-up commit, force-move
   the tag to that commit, and re-push. This is the messy path —
   prefer #1.

## Why this can't drift

- Locally, the script is the only sanctioned path. It does the bump
  and the tag in the same step, against the same commit.
- In CI, the `verify-version` job runs *before* any binary is built or
  uploaded, so a manual `git tag v0.3.0 && git push --tags` against an
  unbumped manifest fails loudly without ever producing an asset.
- The build matrix `needs: verify-version`, so even if the verify job
  is somehow misconfigured, the binaries can't be uploaded under a
  bad tag.
