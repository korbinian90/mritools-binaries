#!/usr/bin/env bash
# scripts/release.sh — bump workspace version, run sanity checks,
# commit, and tag in one atomic step.
#
# Usage:
#   scripts/release.sh X.Y.Z
#
# After this script succeeds, push with:
#   git push origin main
#   git push origin vX.Y.Z
#
# The push of the tag triggers .github/workflows/release.yml, which
# first verifies Cargo.toml matches the tag (would fail if you skipped
# this script and tagged manually with a mismatched manifest).

set -euo pipefail

if [[ $# -ne 1 ]]; then
    echo "Usage: $0 <version>      # e.g. $0 0.3.0" >&2
    exit 1
fi

VERSION="$1"

# ── Validate semver (basic) ────────────────────────────────────────────
if [[ ! "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[A-Za-z0-9.-]+)?$ ]]; then
    echo "ERROR: version must be X.Y.Z or X.Y.Z-prerelease (got '$VERSION')" >&2
    exit 2
fi

# ── Locate repo root + verify branch / cleanliness ─────────────────────
REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

BRANCH="$(git rev-parse --abbrev-ref HEAD)"
if [[ "$BRANCH" != "main" ]]; then
    echo "ERROR: must be on 'main' (currently on '$BRANCH')." >&2
    echo "       Merge the feature branch to main, then re-run this script." >&2
    exit 3
fi

if [[ -n "$(git status --porcelain)" ]]; then
    echo "ERROR: working tree is not clean. Commit or stash first." >&2
    git status --short >&2
    exit 4
fi

# ── Refuse to reuse an existing tag ─────────────────────────────────────
if git rev-parse "v$VERSION" >/dev/null 2>&1; then
    echo "ERROR: tag v$VERSION already exists locally." >&2
    exit 5
fi
if git ls-remote --tags origin "v$VERSION" | grep -q .; then
    echo "ERROR: tag v$VERSION already exists on origin." >&2
    exit 5
fi

# ── Bump Cargo.toml's workspace.package.version ────────────────────────
# We only touch the version line *inside* the [workspace.package] block,
# so any per-crate `version = "..."` lines (currently none — they all use
# `version.workspace = true`) would be safe regardless.
python3 - "$VERSION" <<'PYEOF'
import re, sys
new_version = sys.argv[1]
with open("Cargo.toml") as f:
    text = f.read()

pattern = re.compile(
    r'(\[workspace\.package\][^\[]*?)version\s*=\s*"[^"]+"',
    re.DOTALL,
)
new_text, n = pattern.subn(rf'\g<1>version = "{new_version}"', text, count=1)
if n != 1:
    sys.stderr.write("ERROR: could not find version under [workspace.package]\n")
    sys.exit(1)
with open("Cargo.toml", "w") as f:
    f.write(new_text)
PYEOF

# ── Sanity checks: don't tag a broken commit ───────────────────────────
# `cargo test` refreshes Cargo.lock to reflect the bumped workspace
# version as a side effect of resolving the build.
echo
echo "▶ Running cargo test --workspace --release ..."
cargo test --workspace --release
echo
echo "▶ Running cargo clippy --workspace -- -D warnings ..."
cargo clippy --workspace --release -- -D warnings
echo
echo "▶ Running cargo fmt --all -- --check ..."
cargo fmt --all -- --check

# ── Commit + tag ───────────────────────────────────────────────────────
git add Cargo.toml Cargo.lock
git commit -m "Release v$VERSION"
git tag -a "v$VERSION" -m "Release v$VERSION"

cat <<EOF

✔ Version bumped to $VERSION
✔ Cargo.lock refreshed
✔ Tests, clippy, and fmt all clean
✔ Commit and tag v$VERSION created locally

Next:
    git push origin main
    git push origin v$VERSION

Pushing the tag triggers .github/workflows/release.yml — it first runs
a verify-version job (which would have caught any drift between
Cargo.toml and the tag), then cross-builds and publishes the GitHub
release.
EOF
