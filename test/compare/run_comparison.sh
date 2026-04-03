#!/usr/bin/env bash
# =============================================================================
# Rust vs Julia Comparison Test Runner
#
# Runs each mritools binary (Rust) and its Julia equivalent on the same input
# data, then compares all NIfTI outputs voxel-by-voxel.
#
# Usage:
#   ./run_comparison.sh [OPTIONS]
#
# Options:
#   --data-dir DIR       Input data directory (default: test/data/small)
#   --phase FILE         Phase filename within data dir (default: Phase.nii)
#   --mag FILE           Magnitude filename within data dir (default: Mag.nii)
#   --echo-times T...    Echo times in ms (default: 1 2 3)
#   --tools TOOLS        Comma-separated tools to compare (default: all)
#                        Options: romeo,clearswi,mcpc3ds,makehomogeneous,romeo_mask
#   --tolerance TOL      Max allowed absolute difference (default: 1e-6)
#   --rust-bin-dir DIR   Path to Rust binaries (default: auto-detect via cargo)
#   --julia JULIA        Julia executable (default: julia)
#   --output-dir DIR     Where to store outputs (default: /tmp/mritools_compare)
#   --skip-build         Skip building Rust binaries
#   --skip-rust          Skip running Rust (reuse previous output)
#   --skip-julia         Skip running Julia (reuse previous output)
#   --verbose            Show detailed comparison output
#   --help               Show this help
# =============================================================================

set -euo pipefail

# ── Defaults ─────────────────────────────────────────────────────────────────

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

DATA_DIR="$REPO_ROOT/test/data/small"
PHASE_FILE="Phase.nii"
MAG_FILE="Mag.nii"
ECHO_TIMES="1 2 3"
TOOLS="romeo,clearswi,mcpc3ds,makehomogeneous,romeo_mask"
TOLERANCE="1e-6"
RUST_BIN_DIR=""
JULIA_BIN="julia"
OUTPUT_DIR="/tmp/mritools_compare"
SKIP_BUILD=false
SKIP_RUST=false
SKIP_JULIA=false
VERBOSE=false
JULIA_PROJECT="$SCRIPT_DIR/julia"

# ── Parse arguments ──────────────────────────────────────────────────────────

while [[ $# -gt 0 ]]; do
    case "$1" in
        --data-dir)      DATA_DIR="$2"; shift 2 ;;
        --phase)         PHASE_FILE="$2"; shift 2 ;;
        --mag)           MAG_FILE="$2"; shift 2 ;;
        --echo-times)    shift; ECHO_TIMES=""; while [[ $# -gt 0 && ! "$1" =~ ^-- ]]; do ECHO_TIMES="$ECHO_TIMES $1"; shift; done ;;
        --tools)         TOOLS="$2"; shift 2 ;;
        --tolerance)     TOLERANCE="$2"; shift 2 ;;
        --rust-bin-dir)  RUST_BIN_DIR="$2"; shift 2 ;;
        --julia)         JULIA_BIN="$2"; shift 2 ;;
        --output-dir)    OUTPUT_DIR="$2"; shift 2 ;;
        --skip-build)    SKIP_BUILD=true; shift ;;
        --skip-rust)     SKIP_RUST=true; shift ;;
        --skip-julia)    SKIP_JULIA=true; shift ;;
        --verbose)       VERBOSE=true; shift ;;
        --help|-h)
            sed -n '2,/^# =====/p' "${BASH_SOURCE[0]}" | sed 's/^# \?//'
            exit 0 ;;
        *) echo "Unknown option: $1"; exit 1 ;;
    esac
done

ECHO_TIMES="$(echo "$ECHO_TIMES" | xargs)"  # trim whitespace

# ── Resolve paths ────────────────────────────────────────────────────────────

PHASE_PATH="$DATA_DIR/$PHASE_FILE"
MAG_PATH="$DATA_DIR/$MAG_FILE"

if [[ ! -f "$PHASE_PATH" ]]; then
    echo "ERROR: Phase file not found: $PHASE_PATH"
    exit 2
