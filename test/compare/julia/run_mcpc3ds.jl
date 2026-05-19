#!/usr/bin/env julia
# Run MCPC-3D-S phase combination via Julia MriResearchTools.jl
#
# Usage:
#   julia --project=test/compare/julia run_mcpc3ds.jl \
#       --phase Phase.nii --output output.nii \
#       [--magnitude Mag.nii] [--echo-times 4 8 12] \
#       [--smoothing-sigma 10 10 5] [--bipolar] [--no-rescale]

using MriResearchTools
using NIfTI
using ArgParse

function parse_args()
    s = ArgParseSettings(description="Run MCPC-3D-S via Julia for comparison testing")
    @add_arg_table! s begin
        "--phase", "-p"
            help = "Phase NIfTI file (required)"
            required = true
        "--magnitude", "-m"
            help = "Magnitude NIfTI file"
            default = nothing
        "--output", "-o"
            help = "Output corrected phase file"
            default = "output.nii"
        "--echo-times", "-t"
            help = "Echo times in ms"
            nargs = '*'
            arg_type = Float64
        "--smoothing-sigma", "-s"
            help = "Gaussian smoothing sigma in voxels"
            nargs = 3
            arg_type = Float64
            default = [10.0, 10.0, 5.0]
        "--bipolar", "-b"
            help = "Remove eddy current artefacts"
            action = :store_true
        "--no-rescale"
            help = "Do not rescale phase to [-π, π]"
            action = :store_true
        "--write-phase-offsets"
            help = "Save estimated phase offsets"
            action = :store_true
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

    # Load phase
    phase_nii = niread(args["phase"])
    phase_data = Float64.(phase_nii.raw)

    # Dump input phases before any processing
    if steps_dir !== nothing
        savenii(phase_data, joinpath(steps_dir, "input_phases.nii"); header=phase_nii.header)
    end

    # Rescale phase to [-π, π] if not disabled
    if !args["no-rescale"]
        if ndims(phase_data) == 4
            for echo in 1:size(phase_data, 4)
                vol = @view phase_data[:, :, :, echo]
                mn, mx = extrema(vol)
                if abs(mx - mn) > 1e-10
                    vol .= (vol .- mn) ./ (mx - mn) .* 2π .- π
                end
            end
        else
            mn, mx = extrema(phase_data)
            if abs(mx - mn) > 1e-10
                phase_data .= (phase_data .- mn) ./ (mx - mn) .* 2π .- π
            end
        end
    end

    # Load magnitude
    mag_data = if args["magnitude"] !== nothing
        mag_nii = niread(args["magnitude"])
        Float64.(mag_nii.raw)
    else
        ones(size(phase_data))
    end

    # Echo times
    TEs = if !isempty(args["echo-times"])
        args["echo-times"]
    else
        collect(1.0:size(phase_data, 4))
    end

    println("Running MCPC-3D-S via MriResearchTools.jl...")
    println("  Phase shape: ", size(phase_data))
    println("  Echo times: ", TEs)
    println("  Smoothing sigma: ", args["smoothing-sigma"])
    println("  Bipolar: ", args["bipolar"])

    # Build keyword arguments
    kwargs = Dict{Symbol,Any}()
    kwargs[:TEs] = TEs
    kwargs[:sigma] = args["smoothing-sigma"]
    if args["bipolar"]
        kwargs[:bipolar_correction] = true
    end

    # Run MCPC-3D-S
    # MriResearchTools.mcpc3ds expects (phase, mag) positionally; `mag` is not a kwarg.
    combined = mcpc3ds(phase_data, mag_data; kwargs...)

    # `combined` is a PhaseMag struct when called with (phase, mag); we save only
    # the corrected phase to match the Rust binary output.
    combined_phase = isa(combined, MriResearchTools.PhaseMag) ? combined.phase : combined

    # Save output
    output_path = args["output"]
    if !endswith(output_path, ".nii") && !endswith(output_path, ".nii.gz")
        output_path *= ".nii"
    end
    mkpath(dirname(abspath(output_path)))
    savenii(combined_phase, output_path; header=phase_nii.header)
    println("  Saved: ", output_path)

    # Canonical corrected-phase step (bipolar-aware, matches Rust side)
    if steps_dir !== nothing
        savenii(combined_phase, joinpath(steps_dir, "corrected.nii"); header=phase_nii.header)
        if args["bipolar"]
            savenii(combined_phase, joinpath(steps_dir, "corrected_bipolar.nii"); header=phase_nii.header)
        end
    end

    # Phase offsets
    if args["write-phase-offsets"]
        offset_path = replace(output_path, r"\.nii(\.gz)?$" => "") * "_phase_offset.nii"
        # Re-run to get offsets separately if the API supports it
        println("  Note: Phase offset writing depends on MriResearchTools.jl API")
    end

    println("MCPC-3D-S completed successfully")
end

main()
