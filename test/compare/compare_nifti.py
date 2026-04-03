#!/usr/bin/env python3
"""Compare two NIfTI files and report detailed statistics on differences.

Usage:
    python compare_nifti.py file1.nii file2.nii [--tolerance 1e-6] [--json] [--verbose]
    python compare_nifti.py --dir rust_output/ julia_output/ [--tolerance 1e-6] [--json]

Exit codes:
    0 — Files match within tolerance
    1 — Files differ beyond tolerance
    2 — Error (file not found, incompatible shapes, etc.)
"""

import argparse
import json
import os
import struct
import sys
import gzip
from pathlib import Path

# ── NIfTI-1 minimal reader (no external dependencies) ──────────────────────

NIFTI1_HEADER_SIZE = 348


def _open_nifti(path):
    """Return a file-like object for .nii or .nii.gz."""
    path = str(path)
    if path.endswith(".gz"):
        return gzip.open(path, "rb")
    return open(path, "rb")


def _read_header(fp):
    """Parse NIfTI-1 header fields needed for comparison."""
    hdr_bytes = fp.read(NIFTI1_HEADER_SIZE)
    if len(hdr_bytes) < NIFTI1_HEADER_SIZE:
        raise ValueError("File too small for NIfTI-1 header")

    sizeof_hdr = struct.unpack_from("<i", hdr_bytes, 0)[0]
    if sizeof_hdr != 348:
        raise ValueError(f"Invalid NIfTI-1 header size: {sizeof_hdr}")

    dim = struct.unpack_from("<8h", hdr_bytes, 40)
    ndim = dim[0]
    shape = tuple(dim[1 : ndim + 1])

    datatype = struct.unpack_from("<h", hdr_bytes, 70)[0]
    bitpix = struct.unpack_from("<h", hdr_bytes, 72)[0]

    pixdim = struct.unpack_from("<8f", hdr_bytes, 76)
    voxel_size = tuple(pixdim[1 : ndim + 1])

    vox_offset = struct.unpack_from("<f", hdr_bytes, 108)[0]
    scl_slope = struct.unpack_from("<f", hdr_bytes, 112)[0]
    scl_inter = struct.unpack_from("<f", hdr_bytes, 116)[0]

    return {
        "shape": shape,
        "datatype": datatype,
        "bitpix": bitpix,
        "voxel_size": voxel_size,
        "vox_offset": int(vox_offset),
        "scl_slope": scl_slope,
        "scl_inter": scl_inter,
    }


# Mapping NIfTI datatype codes → struct format characters
_DTYPE_MAP = {
    2: ("B", 1),  # uint8
    4: ("h", 2),  # int16
    8: ("i", 4),  # int32
    16: ("f", 4),  # float32
    32: ("d", 8),  # float64
    64: ("d", 8),  # float64 (complex64 pair — rare)
    256: ("b", 1),  # int8
    512: ("H", 2),  # uint16
    768: ("I", 4),  # uint32
}


def _read_data(fp, header):
    """Read voxel data as a flat list of floats."""
    fp.seek(header["vox_offset"])
    dtype_code = header["datatype"]
    if dtype_code not in _DTYPE_MAP:
        raise ValueError(f"Unsupported NIfTI datatype code: {dtype_code}")

    fmt_char, byte_size = _DTYPE_MAP[dtype_code]
    nvoxels = 1
    for s in header["shape"]:
        nvoxels *= s

    raw = fp.read(nvoxels * byte_size)
    if len(raw) < nvoxels * byte_size:
        raise ValueError(
            f"Data truncated: expected {nvoxels * byte_size} bytes, got {len(raw)}"
        )

    data = list(struct.unpack(f"<{nvoxels}{fmt_char}", raw))

    slope = header["scl_slope"]
    inter = header["scl_inter"]
    if slope != 0 and (slope != 1.0 or inter != 0.0):
        data = [v * slope + inter for v in data]

    return data


def read_nifti(path):
    """Read a NIfTI file and return (header_dict, flat_data_list)."""
    with _open_nifti(path) as fp:
        header = _read_header(fp)
        data = _read_data(fp, header)
    return header, data


# ── Comparison logic ────────────────────────────────────────────────────────


