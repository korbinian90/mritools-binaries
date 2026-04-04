#!/usr/bin/env julia
#=
Compare two NIfTI files and report detailed statistics on differences.

Usage:
    julia --project=test/compare/julia compare_nifti.jl file1.nii file2.nii [--tolerance 1e-6] [--json] [--verbose]
    julia --project=test/compare/julia compare_nifti.jl --dir rust_output/ julia_output/ [--tolerance 1e-6] [--json]

Exit codes:
    0 — Files match within tolerance
    1 — Files differ beyond tolerance
    2 — Error (file not found, incompatible shapes, etc.)
=#

using NIfTI
using Statistics
using Printf
using JSON3

# ── Comparison logic ────────────────────────────────────────────────────────

function compare_data(data1::AbstractArray, data2::AbstractArray, tolerance::Float64)
    v1 = Float64.(vec(data1))
    v2 = Float64.(vec(data2))
    n = length(v1)
    n == 0 && return Dict("error" => "Empty data arrays")

    abs_diffs = abs.(v1 .- v2)
    max_abs_diff = maximum(abs_diffs)
    mean_abs_diff = mean(abs_diffs)
    rmse = sqrt(mean(abs_diffs .^ 2))

    exact_match = count(==(0.0), abs_diffs)
    within_tol = count(<=(tolerance), abs_diffs)

    # Data range for normalization
    all_vals = vcat(v1, v2)
    data_range = maximum(all_vals) - minimum(all_vals)
    nrmse = data_range > 0 ? rmse / data_range : 0.0

    # Pearson correlation
    corr = if std(v1) > 0 && std(v2) > 0
        cor(v1, v2)
    else
        std(v1) == 0 && std(v2) == 0 ? 1.0 : 0.0
    end

    # Distribution of differences
    thresholds = [1e-10, 1e-7, 1e-5, 1e-3, 1e-1, 1.0]
    diff_distribution = Dict{String,Int}()
    for t in thresholds
        key = string("<=", @sprintf("%.0e", t))
        diff_distribution[key] = count(<=(t), abs_diffs)
    end

    return Dict(
        "n_voxels" => n,
        "max_abs_diff" => max_abs_diff,
        "mean_abs_diff" => mean_abs_diff,
        "rmse" => rmse,
        "nrmse" => nrmse,
        "correlation" => corr,
        "exact_match_count" => exact_match,
        "exact_match_pct" => 100.0 * exact_match / n,
        "within_tolerance_count" => within_tol,
        "within_tolerance_pct" => 100.0 * within_tol / n,
        "data_range" => data_range,
        "diff_distribution" => diff_distribution,
        "pass" => max_abs_diff <= tolerance,
    )
end

function compare_files(file1::String, file2::String, tolerance::Float64)
    result = Dict{String,Any}(
        "file1" => file1,
        "file2" => file2,
        "tolerance" => tolerance,
    )

    local nii1, nii2
    try
        nii1 = niread(file1)
        nii2 = niread(file2)
    catch e
        result["error"] = string(e)
        result["pass"] = false
        return result
    end

    shape1 = size(nii1)
    shape2 = size(nii2)
    result["shape1"] = collect(shape1)
    result["shape2"] = collect(shape2)
    result["datatype1"] = Int(nii1.header.datatype)
    result["datatype2"] = Int(nii2.header.datatype)
    result["voxel_size1"] = collect(Float64.(nii1.header.pixdim[2:ndims(nii1)+1]))
    result["voxel_size2"] = collect(Float64.(nii2.header.pixdim[2:ndims(nii2)+1]))

    if shape1 != shape2
        result["error"] = "Shape mismatch: $shape1 vs $shape2"
        result["pass"] = false
        return result
    end

    stats = compare_data(nii1.raw, nii2.raw, tolerance)
    merge!(result, stats)
    return result
end

# ── Formatting ──────────────────────────────────────────────────────────────

function format_result(result::Dict; verbose::Bool=false)
    lines = String[]
    f1 = basename(get(result, "file1", "?"))
    f2 = basename(get(result, "file2", "?"))
    push!(lines, "  Comparing: $f1  vs  $f2")

    if haskey(result, "error")
        push!(lines, "  ERROR: $(result["error"])")
        return join(lines, "\n")
    end

    passed = get(result, "pass", false)
    status = passed ? "PASS ✓" : "FAIL ✗"
    push!(lines, "  Status: $status")
    push!(lines, "  Shape: $(result["shape1"])")

    if get(result, "datatype1", 0) != get(result, "datatype2", 0)
        push!(lines, "  Datatype: $(result["datatype1"]) vs $(result["datatype2"]) (DIFFERENT)")
    end

    push!(lines, @sprintf("  Max absolute diff:  %.6e", result["max_abs_diff"]))
    push!(lines, @sprintf("  Mean absolute diff: %.6e", result["mean_abs_diff"]))
    push!(lines, @sprintf("  RMSE:               %.6e", result["rmse"]))
    push!(lines, @sprintf("  Normalized RMSE:    %.6e", result["nrmse"]))
    push!(lines, @sprintf("  Correlation:        %.10f", result["correlation"]))
    push!(lines, @sprintf("  Exact match:        %d/%d (%.2f%%)",
        result["exact_match_count"], result["n_voxels"], result["exact_match_pct"]))
    push!(lines, @sprintf("  Within tolerance:   %d/%d (%.2f%%)",
        result["within_tolerance_count"], result["n_voxels"], result["within_tolerance_pct"]))

    if verbose && haskey(result, "diff_distribution")
        push!(lines, "  Difference distribution:")
        for (threshold, count) in sort(collect(result["diff_distribution"]))
            pct = 100.0 * count / result["n_voxels"]
            push!(lines, @sprintf("    %s: %d (%.2f%%)", threshold, count, pct))
        end
    end

    return join(lines, "\n")
