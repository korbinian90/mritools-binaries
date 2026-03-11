//! romeo_mask — ROMEO quality-based brain masking CLI
//!
//! Matches the Julia CLI interface from korbinian90/CompileMRI.jl (ROMEO_mask.jl).
//!
//! Reference:
//!   Dymerska, B., et al. (2021). "Phase unwrapping with a rapid opensource
//!   minimum spanning tree algorithm (ROMEO)." MRM, 85(4):2294-2308.
//!   https://doi.org/10.1002/mrm.28563

use anyhow::{Context, Result};
use clap::Parser;
use mritools_common::{parse_echo_times, read_nifti, write_nifti};
use qsm_core::unwrap::romeo::calculate_weights_romeo;
use qsm_core::utils::otsu_threshold;

/// ROMEO quality-based brain masking.
///
/// Generates a brain mask based on ROMEO phase-coherence quality maps.
/// Matches the Julia CLI interface from korbinian90/CompileMRI.jl.
#[derive(Parser, Debug)]
#[command(
    name = "romeo_mask",
    about = "ROMEO quality-based brain masking",
    version
)]
struct Cli {
    /// The phase image
    #[arg(short = 'p', long)]
    phase: Option<String>,

    /// The magnitude image (improves mask quality if specified)
    #[arg(short = 'm', long)]
    magnitude: Option<String>,

    /// The output path or filename [default: unwrapped.nii]
    #[arg(short = 'o', long, default_value = "unwrapped.nii")]
    output: String,

    /// Masking threshold factor in [0;1] [default: 0.1]
    #[arg(short = 'f', long, default_value_t = 0.1)]
    factor: f64,

    /// Echo times in [ms]: "[1.5,3.0]" | "3.5:3.5:14" | "epi [te]"
    #[arg(short = 't', long = "echo-times", num_args = 1..)]
    echo_times: Vec<String>,

    /// Load only the specified echoes from disk
    #[arg(short = 'e', long = "unwrap-echoes", num_args = 1.., default_values = &[":"])]
    unwrap_echoes: Vec<String>,

    /// Weights: romeo | romeo2 | romeo3 | romeo4 | romeo6 | bestpath | <file> | <flags>
    #[arg(short = 'w', long, default_value = "romeo")]
    weights: String,

    /// Deactivate automatic rescaling of phase images to [-π;π]
    #[arg(long)]
    no_phase_rescale: bool,

    /// Fix GE phase slice-jump artefacts
    #[arg(long)]
    fix_ge_phase: bool,

    /// Verbose output
    #[arg(short = 'v', long)]
    verbose: bool,

    /// Write ROMEO quality map (3D, one value per voxel)
    #[arg(short = 'q', long)]
    write_quality: bool,

    /// Write individual quality map for each ROMEO weight
    #[arg(short = 'Q', long)]
    write_quality_all: bool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Warn about flags that are accepted for CLI compatibility but not yet implemented
    if cli.unwrap_echoes.len() != 1 || cli.unwrap_echoes[0] != ":" {
        eprintln!(
            "WARNING: --unwrap-echoes is not yet implemented in this Rust port, loading all echoes"
        );
    }
    if cli.weights != "romeo" {
        eprintln!(
            "WARNING: --weights '{}' variant is not yet implemented, using default romeo weights",
            cli.weights
        );
    }
    if cli.fix_ge_phase {
        eprintln!("WARNING: --fix-ge-phase is not yet implemented in this Rust port, ignoring");
    }