fi
if [[ ! -f "$MAG_PATH" ]]; then
    echo "ERROR: Magnitude file not found: $MAG_PATH"
    exit 2
fi

RUST_OUT="$OUTPUT_DIR/rust"
JULIA_OUT="$OUTPUT_DIR/julia"
mkdir -p "$RUST_OUT" "$JULIA_OUT"

# Convert echo times to formats needed by each tool
# Rust uses space-separated or Julia range syntax: "1:3" or "1 2 3"
ECHO_TIMES_ARRAY=($ECHO_TIMES)
N_ECHOES=${#ECHO_TIMES_ARRAY[@]}
# Julia scripts take space-separated values
JULIA_ECHO_ARGS=""
for t in "${ECHO_TIMES_ARRAY[@]}"; do
    JULIA_ECHO_ARGS="$JULIA_ECHO_ARGS --echo-times $t"
done
# Rust CLIs take space-separated values via -t
RUST_ECHO_ARGS=""
for t in "${ECHO_TIMES_ARRAY[@]}"; do
    RUST_ECHO_ARGS="$RUST_ECHO_ARGS $t"
done

echo "============================================================"
echo " mritools: Rust vs Julia Comparison"
echo "============================================================"
echo "  Data dir:    $DATA_DIR"
echo "  Phase:       $PHASE_FILE"
echo "  Magnitude:   $MAG_FILE"
echo "  Echo times:  $ECHO_TIMES"
echo "  Tools:       $TOOLS"
echo "  Tolerance:   $TOLERANCE"
echo "  Output dir:  $OUTPUT_DIR"
echo "============================================================"
echo ""

# ── Build Rust ───────────────────────────────────────────────────────────────

if [[ -z "$RUST_BIN_DIR" ]]; then
    RUST_BIN_DIR="$REPO_ROOT/target/release"
fi

if [[ "$SKIP_BUILD" == false ]]; then
    echo "▶ Building Rust binaries..."
    (cd "$REPO_ROOT" && cargo build --workspace --release 2>&1) || {
        echo "ERROR: Rust build failed"; exit 1
    }
    echo "  ✓ Build complete"
    echo ""
fi

# ── Check Julia ──────────────────────────────────────────────────────────────

if [[ "$SKIP_JULIA" == false ]]; then
    if ! command -v "$JULIA_BIN" &>/dev/null; then
        echo "WARNING: Julia not found at '$JULIA_BIN'"
        echo "  Install Julia or use --julia /path/to/julia"
        echo "  Skipping Julia runs (will only run Rust)"
        SKIP_JULIA=true
    else
        JULIA_VERSION=$("$JULIA_BIN" --version 2>&1 || true)
        echo "▶ Julia: $JULIA_VERSION"

        # Check if Julia packages are installed
        echo "  Checking Julia packages..."
        "$JULIA_BIN" --project="$JULIA_PROJECT" -e '
            import Pkg
            Pkg.instantiate()
            using ROMEO, CLEARSWI, MriResearchTools, NIfTI, ArgParse
            println("  ✓ All Julia packages available")
        ' 2>&1 || {
            echo "  Julia packages not yet installed. Installing..."
            "$JULIA_BIN" --project="$JULIA_PROJECT" -e '
                import Pkg
                Pkg.instantiate()
                Pkg.precompile()
            ' 2>&1
        }
        echo ""
    fi
fi

# ── Tool runners ─────────────────────────────────────────────────────────────

IFS=',' read -ra TOOL_LIST <<< "$TOOLS"

