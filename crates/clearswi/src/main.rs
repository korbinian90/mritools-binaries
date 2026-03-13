//! CLEAR-SWI — Susceptibility Weighted Imaging CLI
//!
//! Matches the Julia CLI interface from korbinian90/CLEARSWI.jl.
//!
//! Reference:
//!   Eckstein, K., et al. (2024). "CLEAR-SWI: Computational Efficient T2* Weighted Imaging."
//!   Proc. ISMRM.

use anyhow::{Context, Result};
use clap::Parser;
use mritools_common::{
    fix_ge_phase_slices, parse_echo_selection, parse_echo_times, read_nifti_4d, save_settings,
    select_echo_times, select_volumes, write_nifti, write_nifti_from_4d, NiftiData, NiftiData4D,
};
use qsm_core::inversion::tgv::{tgv_qsm, TgvParams};
use qsm_core::region_grow::grow_region_unwrap;
use qsm_core::swi::{calculate_swi, create_mip, softplus_scaling, PhaseScaling};
use qsm_core::unwrap::laplacian::laplacian_unwrap;
use qsm_core::unwrap::romeo::calculate_weights_romeo;
use qsm_core::utils::get_sensitivity;

/// CLEAR-SWI susceptibility weighted imaging.
///
/// Computes SWI images from magnitude and phase NIfTI inputs. Matches the Julia
/// CLEAR-SWI CLI interface from korbinian90/CLEARSWI.jl.
#[derive(Parser, Debug)]
#[command(
    name = "clearswi",
    about = "CLEAR-SWI susceptibility weighted imaging",
    version
)]
struct Cli {
    /// The magnitude image (single or multi-echo)
    #[arg(short = 'm', long)]
    magnitude: Option<String>,

    /// The phase image (single or multi-echo)
    #[arg(short = 'p', long)]
    phase: Option<String>,

    /// The output path or filename [default: clearswi.nii]
    #[arg(short = 'o', long, default_value = "clearswi.nii")]
    output: String,

    /// Echo times in [ms]: "[1.5,3.0]" | "3.5:3.5:14"
    #[arg(short = 't', long = "echo-times", num_args = 1..)]
    echo_times: Vec<String>,

    /// Number of slices in the MIP image [default: 7]
    #[arg(short = 's', long, default_value = "7")]
    mip_slices: String,

    /// Use TGV QSM for phase weighting
    #[arg(long)]
    qsm: bool,

    /// Pre-calculated QSM input instead of phase
    #[arg(long)]
    qsm_input: Option<String>,

    /// Mask for QSM
    #[arg(long)]
    qsm_mask: Option<String>,

    /// Magnitude combination: SNR | average | echo <n> | SE <te>
    #[arg(long = "mag-combine", num_args = 1.., default_values = &["SNR"])]
    mag_combine: Vec<String>,

    /// CLEAR-SWI sensitivity correction: <filename> | on | off
    #[arg(long = "mag-sensitivity-correction", default_value = "on")]
    mag_sensitivity_correction: String,

    /// Softplus scaling of magnitude: on | off
    #[arg(long = "mag-softplus-scaling", default_value = "on")]
    mag_softplus_scaling: String,

    /// Unwrapping algorithm: laplacian | romeo | laplacianslice
    #[arg(long, default_value = "laplacian")]
    unwrapping_algorithm: String,

    /// High-pass phase filter size in voxels: <x> <y> <z>
    #[arg(long = "filter-size", num_args = 1.., default_values = &["[4,4,0]"])]
    filter_size: Vec<String>,

    /// Phase scaling type: tanh | negativetanh | positive | negative | triangular
    #[arg(long, default_value = "tanh")]
    phase_scaling_type: String,

    /// Phase scaling strength [default: 4]
    #[arg(long, default_value = "4")]
    phase_scaling_strength: String,

    /// Load only the specified echoes from disk
    #[arg(short = 'e', long, num_args = 1.., default_values = &[":"])]
    echoes: Vec<String>,

    /// Deactivate automatic rescaling of phase images to [-π;π]
    #[arg(long)]
    no_phase_rescale: bool,

    /// Fix GE phase slice-jump artefacts
    #[arg(long)]
    fix_ge_phase: bool,

    /// Save intermediate steps to this folder
    #[arg(long)]
    writesteps: Option<String>,

    /// Verbose output
    #[arg(short = 'v', long)]
    verbose: bool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let use_qsm = cli.qsm || cli.qsm_input.is_some();