    let phase = cli
        .phase
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("--phase / -p is required"))?;

    if cli.verbose {
        eprintln!("romeo_mask");
        eprintln!("  phase:  {}", phase);
        if let Some(ref m) = cli.magnitude {
            eprintln!("  magnitude: {}", m);
        }
        eprintln!("  output: {}", cli.output);
        eprintln!("  factor: {}", cli.factor);
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
    mritools_common::save_settings(output_dir, "romeo_mask", &args)?;

    // Parse echo times
    let echo_times = parse_echo_times(&cli.echo_times).context("Failed to parse --echo-times")?;

    // Load phase image
    let phase_nii =
        read_nifti(phase).with_context(|| format!("Failed to read phase image '{}'", phase))?;

    let (nx, ny, nz) = phase_nii.dims;
    let n_voxels = nx * ny * nz;

    // Rescale phase to [-π; π] if needed
    let mut phase_data = phase_nii.data.clone();
    if !cli.no_phase_rescale {
        rescale_phase(&mut phase_data);
    }

    // Load magnitude image if provided
    let mag_data: Vec<f64> = if let Some(ref mag_path) = cli.magnitude {
        let mag_nii = read_nifti(mag_path)
            .with_context(|| format!("Failed to read magnitude image '{}'", mag_path))?;
        mag_nii.data
    } else {
        vec![]
    };

    // Calculate echo time parameters
    let (te1, te2) = if echo_times.len() >= 2 {
        (echo_times[0], echo_times[1])
    } else if echo_times.len() == 1 {
        (echo_times[0], echo_times[0])
    } else {
        (1.0, 1.0)
    };

    // Build a simple mask (all ones, or magnitude-based)
    let initial_mask = if !mag_data.is_empty() {
        robust_mask(&mag_data)
    } else {
        vec![1u8; n_voxels]
    };

    // Calculate ROMEO weights
    let weights = calculate_weights_romeo(
        &phase_data,
        &mag_data,
        None,
        te1,
        te2,
        &initial_mask,
        nx,
        ny,
        nz,
    );

    // Compute per-voxel quality map from weights
    let quality = compute_quality_map(&weights, n_voxels);

    if cli.verbose {
        let qmin = quality.iter().cloned().fold(f64::INFINITY, f64::min);
        let qmax = quality.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        eprintln!("  quality range: [{:.3}, {:.3}]", qmin, qmax);
    }

    // Threshold the quality map to create a mask
    // Use Otsu threshold scaled by the factor parameter
    let threshold = otsu_threshold(&quality, 256) * (1.0 - cli.factor);

    let mask: Vec<f64> = quality
        .iter()
        .zip(initial_mask.iter())
        .map(|(&q, &m)| if m > 0 && q >= threshold { 1.0 } else { 0.0 })
        .collect();

    if cli.verbose {
        let n_mask = mask.iter().filter(|&&v| v > 0.5).count();
        eprintln!("  threshold: {:.4}", threshold);
        eprintln!("  mask voxels: {}/{}", n_mask, n_voxels);
    }

    // Write mask output
    let out_path = if cli.output.ends_with(".nii.gz") || cli.output.ends_with(".nii") {
        cli.output.clone()
    } else {
        format!("{}.nii", cli.output)
    };

    let mut out_nii = phase_nii;
    out_nii.data = mask;
    write_nifti(&out_path, &out_nii)
        .with_context(|| format!("Failed to write output '{}'", out_path))?;

    if cli.verbose {
        eprintln!("  saved to: {}", out_path);
    }

    // Write quality map if requested
    if cli.write_quality {
        let mut q_nii = mritools_common::read_nifti(phase)?;
        q_nii.data = quality.clone();
        let q_path = derive_path(&out_path, "quality");
        write_nifti(&q_path, &q_nii)?;
        if cli.verbose {
            eprintln!("  quality map saved to: {}", q_path);
        }
    }

    // Write all quality maps if requested
    if cli.write_quality_all {
        let per_dim = weights.len() / 3;
        for (d, name) in [(0, "quality_x"), (1, "quality_y"), (2, "quality_z")] {
            let mut q_data = vec![0.0f64; n_voxels];
            for idx in 0..per_dim.min(n_voxels) {
                q_data[idx] = weights[d * per_dim + idx] as f64 / 255.0;
            }
            let mut q_nii = mritools_common::read_nifti(phase)?;
            q_nii.data = q_data;
            let q_path = derive_path(&out_path, name);
            write_nifti(&q_path, &q_nii)?;
            if cli.verbose {
                eprintln!("  {} saved to: {}", name, q_path);
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

/// Compute a per-voxel quality map from the edge weights (average of adjacent edges).
fn compute_quality_map(weights: &[u8], n_voxels: usize) -> Vec<f64> {
    let n_w = weights.len();
    let per_dim = n_w / 3;
    let mut quality = vec![0.0f64; n_voxels];
    let mut counts = vec![0u32; n_voxels];
    for d in 0..3usize {
        for idx in 0..per_dim {
            if idx < n_voxels {
                let w = weights[d * per_dim + idx] as f64 / 255.0;
                quality[idx] += w;
                counts[idx] += 1;
            }
        }
    }
    for (q, &c) in quality.iter_mut().zip(counts.iter()) {
        if c > 0 {
            *q /= c as f64;
        }
    }
    quality
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