end

# ── Directory comparison ────────────────────────────────────────────────────

function find_nifti_files(directory::String)
    files = Dict{String,String}()
    for f in readdir(directory)
        if endswith(f, ".nii") || endswith(f, ".nii.gz")
            files[f] = joinpath(directory, f)
        end
    end
    return files
end

function compare_directories(dir1::String, dir2::String, tolerance::Float64)
    files1 = find_nifti_files(dir1)
    files2 = find_nifti_files(dir2)

    all_names = sort(collect(union(keys(files1), keys(files2))))
    results = Dict{String,Any}[]

    for name in all_names
        startswith(name, "settings_") && continue

        if !haskey(files1, name)
            push!(results, Dict{String,Any}(
                "file1" => "MISSING ($name)",
                "file2" => files2[name],
                "error" => "File only in $dir2",
                "pass" => false,
            ))
        elseif !haskey(files2, name)
            push!(results, Dict{String,Any}(
                "file1" => files1[name],
                "file2" => "MISSING ($name)",
                "error" => "File only in $dir1",
                "pass" => false,
            ))
        else
            push!(results, compare_files(files1[name], files2[name], tolerance))
        end
    end

    return results
end

# ── JSON output ─────────────────────────────────────────────────────────────

function to_json(results)
    # Use JSON3 if available, otherwise manual formatting
    try
        return JSON3.pretty(results)
    catch
        return string(results)
    end
end

# ── CLI ─────────────────────────────────────────────────────────────────────

function parse_cli_args(args)
    tolerance = 1e-6
    json_output = false
    verbose = false
    dir_mode = false
    paths = String[]

    i = 1
    while i <= length(args)
        arg = args[i]
        if arg == "--tolerance" && i < length(args)
            tolerance = parse(Float64, args[i+1])
            i += 2
        elseif arg == "--json"
            json_output = true
            i += 1
        elseif arg == "--verbose" || arg == "-v"
            verbose = true
            i += 1
        elseif arg == "--dir"
            dir_mode = true
            i += 1
        elseif arg == "--help" || arg == "-h"
            println("""
Compare NIfTI files between Rust and Julia implementations.

Usage:
    julia compare_nifti.jl file1.nii file2.nii [--tolerance 1e-6] [--json] [--verbose]
    julia compare_nifti.jl --dir dir1/ dir2/ [--tolerance 1e-6] [--json] [--verbose]

Options:
    --tolerance TOL  Maximum allowed absolute difference (default: 1e-6)
    --json           Output results as JSON
    --verbose, -v    Show detailed difference distribution
    --dir            Compare all matching NIfTI files in two directories
    --help, -h       Show this help
""")
            exit(0)
        else
            push!(paths, arg)
            i += 1
        end
    end

    return (; tolerance, json_output, verbose, dir_mode, paths)
end

function main()
    opts = parse_cli_args(ARGS)

    if length(opts.paths) != 2
        println(stderr, "Error: Exactly two paths required (files or directories)")
        exit(2)
    end

    path1, path2 = opts.paths
    dir_mode = opts.dir_mode || (isdir(path1) && isdir(path2))

    results = if dir_mode
        compare_directories(path1, path2, opts.tolerance)
    else
        [compare_files(path1, path2, opts.tolerance)]
    end

    if opts.json_output
        println(to_json(results))
    else
        all_pass = true
        for r in results
            println(format_result(r; verbose=opts.verbose))
            println()
            if !get(r, "pass", false)
                all_pass = false
            end
        end

        n_pass = count(r -> get(r, "pass", false), results)
        n_total = length(results)
        println("=" ^ 60)
        @printf("Summary: %d/%d comparisons passed (tolerance=%.0e)\n", n_pass, n_total, opts.tolerance)
        if all_pass
            println("Overall: PASS ✓")
        else
            println("Overall: FAIL ✗")
        end
    end

    all_pass = all(r -> get(r, "pass", false), results)
    exit(all_pass ? 0 : 1)
end

main()