    let magnitude = cli
        .magnitude
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("--magnitude / -m is required"))?;

    if cli.verbose {
        eprintln!("CLEAR-SWI");
        eprintln!("  magnitude: {}", magnitude);
        if let Some(ref p) = cli.phase {
            eprintln!("  phase:     {}", p);
        }
        eprintln!("  output:    {}", cli.output);
    }

    let output_dir = std::path::Path::new(&cli.output)
        .parent()
        .and_then(|p| p.to_str())
        .unwrap_or(".");
    let output_dir = if output_dir.is_empty() {
        "."
    } else {
        output_dir
    };

    std::fs::create_dir_all(output_dir)
        .with_context(|| format!("Cannot create output directory '{}'", output_dir))?;

    let args: Vec<String> = std::env::args().collect();
    save_settings(output_dir, "clearswi", &args)?;

    // Parse echo times
    let mut echo_times =
        parse_echo_times(&cli.echo_times).context("Failed to parse --echo-times")?;

    // Load magnitude image (4D)
    let mut mag_4d = read_nifti_4d(magnitude)
        .with_context(|| format!("Failed to read magnitude image '{}'", magnitude))?;

    // Apply echo selection (filter volumes and echo times with same indices)
    if let Some(sel) = parse_echo_selection(&cli.echoes, mag_4d.nt) {
        if cli.verbose {
            eprintln!(
                "  selecting echoes: {:?} (1-based)",
                sel.iter().map(|i| i + 1).collect::<Vec<_>>()
            );
        }
        mag_4d = select_volumes(&mag_4d, &sel);
        if !echo_times.is_empty() {
            echo_times = select_echo_times(&echo_times, &sel);
        }
    }

    let (nx, ny, nz) = mag_4d.dims;
    let (vsx, vsy, vsz) = mag_4d.voxel_size;
    let n_voxels = nx * ny * nz;

    if cli.verbose {
        eprintln!("  dims: {}x{}x{}, {} echoes", nx, ny, nz, mag_4d.nt);
        eprintln!("  voxel size: {:.3}x{:.3}x{:.3} mm", vsx, vsy, vsz);
    }

    if mag_4d.volumes.is_empty() {
        anyhow::bail!("Magnitude image contains no volumes");
    }

    // Combine magnitude echoes based on --mag-combine method
    let mag_combined: Vec<f64> = combine_magnitude(&mag_4d, &cli.mag_combine, &echo_times);

    // Build mask from combined magnitude
    let mask = robust_mask(&mag_combined);

    // Apply magnitude sensitivity correction
    let mag_corrected: Vec<f64> = match cli.mag_sensitivity_correction.as_str() {
        "off" => mag_combined.clone(),
        "on" => {
            let sensitivity = get_sensitivity(&mag_combined, nx, ny, nz, vsx, vsy, vsz, 7.0, 15);
            let mut corrected = vec![0.0; n_voxels];
            for i in 0..n_voxels {
                if sensitivity[i] > 1e-10 {
                    corrected[i] = mag_combined[i] / sensitivity[i];
                } else {
                    corrected[i] = mag_combined[i];
                }
            }
            corrected
        }
        path => {
            // Load sensitivity from file
            if let Ok(sens_4d) = read_nifti_4d(path) {
                if sens_4d.volumes.len() != 1 {
                    eprintln!(
                        "WARNING: sensitivity file '{}' must contain exactly one volume (found {}), skipping correction",
                        path,
                        sens_4d.volumes.len()
                    );
                    mag_combined.clone()
                } else {
                    let sensitivity = &sens_4d.volumes[0];
                    if sensitivity.len() != n_voxels {
                        eprintln!(
                            "WARNING: sensitivity file '{}' has {} voxels, but magnitude image has {}, skipping correction",
                            path,
                            sensitivity.len(),
                            n_voxels
                        );
                        mag_combined.clone()
                    } else {
                        let mut corrected = vec![0.0; n_voxels];
                        for i in 0..n_voxels {
                            if sensitivity[i] > 1e-10 {
                                corrected[i] = mag_combined[i] / sensitivity[i];
                            } else {
                                corrected[i] = mag_combined[i];
                            }
                        }
                        corrected
                    }
                }
            } else {
                eprintln!(
                    "WARNING: could not load sensitivity file '{}', skipping correction",
                    path
                );
                mag_combined.clone()
            }
        }
    };

    // Setup writesteps directory if requested
    let writesteps_dir = cli.writesteps.as_deref();
    if let Some(dir) = writesteps_dir {
        std::fs::create_dir_all(dir)
            .with_context(|| format!("Cannot create writesteps directory '{}'", dir))?;

        // Save combined magnitude
        write_step(dir, "mag_combined", &mag_corrected, &mag_4d)?;
    }

    // Get phase data: QSM path or standard unwrap path
    let unwrapped_phase: Vec<f64> = if let Some(ref qsm_input_path) = cli.qsm_input {
        // --qsm-input: load pre-computed QSM directly
        let qsm_4d = read_nifti_4d(qsm_input_path)
            .with_context(|| format!("Failed to read QSM input '{}'", qsm_input_path))?;
        if qsm_4d.dims != mag_4d.dims {
            anyhow::bail!(
                "QSM input dimensions {:?} do not match magnitude dimensions {:?}",
                qsm_4d.dims,
                mag_4d.dims
            );
        }
        if qsm_4d.volumes.is_empty() {
            anyhow::bail!("QSM input contains no volumes");
        }
        if cli.verbose {
            eprintln!("  using pre-computed QSM input: {}", qsm_input_path);
        }
        qsm_4d.volumes[0].clone()
    } else if use_qsm {
        // --qsm: compute TGV-QSM from phase
        let phase_path = cli
            .phase
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("--phase / -p is required when --qsm is used"))?;
        let mut phase_4d = read_nifti_4d(phase_path)
            .with_context(|| format!("Failed to read phase image '{}'", phase_path))?;

        if let Some(sel) = parse_echo_selection(&cli.echoes, phase_4d.nt) {
            phase_4d = select_volumes(&phase_4d, &sel);
        }

        if phase_4d.dims != mag_4d.dims {
            anyhow::bail!(
                "Phase image dimensions {:?} do not match magnitude dimensions {:?}",
                phase_4d.dims,
                mag_4d.dims
            );
        }
        if phase_4d.volumes.is_empty() {
            anyhow::bail!("Phase image contains no volumes");
        }

        let mut phase_data = phase_4d.volumes[0].clone();

        if !cli.no_phase_rescale {
            rescale_phase(&mut phase_data);
        }

        if cli.fix_ge_phase {
            fix_ge_phase_slices(&mut phase_data, nx, ny, nz);
            if cli.verbose {
                eprintln!("  applied GE phase slice-jump correction");
            }
        }

        if let Some(dir) = writesteps_dir {
            write_step(dir, "phase_rescaled", &phase_data, &mag_4d)?;
        }

        // Build QSM mask: use --qsm-mask if provided, otherwise the magnitude mask
        let qsm_mask: Vec<u8> = if let Some(ref mask_path) = cli.qsm_mask {
            let mask_4d = read_nifti_4d(mask_path)
                .with_context(|| format!("Failed to read QSM mask '{}'", mask_path))?;
            if mask_4d.dims != mag_4d.dims {
                anyhow::bail!(
                    "QSM mask dimensions {:?} do not match magnitude dimensions {:?}",
                    mask_4d.dims,
                    mag_4d.dims
                );
            }
            if mask_4d.volumes.is_empty() {
                anyhow::bail!("QSM mask contains no volumes");
            }
            mask_4d.volumes[0]
                .iter()
                .map(|&v| if v > 0.5 { 1u8 } else { 0u8 })
                .collect()
        } else {
            mask.clone()
        };

        // Determine echo time for TGV (first echo time in seconds, default 20ms)
        let te_s: f32 = if !echo_times.is_empty() {
            (echo_times[0] / 1000.0) as f32
        } else {
            0.020
        };

        let tgv_params = TgvParams {
            iterations: 800,
            erosions: 0,
            te: te_s,
            ..TgvParams::default()
        };

        if cli.verbose {
            eprintln!(
                "  TGV-QSM: TE={:.3}ms, α₁={}, α₀={}, {} iterations",
                tgv_params.te * 1000.0,
                tgv_params.alpha1,
                tgv_params.alpha0,
                tgv_params.iterations,
            );
        }

        // Convert phase to f32 for TGV
        let phase_f32: Vec<f32> = phase_data.iter().map(|&v| v as f32).collect();

        let b0_dir = (0.0_f32, 0.0_f32, 1.0_f32);
        let chi = tgv_qsm(
            &phase_f32,
            &qsm_mask,
            nx,
            ny,
            nz,
            vsx as f32,
            vsy as f32,
            vsz as f32,
            &tgv_params,
            b0_dir,
        );

        // Convert back to f64
        let qsm_result: Vec<f64> = chi.iter().map(|&v| v as f64).collect();

        if cli.verbose {
            let qmin = qsm_result.iter().cloned().fold(f64::INFINITY, f64::min);
            let qmax = qsm_result.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            eprintln!("  QSM range: [{:.6}, {:.6}]", qmin, qmax);
        }

        if let Some(dir) = writesteps_dir {
            write_step(dir, "qsm", &qsm_result, &mag_4d)?;
        }

        qsm_result
    } else if let Some(ref phase_path) = cli.phase {
        // Standard phase unwrapping path
        let mut phase_4d = read_nifti_4d(phase_path)
            .with_context(|| format!("Failed to read phase image '{}'", phase_path))?;

        // Apply echo selection to phase too
        if let Some(sel) = parse_echo_selection(&cli.echoes, phase_4d.nt) {
            phase_4d = select_volumes(&phase_4d, &sel);
        }

        // Validate dimensions match magnitude
        if phase_4d.dims != mag_4d.dims {
            anyhow::bail!(
                "Phase image dimensions {:?} do not match magnitude dimensions {:?}",
                phase_4d.dims,
                mag_4d.dims
            );
        }
        if phase_4d.volumes.is_empty() {
            anyhow::bail!("Phase image contains no volumes");
        }

        // Use first echo phase for unwrapping
        let mut phase_data = phase_4d.volumes[0].clone();

        // Rescale phase to [-π; π] if needed
        if !cli.no_phase_rescale {
            rescale_phase(&mut phase_data);
        }

        // Fix GE phase if requested
        if cli.fix_ge_phase {
            fix_ge_phase_slices(&mut phase_data, nx, ny, nz);
            if cli.verbose {
                eprintln!("  applied GE phase slice-jump correction");
            }
        }

        if let Some(dir) = writesteps_dir {
            write_step(dir, "phase_rescaled", &phase_data, &mag_4d)?;
        }

        // Unwrap phase using selected algorithm
        let unwrapped = match cli.unwrapping_algorithm.to_lowercase().as_str() {
            "romeo" => unwrap_romeo(&phase_data, &mag_corrected, &mask, nx, ny, nz),
            _ => laplacian_unwrap(&phase_data, &mask, nx, ny, nz, vsx, vsy, vsz),
        };

        if let Some(dir) = writesteps_dir {
            write_step(dir, "phase_unwrapped", &unwrapped, &mag_4d)?;
        }

        unwrapped
    } else {
        vec![0.0; n_voxels]
    };

    // Parse filter size
    let hp_sigma = parse_filter_size(&cli.filter_size);

    // Parse phase scaling type
    let scaling = match cli.phase_scaling_type.to_lowercase().as_str() {
        "tanh" => PhaseScaling::Tanh,
        "negativetanh" => PhaseScaling::NegativeTanh,
        "positive" => PhaseScaling::Positive,
        "negative" => PhaseScaling::Negative,
        "triangular" => PhaseScaling::Triangular,
        _ => PhaseScaling::Tanh,
    };

    let strength: f64 = cli.phase_scaling_strength.parse().unwrap_or(4.0);

    if cli.verbose {
        eprintln!(
            "  filter size: [{:.1}, {:.1}, {:.1}]",
            hp_sigma[0], hp_sigma[1], hp_sigma[2]
        );
        eprintln!("  phase scaling: {:?}, strength: {}", scaling, strength);
    }

    // Calculate SWI
    let mut swi = calculate_swi(
        &unwrapped_phase,
        &mag_corrected,
        &mask,
        nx,
        ny,
        nz,
        vsx,
        vsy,
        vsz,
        hp_sigma,
        scaling,
        strength,
    );

    // Apply softplus scaling if enabled
    if cli.mag_softplus_scaling != "off" {
        let masked_vals: Vec<f64> = mag_corrected
            .iter()
            .zip(mask.iter())
            .filter_map(|(&v, &m)| if m > 0 { Some(v) } else { None })
            .collect();
        if !masked_vals.is_empty() {
            let offset = compute_median(&masked_vals) * 0.1;
            if offset > 0.0 {
                swi = softplus_scaling(&swi, offset, 2.0);
            }
        }
    }

    if cli.verbose {
        eprintln!("  SWI calculation complete");
    }

    if let Some(dir) = writesteps_dir {
        write_step(dir, "swi_unscaled", &swi, &mag_4d)?;
    }

    // Write SWI output
    let out_path = if cli.output.ends_with(".nii.gz") || cli.output.ends_with(".nii") {
        cli.output.clone()
    } else {
        format!("{}.nii", cli.output)
    };

    let out_nii = NiftiData {
        data: swi.clone(),
        dims: (nx, ny, nz),
        voxel_size: mag_4d.voxel_size,
        affine: mag_4d.affine,
        scl_slope: 1.0,
        scl_inter: 0.0,
    };
    write_nifti(&out_path, &out_nii)
        .with_context(|| format!("Failed to write output '{}'", out_path))?;

    if cli.verbose {
        eprintln!("  saved to: {}", out_path);
    }

    // Create MIP if requested
    let mip_window: usize = cli.mip_slices.parse().unwrap_or(7);
    if mip_window > 0 && mip_window <= nz {
        let mip = create_mip(&swi, nx, ny, nz, mip_window);
        if !mip.is_empty() {
            let nz_mip = nz - mip_window + 1;
            let mip_path = derive_path(&out_path, "mip");
            let mip_nii = NiftiData {
                data: mip,
                dims: (nx, ny, nz_mip),
                voxel_size: mag_4d.voxel_size,
                affine: mag_4d.affine,
                scl_slope: 1.0,
                scl_inter: 0.0,
            };
            write_nifti(&mip_path, &mip_nii)
                .with_context(|| format!("Failed to write MIP '{}'", mip_path))?;
            if cli.verbose {
                eprintln!("  MIP saved to: {}", mip_path);
            }
        }
    }

    Ok(())
}

