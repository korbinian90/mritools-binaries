#!/usr/bin/env julia
# Run romeo_mask via Julia ROMEO.jl package
#
# Usage:
#   julia --project=test/compare/julia run_romeo_mask.jl \
#       --phase Phase.nii --output mask.nii \
#       [--magnitude Mag.nii] [--echo-times 4 8 12] \
#       [--factor 0.1] [--no-rescale]

using ROMEO
using MriResearchTools
using NIfTI
using ArgParse

function parse_args()
    s = ArgParseSettings(description="Run romeo_mask via Julia for comparison testing")
    @add_arg_table! s begin
        "--phase", "-p"
            help = "Phase NIfTI file (required)"
            required = true
        "--magnitude", "-m"
            help = "Magnitude NIfTI file"
            default = nothing
        "--output", "-o"
            help = "Output mask file"
            default = "mask.nii"
        "--echo-times", "-t"
            help = "Echo times in ms"
            nargs = '*'
            arg_type = Float64
        "--factor", "-f"
            help = "Masking threshold factor [0, 1]"
            arg_type = Float64
            default = 0.1
        "--no-rescale"
            help = "Do not rescale phase to [-π, π]"
            action = :store_true
        "--weights", "-w"
            help = "Weight type: romeo, romeo2, romeo3, romeo4, romeo6, bestpath"
            default = "romeo"
        "--write-quality", "-q"
            help = "Write quality map"
            action = :store_true
    end
    return ArgParse.parse_args(s)
end

function main()
    args = parse_args()

    # Load phase
    phase_nii = niread(args["phase"])
    phase_data = Float64.(phase_nii.raw)

    # Rescale phase to [-π, π] if not disabled
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

    println("Running romeo_mask via ROMEO.jl...")
    println("  Phase shape: ", size(phase_data))
    println("  Echo times: ", TEs)
    println("  Factor: ", args["factor"])
    println("  Weights: ", args["weights"])

    # Build keyword arguments
    kwargs = Dict{Symbol,Any}()
    if mag_data !== nothing
        kwargs[:mag] = mag_data
    end
    kwargs[:TEs] = TEs
    kwargs[:threshold] = args["factor"]
    kwargs[:weights] = Symbol(args["weights"])

    # Use first echo for masking (matching Rust behavior)
    phase_for_mask = if ndims(phase_data) == 4
        phase_data[:, :, :, 1]
    else
        phase_data
    end

    # Run ROMEO mask generation
    mask = create_mask(phase_for_mask; kwargs...)

    # Save output
    output_path = args["output"]
    mkpath(dirname(abspath(output_path)))
    savenii(Float64.(mask), output_path; header=phase_nii.header)
    println("  Saved: ", output_path)

    # Quality map
    if args["write-quality"]
        quality_path = replace(output_path, r"\.nii(\.gz)?$" => "") * "_quality.nii"
        # Note: Quality map extraction depends on ROMEO.jl API
        println("  Note: Quality map writing depends on ROMEO.jl API version")
    end

    println("romeo_mask completed successfully")
end

main()
