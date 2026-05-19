#!/usr/bin/env bash
# Fetch larger test data for the Rust↔Julia comparison harness.
#
# This script is intentionally URL-agnostic: it reads a manifest of
# (path, url, sha256) tuples from a config file you point it at, and
# downloads + verifies each entry. No URLs are committed to the repo,
# so the script works the same against OSF, Zenodo, GitHub Releases,
# S3, or a local mirror.
#
# Usage:
#   test/data/extra/fetch.sh [path/to/manifest.tsv]
#
# Manifest format (tab-separated; lines starting with # are skipped):
#   <relative_path>  <url>  <sha256>
# Example:
#   Phase.nii  https://example.org/path/Phase.nii  3a2b1c...
#   Mag.nii    https://example.org/path/Mag.nii    9d8e7f...
#
# Once fetched, drive the harness against the new tree with:
#   test/compare/run_comparison.sh --data-dir test/data/extra \
#       --phase Phase.nii --mag Mag.nii --echo-times "<your tes>"

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MANIFEST="${1:-$SCRIPT_DIR/manifest.tsv}"

if [[ ! -f "$MANIFEST" ]]; then
    cat >&2 <<EOF
ERROR: Manifest file not found: $MANIFEST

Create a manifest with one (path, url, sha256) per line, tab-separated.
See test/data/extra/README.md for the expected layout and an example.
EOF
    exit 2
fi

cd "$SCRIPT_DIR"

while IFS=$'\t' read -r path url sha256; do
    # Skip blanks and comments
    [[ -z "${path:-}" || "$path" == \#* ]] && continue

    if [[ -f "$path" ]]; then
        existing=$(sha256sum "$path" | awk '{print $1}')
        if [[ "$existing" == "$sha256" ]]; then
            echo "OK    $path (already present, checksum matches)"
            continue
        else
            echo "STALE $path (checksum mismatch, re-downloading)"
            rm -f "$path"
        fi
    fi

    mkdir -p "$(dirname "$path")"
    echo "FETCH $path  <-  $url"
    if command -v curl >/dev/null 2>&1; then
        curl -L --fail -o "$path" "$url"
    elif command -v wget >/dev/null 2>&1; then
        wget -O "$path" "$url"
    else
        echo "ERROR: neither curl nor wget available" >&2
        exit 3
    fi

    actual=$(sha256sum "$path" | awk '{print $1}')
    if [[ "$actual" != "$sha256" ]]; then
        echo "ERROR: sha256 mismatch for $path" >&2
        echo "  expected: $sha256" >&2
        echo "  got:      $actual" >&2
        rm -f "$path"
        exit 4
    fi
    echo "OK    $path"
done < "$MANIFEST"

echo "All fetched files verified."
