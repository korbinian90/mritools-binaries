#!/usr/bin/env julia
# Run CLEARSWI via Julia CLEARSWI.jl package
#
# Usage:
#   julia --project=test/compare/julia run_clearswi.jl \
#       --magnitude Mag.nii --phase Phase.nii --output clearswi.nii \
#       [--echo-times 4 8 12] [--no-rescale] [--mip-slices 7]

using CLEARSWI
using MriResearchTools
using NIfTI
using ArgParse

function parse_args()
    s = ArgParseSettings(description="Run CLEARSWI via Julia for comparison testing")
    @add_arg_table! s begin
        "--magnitude", "-m"
            help = "Magnitude NIfTI file (required)"
            required = true
        "--phase", "-p"
            help = "Phase NIfTI file"
            default = nothing
        "--output", "-o"
            help = "Output SWI file"
            default = "clearswi.nii"
        "--echo-times", "-t"
            help = "Echo times in ms"
            nargs = '*'
            arg_type = Float64
        "--no-rescale"
            help = "Do not rescale phase to [-π, π]"
            action = :store_true
        "--mip-slices", "-s"
            help = "Number of slices for MIP"
            arg_type = Int
            default = 7
        "--unwrapping-algorithm"
            help = "Unwrapping algorithm: laplacian, romeo"
            default = "laplacian"
        "--phase-scaling-type"
            help = "Phase scaling: tanh, negativetanh, positive, negative, triangular"
            default = "tanh"
        "--phase-scaling-strength"
            help = "Phase scaling strength"
            arg_type = Float64
            default = 4.0
        "--filter-size"
            help = "High-pass filter size"
            nargs = 3
            arg_type = Float64
            default = [4.0, 4.0, 0.0]
        "--mag-combine"
            help = "Magnitude combination: SNR, average"
            default = "SNR"
        "--mag-sensitivity-correction"
            help = "Sensitivity correction: on, off"
            default = "on"
    end
    return ArgParse.parse_args(s)
end

function main()
    args = parse_args()

    # Load magnitude
    mag_nii = niread(args["magnitude"])
    mag_data = Float64.(mag_nii.raw)

    # Load phase
    phase_data = if args["phase"] !== nothing
        phase_nii = niread(args["phase"])
        pdata = Float64.(phase_nii.raw)

        # Rescale phase to [-π, π] if not disabled
        if !args["no-rescale"]
            for echo in 1:size(pdata, 4)
                vol = @view pdata[:, :, :, echo]
                mn, mx = extrema(vol)
                if abs(mx - mn) > 1e-10
                    vol .= (vol .- mn) ./ (mx - mn) .* 2π .- π
                end
            end
        end
        pdata
    else
        nothing
    end

    # Echo times
    TEs = if !isempty(args["echo-times"])
        args["echo-times"]
    else
        collect(1.0:size(mag_data, 4))
    end

    println("Running CLEARSWI.jl...")
    println("  Magnitude shape: ", size(mag_data))
    if phase_data !== nothing
        println("  Phase shape: ", size(phase_data))
    end
    println("  Echo times: ", TEs)

    # Create CLEARSWI options
    # Map unwrapping algorithm
    unwrap_alg = if args["unwrapping-algorithm"] == "romeo"
        :romeo
    else
        :laplacian
    end

    # Map phase scaling
    phase_scaling = Symbol(args["phase-scaling-type"])

    # Build keyword arguments
    kwargs = Dict{Symbol,Any}()
    kwargs[:TEs] = TEs
    kwargs[:unwrapping] = unwrap_alg
    kwargs[:phase_scaling_type] = phase_scaling
    kwargs[:phase_scaling_strength] = args["phase-scaling-strength"]
    kwargs[:filter_size] = args["filter-size"]
    kwargs[:mag_combine] = Symbol(args["mag-combine"])
    kwargs[:sensitivity] = args["mag-sensitivity-correction"] == "on"

    # Run CLEARSWI
    if phase_data !== nothing
        swi = clearswi(mag_data, phase_data; kwargs...)
    else
        swi = clearswi(mag_data; kwargs...)
    end

    # Save output
    output_path = args["output"]
    mkpath(dirname(abspath(output_path)))
    savenii(swi, output_path; header=mag_nii.header)
    println("  Saved: ", output_path)

    # MIP
    mip_path = replace(output_path, r"\.nii(\.gz)?$" => "") * "_mip.nii"
    if ndims(swi) >= 3
        mip = CLEARSWI.create_mip(swi; slices=args["mip-slices"])
        savenii(mip, mip_path; header=mag_nii.header)
        println("  Saved MIP: ", mip_path)
    end

    println("CLEARSWI.jl completed successfully")
end

main()