/// Combine magnitude echoes according to the specified method.
fn combine_magnitude(mag_4d: &NiftiData4D, method: &[String], echo_times: &[f64]) -> Vec<f64> {
    let n_voxels = mag_4d.dims.0 * mag_4d.dims.1 * mag_4d.dims.2;

    let method_str = method
        .first()
        .map(|s| s.to_lowercase())
        .unwrap_or_else(|| "snr".to_string());

    match method_str.as_str() {
        "snr" => {
            // Root sum of squares
            if mag_4d.nt > 1 {
                let mut combined = vec![0.0f64; n_voxels];
                for vol in &mag_4d.volumes {
                    for (c, &v) in combined.iter_mut().zip(vol.iter()) {
                        *c += v * v;
                    }
                }
                combined.iter_mut().for_each(|c| *c = c.sqrt());
                combined
            } else {
                mag_4d.volumes[0].clone()
            }
        }
        "average" => {
            // Average across echoes
            if mag_4d.nt > 1 {
                let mut combined = vec![0.0f64; n_voxels];
                for vol in &mag_4d.volumes {
                    for (c, &v) in combined.iter_mut().zip(vol.iter()) {
                        *c += v;
                    }
                }
                let nt = mag_4d.nt as f64;
                combined.iter_mut().for_each(|c| *c /= nt);
                combined
            } else {
                mag_4d.volumes[0].clone()
            }
        }
        "echo" => {
            // Select specific echo number
            let echo_num: usize = method.get(1).and_then(|s| s.parse().ok()).unwrap_or(1);
            let idx = (echo_num - 1).min(mag_4d.nt - 1);
            mag_4d.volumes[idx].clone()
        }
        "se" => {
            // Select echo closest to specified TE
            let target_te: f64 = method.get(1).and_then(|s| s.parse().ok()).unwrap_or(0.0);
            if echo_times.is_empty() {
                mag_4d.volumes[0].clone()
            } else {
                let idx = echo_times
                    .iter()
                    .enumerate()
                    .min_by(|(_, a), (_, b)| {
                        (*a - target_te).abs().total_cmp(&(*b - target_te).abs())
                    })
                    .map(|(i, _)| i)
                    .unwrap_or(0)
                    .min(mag_4d.nt - 1);
                mag_4d.volumes[idx].clone()
            }
        }
        _ => {
            // Default: SNR
            if mag_4d.nt > 1 {
                let mut combined = vec![0.0f64; n_voxels];
                for vol in &mag_4d.volumes {
                    for (c, &v) in combined.iter_mut().zip(vol.iter()) {
                        *c += v * v;
                    }
                }
                combined.iter_mut().for_each(|c| *c = c.sqrt());
                combined
            } else {
                mag_4d.volumes[0].clone()
            }
        }
    }
}