run_rust_tool() {
    local tool="$1"
    local out_dir="$RUST_OUT/$tool"
    mkdir -p "$out_dir"

    echo "  ▶ Rust: $tool"

    case "$tool" in
        romeo)
            "$RUST_BIN_DIR/romeo" \
                -p "$PHASE_PATH" \
                -m "$MAG_PATH" \
                -t $RUST_ECHO_ARGS \
                -B \
                -o "$out_dir/unwrapped.nii" \
                2>&1 | { $VERBOSE && cat || tail -1; } || true
            ;;
        clearswi)
            "$RUST_BIN_DIR/clearswi" \
                -m "$MAG_PATH" \
                -p "$PHASE_PATH" \
                -t $RUST_ECHO_ARGS \
                -o "$out_dir/clearswi.nii" \
                2>&1 | { $VERBOSE && cat || tail -1; } || true
            ;;
        mcpc3ds)
            "$RUST_BIN_DIR/mcpc3ds" \
                -p "$PHASE_PATH" \
                -m "$MAG_PATH" \
                -t $RUST_ECHO_ARGS \
                -o "$out_dir/output" \
                2>&1 | { $VERBOSE && cat || tail -1; } || true
            ;;
        makehomogeneous)
            "$RUST_BIN_DIR/makehomogeneous" \
                -m "$MAG_PATH" \
                -o "$out_dir/homogeneous" \
                2>&1 | { $VERBOSE && cat || tail -1; } || true
            ;;
        romeo_mask)
            "$RUST_BIN_DIR/romeo_mask" \
                -p "$PHASE_PATH" \
                -m "$MAG_PATH" \
                -t $RUST_ECHO_ARGS \
                -o "$out_dir/mask.nii" \
                2>&1 | { $VERBOSE && cat || tail -1; } || true
            ;;
        *)
            echo "    Unknown tool: $tool"
            return 1
            ;;
    esac

    # List outputs
    local n_files
    n_files=$(find "$out_dir" -name "*.nii" -o -name "*.nii.gz" | wc -l)
    echo "    → $n_files output file(s) in $out_dir"
}

run_julia_tool() {
    local tool="$1"
    local out_dir="$JULIA_OUT/$tool"
    mkdir -p "$out_dir"

    echo "  ▶ Julia: $tool"

    case "$tool" in
        romeo)
            "$JULIA_BIN" --project="$JULIA_PROJECT" "$JULIA_PROJECT/run_romeo.jl" \
                --phase "$PHASE_PATH" \
                --magnitude "$MAG_PATH" \
                $JULIA_ECHO_ARGS \
                --compute-B0 \
                --output "$out_dir/unwrapped.nii" \
                2>&1 | { $VERBOSE && cat || tail -1; } || true
            ;;
        clearswi)
            "$JULIA_BIN" --project="$JULIA_PROJECT" "$JULIA_PROJECT/run_clearswi.jl" \
                --magnitude "$MAG_PATH" \
                --phase "$PHASE_PATH" \
                $JULIA_ECHO_ARGS \
                --output "$out_dir/clearswi.nii" \
                2>&1 | { $VERBOSE && cat || tail -1; } || true
            ;;
        mcpc3ds)
            "$JULIA_BIN" --project="$JULIA_PROJECT" "$JULIA_PROJECT/run_mcpc3ds.jl" \
                --phase "$PHASE_PATH" \
                --magnitude "$MAG_PATH" \
                $JULIA_ECHO_ARGS \
                --output "$out_dir/output.nii" \
                2>&1 | { $VERBOSE && cat || tail -1; } || true
            ;;
        makehomogeneous)
            "$JULIA_BIN" --project="$JULIA_PROJECT" "$JULIA_PROJECT/run_makehomogeneous.jl" \
                --magnitude "$MAG_PATH" \
                --output "$out_dir/homogeneous.nii" \
                2>&1 | { $VERBOSE && cat || tail -1; } || true
            ;;
        romeo_mask)
            "$JULIA_BIN" --project="$JULIA_PROJECT" "$JULIA_PROJECT/run_romeo_mask.jl" \
                --phase "$PHASE_PATH" \
                --magnitude "$MAG_PATH" \
                $JULIA_ECHO_ARGS \
                --output "$out_dir/mask.nii" \
                2>&1 | { $VERBOSE && cat || tail -1; } || true
            ;;
        *)
            echo "    Unknown tool: $tool"
            return 1
            ;;
    esac

    local n_files
    n_files=$(find "$out_dir" -name "*.nii" -o -name "*.nii.gz" | wc -l)
    echo "    → $n_files output file(s) in $out_dir"
}

