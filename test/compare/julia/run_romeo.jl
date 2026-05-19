#!/usr/bin/env julia
# Run ROMEO phase unwrapping via Julia ROMEO.jl package
#
# Usage:
#   julia --project=test/compare/julia run_romeo.jl \
#       --phase Phase.nii --output unwrapped.nii \
#       [--magnitude Mag.nii] [--echo-times 4 8 12] \
#       [--no-rescale] [--compute-B0]

using ROMEO
using MriResearchTools
using NIfTI
using ArgParse

function parse_args()
    s = ArgParseSettings(description="Run ROMEO via Julia for comparison testing")
    @add_arg_table! s begin
        "--phase", "-p"
            help = "Phase NIfTI file (required)"
            required = true
        "--magnitude", "-m"
            help = "Magnitude NIfTI file"
            default = nothing
        "--output", "-o"
            help = "Output unwrapped phase file"
            default = "unwrapped.nii"
        "--echo-times", "-t"
            help = "Echo times in ms"
            nargs = '*'
            arg_type = Float64
        "--no-rescale"
            help = "Do not rescale phase to [-π, π]"
            action = :store_true
        "--compute-B0", "-B"
            help = "Compute B0 map"
            action = :store_true
        "--individual"
            help = "Unwrap echoes individually"
            action = :store_true
        "--phase-offset-correction"
            help = "Phase offset correction: on, off, bipolar"
            default = "on"
        "--template"
            help = "Template echo number (1-based)"
            arg_type = Int
            default = 1
        "--weights", "-w"
            help = "Weight type: romeo, romeo2, romeo3, romeo4, romeo6, bestpath"
            default = "romeo"
        "--writesteps"
            help = "Directory to write canonical intermediate NIfTIs to"
            default = nothing
    end
    return ArgParse.parse_args(s)
end

function main()
    args = parse_args()

    # Setup writesteps dir
    steps_dir = args["writesteps"]
    if steps_dir !== nothing
        mkpath(steps_dir)
    end

    # Load phase
    phase_nii = niread(args["phase"])
    phase_data = Float64.(phase_nii.raw)

    # Rescale phase to [-π, π] if not disabled (matches Rust default behavior)
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

    # Canonical step: phase_rescaled (post-rescale, pre-processing)
    if steps_dir !== nothing
        savenii(phase_data, joinpath(steps_dir, "phase_rescaled.nii"); header=phase_nii.header)
    end

    # Load magnitude
    mag_data = if args["magnitude"] !== nothing
        mag_nii = niread(args["magnitude"])
        Float64.(mag_nii.raw)
    else
        nothing
    end

    # Echo times
    TEs = if !isempty(args["echo-times"])
        args["echo-times"]
    else
        collect(1.0:size(phase_data, 4))
    end

    # ROMEO.jl's `unwrap!` does NOT apply MCPC-3D-S phase offset correction
    # itself — only the CompileMRI.jl / RomeoApp CLI wrapper does, which is
    # what the Rust port mirrors. To match that workflow, apply mcpc3ds
    # explicitly here when multi-echo + phase_offset_correction != off,
    # then hand the corrected phase to romeo.
    poc = args["phase-offset-correction"]
    bipolar = poc == "bipolar"

    if ndims(phase_data) == 4 && size(phase_data, 4) >= 2 && poc != "off" && mag_data !== nothing
        # MriResearchTools.mcpc3ds expects (phase, mag) positionally and
        # returns a PhaseMag struct when both are passed.
        corrected = mcpc3ds(phase_data, mag_data; TEs=TEs, bipolar_correction=bipolar)
        phase_data = isa(corrected, MriResearchTools.PhaseMag) ? corrected.phase : corrected
        println("  applied MCPC-3D-S phase offset correction (bipolar=$bipolar)")
        if steps_dir !== nothing
            savenii(phase_data, joinpath(steps_dir, "phase_corrected.nii"); header=phase_nii.header)
        end
    end

    # Build keyword arguments for ROMEO
    kwargs = Dict{Symbol,Any}()
    if mag_data !== nothing
        kwargs[:mag] = mag_data
    end
    kwargs[:TEs] = TEs

    if args["individual"]
        kwargs[:individual] = true
    end
    kwargs[:template] = args["template"]
    kwargs[:weights] = Symbol(args["weights"])

    # Pass :off so romeo does not re-apply any internal correction
    # (the kwarg only feeds into calculateweights downstream anyway).
    kwargs[:phase_offset_correction] = :off

    # Run ROMEO unwrapping
    println("Running ROMEO.jl unwrapping...")
    println("  Phase shape: ", size(phase_data))
    println("  Echo times: ", TEs)

    unwrapped = romeo(phase_data; kwargs...)

    # Save output
    output_path = args["output"]
    mkpath(dirname(abspath(output_path)))
    savenii(unwrapped, output_path; header=phase_nii.header)
    println("  Saved: ", output_path)

    # Canonical unwrapped steps
    if steps_dir !== nothing
        if ndims(unwrapped) == 4
            for i in 1:size(unwrapped, 4)
                savenii(unwrapped[:, :, :, i],
                        joinpath(steps_dir, "unwrapped_echo_$(i).nii");
                        header=phase_nii.header)
            end
            savenii(unwrapped, joinpath(steps_dir, "unwrapped.nii"); header=phase_nii.header)
        else
            savenii(unwrapped, joinpath(steps_dir, "unwrapped_echo_1.nii"); header=phase_nii.header)
            savenii(unwrapped, joinpath(steps_dir, "unwrapped.nii"); header=phase_nii.header)
        end

        # Per-voxel quality map (matches romeo's --write-quality / Rust `quality.nii`).
        # Built from the same calculateweights call ROMEO uses for unwrapping.
        if ndims(phase_data) == 4 && size(phase_data, 4) >= 2
            qkwargs = Dict{Symbol,Any}(:TEs => TEs)
            if mag_data !== nothing
                qkwargs[:mag] = mag_data
            end
            qmap = voxelquality(phase_data; qkwargs...)
            savenii(Float64.(qmap), joinpath(steps_dir, "quality.nii"); header=phase_nii.header)
        end
    end

    # B0 computation
    if args["compute-B0"]
        b0_path = joinpath(dirname(abspath(output_path)), "B0.nii")
        if ndims(unwrapped) == 4 && length(TEs) >= 2
            # calculateB0_unwrapped(unwrapped_phase, mag, TEs); use ones if no mag
            b0_mag = mag_data !== nothing ? mag_data : ones(eltype(unwrapped), size(unwrapped))
            b0 = calculateB0_unwrapped(unwrapped, b0_mag, TEs)
            savenii(b0, b0_path; header=phase_nii.header)
            println("  Saved B0: ", b0_path)
            if steps_dir !== nothing
                savenii(b0, joinpath(steps_dir, "b0.nii"); header=phase_nii.header)
            end
        else
            println("  Warning: B0 requires multi-echo data, skipping")
        end
    end

    println("ROMEO.jl completed successfully")
end

main()
