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
    end
    return ArgParse.parse_args(s)
end

function main()
    args = parse_args()

    # Load phase
    phase_nii = niread(args["phase"])
    phase_data = Float64.(phase_nii.raw)

    # Rescale phase to [-π, π] if not disabled (matches Rust default behavior)
    if !args["no-rescale"]
        for echo in 1:size(phase_data, 4)
            vol = @view phase_data[:, :, :, echo]
            mn, mx = extrema(vol)
            if abs(mx - mn) > 1e-10
                vol .= (vol .- mn) ./ (mx - mn) .* 2π .- π
            end
        end
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

    if args["phase-offset-correction"] == "off"
        kwargs[:phase_offset_correction] = :off
    elseif args["phase-offset-correction"] == "bipolar"
        kwargs[:phase_offset_correction] = :bipolar
    else
        kwargs[:phase_offset_correction] = :on
    end

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

    # B0 computation
    if args["compute-B0"]
        b0_path = replace(output_path, r"\.nii(\.gz)?$" => "") * "_B0.nii"
        if ndims(unwrapped) == 4 && length(TEs) >= 2
            b0 = calculateB0_unwrapped(unwrapped, TEs)
            savenii(b0, b0_path; header=phase_nii.header)
            println("  Saved B0: ", b0_path)
        else
            println("  Warning: B0 requires multi-echo data, skipping")
        end
    end

    println("ROMEO.jl completed successfully")
end

main()