# ── Run tools ────────────────────────────────────────────────────────────────

if [[ "$SKIP_RUST" == false ]]; then
    echo "────────────────────────────────────────────────────────────"
    echo "Running Rust binaries"
    echo "────────────────────────────────────────────────────────────"
    for tool in "${TOOL_LIST[@]}"; do
        tool=$(echo "$tool" | xargs)  # trim
        run_rust_tool "$tool"
    done
    echo ""
fi

if [[ "$SKIP_JULIA" == false ]]; then
    echo "────────────────────────────────────────────────────────────"
    echo "Running Julia implementations"
    echo "────────────────────────────────────────────────────────────"
    for tool in "${TOOL_LIST[@]}"; do
        tool=$(echo "$tool" | xargs)
        run_julia_tool "$tool"
    done
    echo ""
fi

# ── Compare outputs ──────────────────────────────────────────────────────────

echo "────────────────────────────────────────────────────────────"
echo "Comparing outputs (tolerance=$TOLERANCE)"
echo "────────────────────────────────────────────────────────────"

COMPARE_SCRIPT="$SCRIPT_DIR/compare_nifti.py"
OVERALL_PASS=true
COMPARE_FLAGS=""
if [[ "$VERBOSE" == true ]]; then
    COMPARE_FLAGS="--verbose"
fi

for tool in "${TOOL_LIST[@]}"; do
    tool=$(echo "$tool" | xargs)
    rust_dir="$RUST_OUT/$tool"
    julia_dir="$JULIA_OUT/$tool"

    echo ""
    echo "═══ $tool ═══"

    if [[ ! -d "$rust_dir" ]]; then
        echo "  SKIP: No Rust output directory"
        continue
    fi
    if [[ ! -d "$julia_dir" ]]; then
        echo "  SKIP: No Julia output directory"
        continue
    fi

    # Find all NIfTI files in rust output and compare with julia
    found_any=false
    for rust_file in "$rust_dir"/*.nii "$rust_dir"/*.nii.gz; do
        [[ -f "$rust_file" ]] || continue
        basename=$(basename "$rust_file")

        # Skip settings files
        [[ "$basename" == settings_* ]] && continue

        julia_file="$julia_dir/$basename"
        if [[ ! -f "$julia_file" ]]; then
            echo "  MISSING in Julia: $basename"
            OVERALL_PASS=false
            continue
        fi

        found_any=true
        python3 "$COMPARE_SCRIPT" "$rust_file" "$julia_file" \
            --tolerance "$TOLERANCE" $COMPARE_FLAGS || OVERALL_PASS=false
    done

    # Check for files only in julia output
    for julia_file in "$julia_dir"/*.nii "$julia_dir"/*.nii.gz; do
        [[ -f "$julia_file" ]] || continue
        basename=$(basename "$julia_file")
        [[ "$basename" == settings_* ]] && continue
        rust_file="$rust_dir/$basename"
        if [[ ! -f "$rust_file" ]]; then
            echo "  EXTRA in Julia (not in Rust): $basename"
        fi
    done

    if [[ "$found_any" == false ]]; then
        echo "  No matching NIfTI files to compare"
    fi
done

# ── Summary ──────────────────────────────────────────────────────────────────

echo ""
echo "============================================================"
if [[ "$OVERALL_PASS" == true ]]; then
    echo " OVERALL: PASS ✓  (all outputs match within tolerance=$TOLERANCE)"
else
    echo " OVERALL: FAIL ✗  (some outputs differ beyond tolerance=$TOLERANCE)"
fi
echo "============================================================"
echo ""
echo "Output directories:"
echo "  Rust:  $RUST_OUT"
echo "  Julia: $JULIA_OUT"

if [[ "$OVERALL_PASS" == true ]]; then
    exit 0
else
    exit 1
fi
