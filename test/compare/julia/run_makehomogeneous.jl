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
        "--writesteps"
            help = "Directory to write canonical intermediate NIfTIs to"
            default = nothing
    end
    return ArgParse.parse_args(s)
end

function main()
    args = parse_args()

    steps_dir = args["writesteps"]
    if steps_dir !== nothing
        mkpath(steps_dir)
    end

    # Load magnitude
    mag_nii = niread(args["magnitude"])
    mag_data = Float64.(mag_nii.raw)

    # Convert sigma_mm to voxels using the NIfTI pixdim, matching the Rust binary.
    pixdim = mag_nii.header.pixdim[2:1+min(3, ndims(mag_data))]
    sigma_vox = collect(args["sigma"] ./ pixdim)

    println("Running makehomogeneous via MriResearchTools.jl...")
    println("  Magnitude shape: ", size(mag_data))
    println("  Sigma (mm): ", args["sigma"])
    println("  Sigma (vox): ", sigma_vox)
    println("  N boxes: ", args["nbox"])

    # Process each echo independently (matching Rust behavior)
    if ndims(mag_data) == 4
        result = similar(mag_data)
        for echo in 1:size(mag_data, 4)
            vol = mag_data[:, :, :, echo]
            result[:, :, :, echo] = makehomogeneous(vol;
                sigma=sigma_vox,
                nbox=args["nbox"]
            )
        end
    else
        result = makehomogeneous(mag_data;
            sigma=sigma_vox,
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

    # Canonical step dumps
    if steps_dir !== nothing
        vol_in = ndims(mag_data) == 4 ? mag_data[:, :, :, 1] : mag_data
        vol_out = ndims(result) == 4 ? result[:, :, :, 1] : result
        # Bias field = input / output (Gaussian-smoothed quotient recovers it)
        bias = similar(vol_in)
        for i in eachindex(vol_in)
            bias[i] = abs(vol_out[i]) > 1e-10 ? vol_in[i] / vol_out[i] : 1.0
        end
        savenii(bias, joinpath(steps_dir, "bias_field.nii"); header=mag_nii.header)
        savenii(vol_out, joinpath(steps_dir, "homogeneous.nii"); header=mag_nii.header)
    end

    println("makehomogeneous completed successfully")
end

main()