/// Unwrap phase using ROMEO algorithm (instead of Laplacian).
fn unwrap_romeo(
    phase: &[f64],
    mag: &[f64],
    mask: &[u8],
    nx: usize,
    ny: usize,
    nz: usize,
) -> Vec<f64> {
    let weights = calculate_weights_romeo(phase, mag, None, 1.0, 1.0, mask, nx, ny, nz);

    let mut unwrapped = phase.to_vec();
    let mut mask_work = mask.to_vec();

    let seed = find_seed(mask, &weights, nx, ny, nz);
    grow_region_unwrap(
        &mut unwrapped,
        &weights,
        &mut mask_work,
        nx,
        ny,
        nz,
        seed.0,
        seed.1,
        seed.2,
    );

    unwrapped
}

/// Write an intermediate step NIfTI file.
fn write_step(dir: &str, name: &str, data: &[f64], nii4d: &NiftiData4D) -> Result<()> {
    let path = std::path::Path::new(dir).join(format!("{}.nii", name));
    write_nifti_from_4d(path.to_str().unwrap(), data, nii4d)?;
    Ok(())
}

/// Rescale phase data to the range [-π, π].
fn rescale_phase(phase: &mut [f64]) {
    let min = phase.iter().cloned().fold(f64::INFINITY, f64::min);
    let max = phase.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    if (max - min).abs() < 1e-10 {
        return;
    }
    let pi = std::f64::consts::PI;
    for v in phase.iter_mut() {
        *v = (*v - min) / (max - min) * 2.0 * pi - pi;
    }
}

