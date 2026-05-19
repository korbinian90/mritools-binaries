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
        "--qsm"
            help = "Use TGV QSM for phase weighting"
            action = :store_true
        "--qsm-mask"
            help = "Mask for QSM (NIfTI file)"
            default = nothing
        "--writesteps"
            help = "Directory to write canonical intermediate NIfTIs to"
            default = nothing
    end
    return ArgParse.parse_args(s)
end

# Map CLEARSWI.jl's writesteps file names to the Rust canonical step
# names so test/compare/run_comparison.sh can diff matching pairs.
const CLEARSWI_STEP_RENAME = Dict(
    "combined_mag.nii"               => "mag_combined.nii",
    "sensitivity_corrected_mag.nii"  => "mag_corrected.nii",
    "sensitivity.nii"                => "sensitivity.nii",
    "maskforphase.nii"               => "phase_mask.nii",
    "unwrappedphase.nii"             => "phase_unwrapped.nii",
    "combinedphase.nii"              => "phase_combined.nii",
)

function main()
    args = parse_args()

    steps_dir = args["writesteps"]
    if steps_dir !== nothing
        mkpath(steps_dir)
    end

    # Load magnitude
    mag_nii = niread(args["magnitude"])
    mag_data = Float64.(mag_nii.raw)

    # Load phase
    phase_data = if args["phase"] !== nothing
        phase_nii = niread(args["phase"])
        pdata = Float64.(phase_nii.raw)

        # Rescale phase to [-π, π] if not disabled
        if !args["no-rescale"]
            if ndims(pdata) == 4
                for echo in 1:size(pdata, 4)
                    vol = @view pdata[:, :, :, echo]
                    mn, mx = extrema(vol)
                    if abs(mx - mn) > 1e-10
                        vol .= (vol .- mn) ./ (mx - mn) .* 2π .- π
                    end
                end
            else
                mn, mx = extrema(pdata)
                if abs(mx - mn) > 1e-10
                    pdata .= (pdata .- mn) ./ (mx - mn) .* 2π .- π
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

    # CLEARSWI.jl public API: calculateSWI(Data, Options).
    # `Data` carries the NIfTI header so per-step `savenii` calls inside the
    # package have a header to attach.
    if phase_data === nothing
        error("CLEARSWI.jl requires both magnitude and phase inputs")
    end

    # Build the QSM mask if --qsm-mask was supplied.
    qsm_mask = if args["qsm-mask"] !== nothing
        Bool.(niread(args["qsm-mask"]).raw .!= 0)
    else
        nothing
    end

    data = Data(mag_data, phase_data, mag_nii.header, TEs)
    options = Options(;
        mag_combine = Symbol(args["mag-combine"]),
        mag_sens = args["mag-sensitivity-correction"] == "off" ? [1] : nothing,
        phase_unwrap = Symbol(args["unwrapping-algorithm"]),
        phase_hp_sigma = args["filter-size"],
        phase_scaling_type = Symbol(args["phase-scaling-type"]),
        phase_scaling_strength = args["phase-scaling-strength"],
        qsm = args["qsm"],
        qsm_mask = qsm_mask,
        # CLEARSWI emits its own intermediates under writesteps; we rename
        # them below to match the Rust canonical basenames.
        writesteps = steps_dir,
    )
    swi = calculateSWI(data, options)

    # Save output
    output_path = args["output"]
    mkpath(dirname(abspath(output_path)))
    savenii(swi, output_path; header=mag_nii.header)
    println("  Saved: ", output_path)

    # Canonical intermediate dumps: final SWI and MIP, plus renames of
    # CLEARSWI.jl's own writesteps output to match the Rust naming.
    if steps_dir !== nothing
        savenii(swi, joinpath(steps_dir, "swi.nii"); header=mag_nii.header)
        for (julia_name, rust_name) in CLEARSWI_STEP_RENAME
            src = joinpath(steps_dir, julia_name)
            dst = joinpath(steps_dir, rust_name)
            if isfile(src) && src != dst
                mv(src, dst; force=true)
            end
        end
    end

    # MIP
    mip_path = replace(output_path, r"\.nii(\.gz)?$" => "") * "_mip.nii"
    if ndims(swi) >= 3
        mip = createMIP(swi, args["mip-slices"])
        savenii(mip, mip_path; header=mag_nii.header)
        println("  Saved MIP: ", mip_path)
        if steps_dir !== nothing
            savenii(mip, joinpath(steps_dir, "mip.nii"); header=mag_nii.header)
        end
    end

    println("CLEARSWI.jl completed successfully")
end

main()