def compare_data(data1, data2, tolerance):
    """Compute comparison statistics between two flat data arrays."""
    n = len(data1)
    if n == 0:
        return {"error": "Empty data arrays"}

    abs_diffs = [abs(a - b) for a, b in zip(data1, data2)]
    max_abs_diff = max(abs_diffs)
    mean_abs_diff = sum(abs_diffs) / n
    rmse = (sum(d * d for d in abs_diffs) / n) ** 0.5

    exact_match = sum(1 for d in abs_diffs if d == 0.0)
    within_tol = sum(1 for d in abs_diffs if d <= tolerance)

    # Data range for normalization
    all_vals = data1 + data2
    vmin = min(all_vals)
    vmax = max(all_vals)
    data_range = vmax - vmin
    nrmse = rmse / data_range if data_range > 0 else 0.0

    # Pearson correlation
    mean1 = sum(data1) / n
    mean2 = sum(data2) / n
    cov = sum((a - mean1) * (b - mean2) for a, b in zip(data1, data2)) / n
    std1 = (sum((a - mean1) ** 2 for a in data1) / n) ** 0.5
    std2 = (sum((b - mean2) ** 2 for b in data2) / n) ** 0.5
    if std1 > 0 and std2 > 0:
        correlation = cov / (std1 * std2)
    else:
        correlation = 1.0 if std1 == 0 and std2 == 0 else 0.0

    # Distribution of differences
    thresholds = [1e-10, 1e-7, 1e-5, 1e-3, 1e-1, 1.0]
    diff_distribution = {}
    for t in thresholds:
        count = sum(1 for d in abs_diffs if d <= t)
        diff_distribution[f"<={t:.0e}"] = count

    return {
        "n_voxels": n,
        "max_abs_diff": max_abs_diff,
        "mean_abs_diff": mean_abs_diff,
        "rmse": rmse,
        "nrmse": nrmse,
        "correlation": correlation,
        "exact_match_count": exact_match,
        "exact_match_pct": 100.0 * exact_match / n,
        "within_tolerance_count": within_tol,
        "within_tolerance_pct": 100.0 * within_tol / n,
        "data_range": data_range,
        "diff_distribution": diff_distribution,
        "pass": max_abs_diff <= tolerance,
    }


def compare_files(file1, file2, tolerance, verbose=False):
    """Compare two NIfTI files. Returns a result dict."""
    result = {
        "file1": str(file1),
        "file2": str(file2),
        "tolerance": tolerance,
    }

    try:
        hdr1, data1 = read_nifti(file1)
        hdr2, data2 = read_nifti(file2)
    except Exception as e:
        result["error"] = str(e)
        result["pass"] = False
        return result

    # Header comparison
    result["shape1"] = hdr1["shape"]
    result["shape2"] = hdr2["shape"]
    result["datatype1"] = hdr1["datatype"]
    result["datatype2"] = hdr2["datatype"]
    result["voxel_size1"] = hdr1["voxel_size"]
    result["voxel_size2"] = hdr2["voxel_size"]

    if hdr1["shape"] != hdr2["shape"]:
        result["error"] = (
            f"Shape mismatch: {hdr1['shape']} vs {hdr2['shape']}"
        )
        result["pass"] = False
        return result

    # Data comparison
    stats = compare_data(data1, data2, tolerance)
    result.update(stats)

    return result


def format_result(result, verbose=False):
    """Format comparison result as a human-readable string."""
    lines = []
    f1 = os.path.basename(result.get("file1", "?"))
    f2 = os.path.basename(result.get("file2", "?"))
    lines.append(f"  Comparing: {f1}  vs  {f2}")

    if "error" in result:
        lines.append(f"  ERROR: {result['error']}")
        return "\n".join(lines)

    passed = result.get("pass", False)
    status = "PASS ✓" if passed else "FAIL ✗"
    lines.append(f"  Status: {status}")
    lines.append(f"  Shape: {result['shape1']}")

    if result.get("datatype1") != result.get("datatype2"):
        lines.append(
            f"  Datatype: {result['datatype1']} vs {result['datatype2']} (DIFFERENT)"
        )

    lines.append(f"  Max absolute diff:  {result['max_abs_diff']:.6e}")
    lines.append(f"  Mean absolute diff: {result['mean_abs_diff']:.6e}")
    lines.append(f"  RMSE:               {result['rmse']:.6e}")
    lines.append(f"  Normalized RMSE:    {result['nrmse']:.6e}")
    lines.append(f"  Correlation:        {result['correlation']:.10f}")
    lines.append(
        f"  Exact match:        {result['exact_match_count']}/{result['n_voxels']} "
        f"({result['exact_match_pct']:.2f}%)"
    )
    lines.append(
        f"  Within tolerance:   {result['within_tolerance_count']}/{result['n_voxels']} "
        f"({result['within_tolerance_pct']:.2f}%)"
    )

    if verbose and "diff_distribution" in result:
        lines.append("  Difference distribution:")
        for threshold, count in result["diff_distribution"].items():
            pct = 100.0 * count / result["n_voxels"]
            lines.append(f"    {threshold}: {count} ({pct:.2f}%)")

    return "\n".join(lines)


