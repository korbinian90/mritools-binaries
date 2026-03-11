//! CLEAR-SWI — Susceptibility Weighted Imaging CLI
//!
//! Matches the Julia CLI interface from korbinian90/CLEARSWI.jl.
//!
//! Reference:
//!   Eckstein, K., et al. (2024). "CLEAR-SWI: Computational Efficient T2* Weighted Imaging."
//!   Proc. ISMRM.

use anyhow::{Context, Result};
use clap::Parser;
use mritools_common::{parse_echo_times, read_nifti_4d, write_nifti, NiftiData};
use qsm_core::swi::{calculate_swi, create_mip, softplus_scaling, PhaseScaling};
use qsm_core::unwrap::laplacian::laplacian_unwrap;

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

    // Warn about flags that are accepted for CLI compatibility but not yet implemented
    if cli.qsm {
        eprintln!("WARNING: --qsm is not yet implemented in this Rust port, ignoring");
    }
    if cli.qsm_input.is_some() {
        eprintln!("WARNING: --qsm-input is not yet implemented in this Rust port, ignoring");
    }
    if cli.qsm_mask.is_some() {
        eprintln!("WARNING: --qsm-mask is not yet implemented in this Rust port, ignoring");
    }
    if cli.mag_combine.len() != 1 || cli.mag_combine[0] != "SNR" {
        eprintln!("WARNING: --mag-combine is not yet implemented in this Rust port, using default");
    }
    if cli.mag_sensitivity_correction != "on" {
        eprintln!("WARNING: --mag-sensitivity-correction is not yet implemented in this Rust port, ignoring");
    }
    if cli.unwrapping_algorithm != "laplacian" {
        eprintln!(
            "WARNING: --unwrapping-algorithm '{}' is not yet implemented, using laplacian",
            cli.unwrapping_algorithm
        );
    }
    if cli.echoes.len() != 1 || cli.echoes[0] != ":" {
        eprintln!("WARNING: --echoes is not yet implemented in this Rust port, loading all echoes");
    }
    if cli.fix_ge_phase {
        eprintln!("WARNING: --fix-ge-phase is not yet implemented in this Rust port, ignoring");
    }
    if cli.writesteps.is_some() {
        eprintln!("WARNING: --writesteps is not yet implemented in this Rust port, ignoring");
    }

    let magnitude = cli
        .magnitude
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("--magnitude / -m is required"))?;
    let _phase = cli.phase.as_deref();

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
    mritools_common::save_settings(output_dir, "clearswi", &args)?;

    // Parse echo times
    let _echo_times = parse_echo_times(&cli.echo_times).context("Failed to parse --echo-times")?;

    // Load magnitude image (4D)
    let mag_4d = read_nifti_4d(magnitude)
        .with_context(|| format!("Failed to read magnitude image '{}'", magnitude))?;

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

    // Combine magnitude echoes (SNR-like: root sum of squares)
    let mag_combined: Vec<f64> = if mag_4d.nt > 1 {
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
    };

    // Build mask from combined magnitude
    let mask = robust_mask(&mag_combined);

    // Get phase data (load and unwrap, or default to zeros if no phase provided)
    let unwrapped_phase: Vec<f64> = if let Some(ref phase_path) = cli.phase {
        let phase_4d = read_nifti_4d(phase_path)
            .with_context(|| format!("Failed to read phase image '{}'", phase_path))?;

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

        // Unwrap phase
        laplacian_unwrap(&phase_data, &mask, nx, ny, nz, vsx, vsy, vsz)
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
        &mag_combined,
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
        // Estimate offset from magnitude (median of masked values)
        let masked_vals: Vec<f64> = mag_combined
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
            let mip_nii = mritools_common::NiftiData {
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
