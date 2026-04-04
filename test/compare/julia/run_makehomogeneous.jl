#!/usr/bin/env julia
# Run makehomogeneous via Julia MriResearchTools.jl
#
# Usage:
#   julia --project=test/compare/julia run_makehomogeneous.jl \
#       --magnitude Mag.nii --output homogeneous.nii \
#       [--sigma 7.0] [--nbox 15]

using MriResearchTools
using NIfTI
using ArgParse

function parse_args()
    s = ArgParseSettings(description="Run makehomogeneous via Julia for comparison testing")
    @add_arg_table! s begin
        "--magnitude", "-m"
            help = "Magnitude NIfTI file (required)"
            required = true
        "--output", "-o"
            help = "Output homogeneous image"
            default = "homogeneous.nii"
        "--sigma", "-s"
            help = "Sigma for bias field smoothing in mm"
            arg_type = Float64
            default = 7.0
        "--nbox", "-n"
            help = "Number of boxes for segmentation"
            arg_type = Int
            default = 15
    end
    return ArgParse.parse_args(s)
end

function main()
    args = parse_args()

    # Load magnitude
    mag_nii = niread(args["magnitude"])
    mag_data = Float64.(mag_nii.raw)

    println("Running makehomogeneous via MriResearchTools.jl...")
    println("  Magnitude shape: ", size(mag_data))
    println("  Sigma: ", args["sigma"])
    println("  N boxes: ", args["nbox"])

    # Process each echo independently (matching Rust behavior)
    if ndims(mag_data) == 4
        result = similar(mag_data)
        for echo in 1:size(mag_data, 4)
            vol = @view mag_data[:, :, :, echo]
            result[:, :, :, echo] = makehomogeneous(vol;
                sigma_mm=args["sigma"],
                nbox=args["nbox"]
            )
        end
    else
        result = makehomogeneous(mag_data;
            sigma_mm=args["sigma"],
            nbox=args["nbox"]
        )
    end

    # Save output
    output_path = args["output"]
    if !endswith(output_path, ".nii") && !endswith(output_path, ".nii.gz")
        output_path *= ".nii"
    end
    mkpath(dirname(abspath(output_path)))
    savenii(result, output_path; header=mag_nii.header)
    println("  Saved: ", output_path)

    println("makehomogeneous completed successfully")
end

main()