# ── Directory comparison ────────────────────────────────────────────────────


def find_nifti_files(directory):
    """Find all NIfTI files in a directory (non-recursive)."""
    nifti_files = {}
    for f in os.listdir(directory):
        if f.endswith(".nii") or f.endswith(".nii.gz"):
            nifti_files[f] = os.path.join(directory, f)
    return nifti_files


def compare_directories(dir1, dir2, tolerance, verbose=False):
    """Compare all matching NIfTI files between two directories."""
    files1 = find_nifti_files(dir1)
    files2 = find_nifti_files(dir2)

    all_names = sorted(set(files1.keys()) | set(files2.keys()))
    results = []

    for name in all_names:
        if name.startswith("settings_"):
            continue  # skip settings files

        if name not in files1:
            results.append(
                {
                    "file1": f"MISSING ({name})",
                    "file2": files2[name],
                    "error": f"File only in {dir2}",
                    "pass": False,
                }
            )
        elif name not in files2:
            results.append(
                {
                    "file1": files1[name],
                    "file2": f"MISSING ({name})",
                    "error": f"File only in {dir1}",
                    "pass": False,
                }
            )
        else:
            result = compare_files(files1[name], files2[name], tolerance, verbose)
            results.append(result)

    return results


# ── CLI ─────────────────────────────────────────────────────────────────────


def main():
    parser = argparse.ArgumentParser(
        description="Compare NIfTI files between Rust and Julia implementations",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog=__doc__,
    )

    parser.add_argument(
        "paths",
        nargs="*",
        help="Two NIfTI files or two directories to compare",
    )
    parser.add_argument(
        "--dir",
        action="store_true",
        help="Compare all matching NIfTI files in two directories",
    )
    parser.add_argument(
        "--tolerance",
        type=float,
        default=1e-6,
        help="Maximum allowed absolute difference (default: 1e-6)",
    )
    parser.add_argument(
        "--json",
        action="store_true",
        help="Output results as JSON",
    )
    parser.add_argument(
        "--verbose", "-v",
        action="store_true",
        help="Show detailed difference distribution",
    )

    args = parser.parse_args()

    if len(args.paths) != 2:
        parser.error("Exactly two paths required (files or directories)")

    path1, path2 = args.paths

    if args.dir or (os.path.isdir(path1) and os.path.isdir(path2)):
        results = compare_directories(path1, path2, args.tolerance, args.verbose)
    else:
        results = [compare_files(path1, path2, args.tolerance, args.verbose)]

    # Output
    if args.json:
        # Convert tuples to lists for JSON serialization
        def make_serializable(obj):
            if isinstance(obj, dict):
                return {k: make_serializable(v) for k, v in obj.items()}
            if isinstance(obj, (list, tuple)):
                return [make_serializable(i) for i in obj]
            return obj

        print(json.dumps(make_serializable(results), indent=2))
    else:
        all_pass = True
        for r in results:
            print(format_result(r, args.verbose))
            print()
            if not r.get("pass", False):
                all_pass = False

        n_pass = sum(1 for r in results if r.get("pass", False))
        n_total = len(results)
        print(f"{'=' * 60}")
        print(f"Summary: {n_pass}/{n_total} comparisons passed (tolerance={args.tolerance:.0e})")
        if all_pass:
            print("Overall: PASS ✓")
        else:
            print("Overall: FAIL ✗")

    # Exit code
    all_pass = all(r.get("pass", False) for r in results)
    sys.exit(0 if all_pass else 1)


if __name__ == "__main__":
    main()
