#!/usr/bin/env julia
# Generate a synthetic multi-echo GRE phantom for parity testing.
#
# Produces Mag.nii and Phase.nii (128×128×96 × 3 echoes, float32) with
# the properties the algorithms in this repo actually expect:
#   - bright brain-like ellipsoid signal on a low-intensity noise floor
#     (so robustmask has a real noise corner to threshold against);
#   - exponential T2* decay across echoes;
#   - smooth spatial phase variation (B0 inhomogeneity-like) that
#     evolves linearly with TE, with multi-echo wrap structure;
#   - magnitude-modulated noise on both channels.
#
# Output:
#   test/data/extra/Mag.nii
#   test/data/extra/Phase.nii
#   TEs are [4.0, 8.0, 12.0] ms — pass `--echo-times 4 8 12` to the harness.

using NIfTI
using Random

Random.seed!(20260519)

# Geometry --------------------------------------------------------------
nx, ny, nz = 128, 128, 96
nt = 3
voxel_size = (1.0f0, 1.0f0, 2.0f0)  # mm
TEs = [4.0, 8.0, 12.0]              # ms
T2star = 25.0                        # ms (tissue average)

# Signal mask: brain-like prolate ellipsoid centred in the FOV --------
cx, cy, cz = nx/2, ny/2, nz/2
rx, ry, rz = 48.0, 56.0, 36.0   # half-axes in voxels

signal = zeros(Float32, nx, ny, nz)
b0_map = zeros(Float32, nx, ny, nz)   # B0 in Hz
for k in 1:nz, j in 1:ny, i in 1:nx
    r2 = ((i-cx)/rx)^2 + ((j-cy)/ry)^2 + ((k-cz)/rz)^2
    if r2 <= 1.0
        # Tissue intensity varies slightly with location to mimic GM/WM mix.
        intensity = 1.0f0 - 0.3f0 * Float32(r2)
        # Add a deeper-intensity "lesion" blob off-centre to give the
        # phase something to track.
        d2 = ((i-cx-15)/8)^2 + ((j-cy-10)/8)^2 + ((k-cz)/8)^2
        if d2 < 1.0
            intensity *= 0.6f0
        end
        signal[i,j,k] = intensity
        # B0 field: smooth quadratic with localised dipole at the lesion.
        b0_map[i,j,k] = Float32(
            8.0 * ((i-cx)/nx)
            + 6.0 * ((j-cy)/ny)^2
            + 60.0 * exp(-d2)        # dipole-like local field
        )
    end
end

# Magnitude with T2* decay + Rician-like noise floor outside --------
mag = zeros(Float32, nx, ny, nz, nt)
for e in 1:nt
    decay = Float32(exp(-TEs[e] / T2star))
    for k in 1:nz, j in 1:ny, i in 1:nx
        s = signal[i,j,k] * decay
        # In-mask: signal + small magnitude-proportional noise.
        # Out-of-mask: low Gaussian noise floor (gives robustmask a real corner).
        if s > 0
            noise = 0.02f0 * randn(Float32) * s
            mag[i,j,k,e] = max(0.0f0, s + noise)
        else
            mag[i,j,k,e] = abs(0.015f0 * randn(Float32))  # noise floor ~0.01
        end
    end
end

# Phase: wraps deliberately at TE3 to exercise the temporal unwrap ----
phase = zeros(Float32, nx, ny, nz, nt)
phase_offset_static = Float32.(
    0.4 * sin.(range(0, 4π, length=nx)) .* ones(1, ny, nz)
    .+ 0.3 * cos.(reshape(range(0, 2π, length=ny), 1, ny)) .* ones(nx, 1, nz)
)
for e in 1:nt
    omega = 2π * b0_map / 1000.0       # rad/ms
    raw_phase = omega * Float32(TEs[e]) + phase_offset_static
    # Wrap to (-π, π] — what the scanner would actually record.
    phase[:,:,:,e] = mod.(raw_phase .+ Float32(π), Float32(2π)) .- Float32(π)
    # Add small phase noise scaled inversely by SNR (mag / noise level).
    for k in 1:nz, j in 1:ny, i in 1:nx
        snr_factor = mag[i,j,k,e] / 0.015f0 + 1.0f0
        phase[i,j,k,e] += randn(Float32) * (0.05f0 / sqrt(snr_factor))
    end
    # Re-wrap after noise.
    phase[:,:,:,e] = mod.(phase[:,:,:,e] .+ Float32(π), Float32(2π)) .- Float32(π)
end

# Write NIfTIs ----------------------------------------------------------
this_dir = abspath(dirname(@__FILE__))

function write_nifti(path, data, voxel_size)
    nii = NIVolume(data; voxel_size=voxel_size)
    niwrite(path, nii)
    println("  wrote ", path, "  ", size(data))
end

write_nifti(joinpath(this_dir, "Mag.nii"),   mag,   voxel_size)
write_nifti(joinpath(this_dir, "Phase.nii"), phase, voxel_size)

println()
println("Phantom summary:")
println("  Magnitude — min=$(minimum(mag)) max=$(maximum(mag)) mean=$(sum(mag)/length(mag))")
println("  Phase     — min=$(minimum(phase)) max=$(maximum(phase))")
println("  TEs (ms):  $TEs")
println("  Echo-times CLI arg: --echo-times $(join(TEs, ' '))")
