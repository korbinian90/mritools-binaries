//! MCPC-3D-S — Multi-Channel Phase Combination CLI
//!
//! Matches the Julia CLI interface from korbinian90/CompileMRI.jl (Mcpc3ds.jl).
//!
//! Reference:
//!   Eckstein, K., et al. (2018). "Computationally Efficient Combination of
//!   Multi-channel Phase Data From Multi-echo Acquisitions (ASPIRE)."
//!   MRM, 79:2996-3006. https://doi.org/10.1002/mrm.26963

use anyhow::{Context, Result};
use clap::Parser;
use mritools_common::{parse_echo_times, read_nifti_4d, write_nifti_from_4d};
use qsm_core::utils::mcpc3ds_single_coil;

/// MCPC-3D-S multi-channel phase combination.
///
/// Removes phase offsets across echoes using the MCPC-3D-S (ASPIRE) algorithm.
/// Matches the Julia CLI interface from korbinian90/CompileMRI.jl.
#[derive(Parser, Debug)]
#[command(
    name = "mcpc3ds",
    about = "MCPC-3D-S multi-channel phase combination",
    version
)]
struct Cli {
    /// The magnitude image (single or multi-echo)
    #[arg(short = 'm', long)]
    magnitude: Option<String>,

    /// The phase image (single or multi-echo)
    #[arg(short = 'p', long)]
    phase: Option<String>,

    /// The output path or filename [default: output]
    #[arg(short = 'o', long, default_value = "output")]
    output: String,

    /// Echo times in [ms]: "[1.5,3.0]" | "3.5:3.5:14"
    #[arg(short = 't', long = "echo-times", num_args = 1..)]
    echo_times: Vec<String>,

    /// Gaussian smoothing sigma in voxels [default: [10,10,5]]
    #[arg(short = 's', long = "smoothing-sigma", num_args = 1..)]
    smoothing_sigma: Vec<String>,

    /// Remove eddy current artefacts (requires >= 3 echoes)
    #[arg(short = 'b', long)]
    bipolar: bool,

    /// Save estimated phase offsets to output folder
    #[arg(long)]
    write_phase_offsets: bool,

    /// Deactivate memory mapping
    #[arg(short = 'N', long = "no-mmap")]
    no_mmap: bool,

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

    let phase = cli
        .phase
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("--phase / -p is required"))?;

    if cli.verbose {
        eprintln!("MCPC-3D-S");
        eprintln!("  phase:  {}", phase);
        if let Some(ref m) = cli.magnitude {
            eprintln!("  magnitude: {}", m);
        }
        eprintln!("  output: {}", cli.output);
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
    mritools_common::save_settings(output_dir, "mcpc3ds", &args)?;

    // Parse echo times
    let echo_times = parse_echo_times(&cli.echo_times).context("Failed to parse --echo-times")?;

    // Load 4D phase image
    let phase_4d =
        read_nifti_4d(phase).with_context(|| format!("Failed to read phase image '{}'", phase))?;

    let (nx, ny, nz) = phase_4d.dims;
    let n_echoes = phase_4d.nt;
    let n_voxels = nx * ny * nz;

    if cli.verbose {
        eprintln!("  dims: {}x{}x{}, {} echoes", nx, ny, nz, n_echoes);
    }

    // Ensure we have enough echo times
    let tes: Vec<f64> = if echo_times.len() >= n_echoes {
        echo_times[..n_echoes].to_vec()
    } else if echo_times.is_empty() {
        // Default: assume equal spacing
        (1..=n_echoes).map(|i| i as f64).collect()
    } else {
        echo_times.clone()
    };

    // Rescale phase to [-π; π] if needed
    let mut phases = phase_4d.volumes.clone();
    if !cli.no_phase_rescale {
        for vol in &mut phases {
            rescale_phase(vol);
        }
    }

    // Load magnitude image if provided
    let mags: Vec<Vec<f64>> = if let Some(ref mag_path) = cli.magnitude {
        let mag_4d = read_nifti_4d(mag_path)
            .with_context(|| format!("Failed to read magnitude image '{}'", mag_path))?;
        mag_4d.volumes
    } else {
        vec![vec![1.0; n_voxels]; n_echoes]
    };

    // Build mask from magnitude (use first echo)
    let mask = robust_mask(&mags[0]);

    // Parse smoothing sigma
    let sigma = parse_sigma(&cli.smoothing_sigma);

    if cli.verbose {
        eprintln!(
            "  smoothing sigma: [{:.1}, {:.1}, {:.1}]",
            sigma[0], sigma[1], sigma[2]
        );
        let n_mask = mask.iter().filter(|&&v| v == 1).count();
        eprintln!("  mask voxels: {}/{}", n_mask, n_voxels);
    }

    // Ensure at least 2 echoes for MCPC-3D-S
    if n_echoes < 2 {
        anyhow::bail!("MCPC-3D-S requires at least 2 echoes, got {}", n_echoes);
    }

    // Run MCPC-3D-S
    let (corrected_phases, phase_offset) =
        mcpc3ds_single_coil(&phases, &mags, &tes, &mask, sigma, [0, 1], nx, ny, nz);

    if cli.verbose {
        eprintln!("  MCPC-3D-S phase combination complete");
    }

    // Write corrected phases (first echo as main output)
    let out_path = if cli.output.ends_with(".nii.gz") || cli.output.ends_with(".nii") {
        cli.output.clone()
    } else {
        format!("{}.nii", cli.output)
    };

    write_nifti_from_4d(&out_path, &corrected_phases[0], &phase_4d)
        .with_context(|| format!("Failed to write output '{}'", out_path))?;

    if cli.verbose {
        eprintln!("  saved corrected phase to: {}", out_path);
    }

    // Write phase offsets if requested
    if cli.write_phase_offsets {
        let po_path = derive_path(&out_path, "phase_offset");
        write_nifti_from_4d(&po_path, &phase_offset, &phase_4d)
            .with_context(|| format!("Failed to write phase offsets '{}'", po_path))?;
        if cli.verbose {
            eprintln!("  phase offsets saved to: {}", po_path);
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

/// Parse smoothing sigma from CLI arguments. Default: [10, 10, 5].
fn parse_sigma(args: &[String]) -> [f64; 3] {
    if args.is_empty() {
        return [10.0, 10.0, 5.0];
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
        0 => [10.0, 10.0, 5.0],
        1 => [vals[0], vals[0], vals[0]],
        2 => [vals[0], vals[1], vals[0]],
        _ => [vals[0], vals[1], vals[2]],
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