/// Build a robust magnitude-based binary mask (threshold at 10% of max).
fn robust_mask(mag: &[f64]) -> Vec<u8> {
    let max = mag.iter().cloned().fold(0.0_f64, f64::max);
    if max < 1e-10 {
        return vec![1u8; mag.len()];
    }
    let threshold = 0.1 * max;
    mag.iter()
        .map(|&v| if v >= threshold { 1u8 } else { 0u8 })
        .collect()
}

/// Parse filter size from CLI arguments. Default: [4.0, 4.0, 0.0].
fn parse_filter_size(args: &[String]) -> [f64; 3] {
    if args.is_empty() {
        return [4.0, 4.0, 0.0];
    }
    let joined = args.join(" ");
    let cleaned = joined
        .trim_start_matches('[')
        .trim_end_matches(']')
        .replace(',', " ");
    let vals: Vec<f64> = cleaned
        .split_whitespace()
        .filter_map(|s| s.parse().ok())
        .collect();
    match vals.len() {
        0 => [4.0, 4.0, 0.0],
        1 => [vals[0], vals[0], 0.0],
        2 => [vals[0], vals[1], 0.0],
        _ => [vals[0], vals[1], vals[2]],
    }
}

/// Compute the median of a slice, filtering out NaN values.
fn compute_median(values: &[f64]) -> f64 {
    let mut sorted: Vec<f64> = values.iter().copied().filter(|v| v.is_finite()).collect();
    if sorted.is_empty() {
        return 0.0;
    }
    sorted.sort_by(|a, b| a.total_cmp(b));
    let n = sorted.len();
    if n.is_multiple_of(2) {
        (sorted[n / 2 - 1] + sorted[n / 2]) / 2.0
    } else {
        sorted[n / 2]
    }
}

/// Find the seed voxel for ROMEO unwrapping.
fn find_seed(
    mask: &[u8],
    _weights: &[u8],
    nx: usize,
    ny: usize,
    nz: usize,
) -> (usize, usize, usize) {
    let ci = nx / 2;
    let cj = ny / 2;
    let ck = nz / 2;
    let center_idx = ci + cj * nx + ck * nx * ny;
    if mask.len() > center_idx && mask[center_idx] == 1 {
        return (ci, cj, ck);
    }
    for k in 0..nz {
        for j in 0..ny {
            for i in 0..nx {
                let idx = i + j * nx + k * nx * ny;
                if mask.len() > idx && mask[idx] == 1 {
                    return (i, j, k);
                }
            }
        }
    }
    (0, 0, 0)
}

/// Derive a side-output path by inserting a suffix before the `.nii` extension.
fn derive_path(base: &str, suffix: &str) -> String {
    if let Some(stripped) = base.strip_suffix(".nii.gz") {
        format!("{}_{}.nii.gz", stripped, suffix)
    } else if let Some(stripped) = base.strip_suffix(".nii") {
        format!("{}_{}.nii", stripped, suffix)
    } else {
        format!("{}_{}.nii", base, suffix)
    }
}
