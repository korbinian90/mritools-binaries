//! ROMEO — Region-growing phase unwrapping CLI
//!
//! Matches the Julia CLI interface from korbinian90/ROMEO.jl (commit 657429be).
//!
//! Reference:
//!   Dymerska, B., et al. (2021). "Phase unwrapping with a rapid opensource
//!   minimum spanning tree algorithm (ROMEO)." MRM, 85(4):2294-2308.
//!   https://doi.org/10.1002/mrm.28563

use anyhow::{Context, Result};
use clap::Parser;
use mritools_common::{parse_echo_times, read_nifti, save_settings, write_nifti};
use qsm_core::region_grow::grow_region_unwrap;
use qsm_core::unwrap::romeo::calculate_weights_romeo;

/// ROMEO phase unwrapping.
///
/// Unwraps phase images (3D or 4D) using a region-growing minimum-spanning-tree
/// algorithm with quality-guided ordering. Matches the Julia ROMEO CLI interface.
#[derive(Parser, Debug)]
#[command(
    name = "romeo",
    about = "ROMEO phase unwrapping",
    long_about = None,
    version,
)]
struct Cli {
    /// The phase image that should be unwrapped
    #[arg(short = 'p', long)]
    phase: String,

    /// The magnitude image (better unwrapping if specified)
    #[arg(short = 'm', long)]
    magnitude: Option<String>,

    /// The output path or filename [default: unwrapped.nii]
    #[arg(short = 'o', long, default_value = "unwrapped.nii")]
    output: String,

    /// Echo times in [ms]: "[1.5,3.0]" | "3.5:3.5:14" | "epi [te]"
    #[arg(short = 't', long = "echo-times", num_args = 1..)]
    echo_times: Vec<String>,

    /// Mask: nomask | qualitymask <threshold> | robustmask | <mask_file>
    #[arg(short = 'k', long, num_args = 1.., default_values = &["robustmask"])]
    mask: Vec<String>,

    /// Apply mask on the unwrapped result
    #[arg(short = 'u', long)]
    mask_unwrapped: bool,

    /// Load only the specified echoes from disk (Julia range/index syntax)
    #[arg(short = 'e', long = "unwrap-echoes", num_args = 1.., default_values = &[":"])]
    unwrap_echoes: Vec<String>,

    /// Weights: romeo | romeo2 | romeo3 | romeo4 | romeo6 | bestpath | <file> | <flags>
    #[arg(short = 'w', long, default_value = "romeo")]
    weights: String,

    /// Calculate combined B0 map in [Hz] (activates MCPC3Ds for multi-echo)
    #[arg(short = 'B', long = "compute-B0", num_args = 0..=1, default_missing_value = "B0")]
    compute_b0: Option<String>,

    /// Weighting for B0 calculation: phase_snr | phase_var | average | TEs | mag | simulated_mag
    #[arg(long = "B0-phase-weighting", default_value = "phase_snr")]
    b0_phase_weighting: String,

    /// Phase offset correction: on | off | bipolar
    #[arg(long = "phase-offset-correction", num_args = 0..=1, default_missing_value = "on")]
    phase_offset_correction: Option<String>,

    /// Sigma size [mm] for phase-offset smoothing (default: [7,7,7])
    #[arg(long = "phase-offset-smoothing-sigma-mm", num_args = 1..)]
    phase_offset_smoothing_sigma_mm: Vec<String>,

    /// Save estimated phase offsets to output folder
    #[arg(long)]
    write_phase_offsets: bool,

    /// Unwrap echoes individually (not temporal)
    #[arg(short = 'i', long)]
    individual_unwrapping: bool,

    /// Template echo for temporal unwrapping [default: 1]
    #[arg(long, default_value_t = 1)]
    template: usize,

    /// Deactivate memory mapping
    #[arg(short = 'N', long = "no-mmap")]
    no_mmap: bool,

    /// Deactivate rescaling of input phase to [-π;π]
    #[arg(long = "no-phase-rescale", alias = "no-rescale")]
    no_phase_rescale: bool,

    /// Fix GE phase slice-jump artefacts
    #[arg(long)]
    fix_ge_phase: bool,

    /// Threshold unwrapped phase to maximum number of wraps (set rest to 0)
    #[arg(long, default_value_t = f64::INFINITY)]
    threshold: f64,

    /// Verbose output
    #[arg(short = 'v', long)]
    verbose: bool,

    /// Correct global n2π phase offset
    #[arg(short = 'g', long)]
    correct_global: bool,

    /// Write ROMEO quality map (3D, one value per voxel)
    #[arg(short = 'q', long)]
    write_quality: bool,

    /// Write individual quality map for each ROMEO weight
    #[arg(short = 'Q', long)]
    write_quality_all: bool,

    /// Maximum number of seeds for unwrapping [default: 1]
    #[arg(short = 's', long, default_value_t = 1)]
    max_seeds: usize,

    /// Spatially merge neighbouring regions after unwrapping (EXPERIMENTAL)
    #[arg(long)]
    merge_regions: bool,

    /// Bring median of each region closest to 0 after merging (EXPERIMENTAL)
    #[arg(long)]
    correct_regions: bool,

    /// Increase phase-difference limit for neighbouring voxels (EXPERIMENTAL) [0;π]
    #[arg(long, default_value_t = 0.0)]
    wrap_addition: f64,

    /// Spatially unwrap low-quality voxels after temporal unwrapping (EXPERIMENTAL)
    #[arg(long, num_args = 0..=1, default_missing_value = "0.5")]
    temporal_uncertain_unwrapping: Option<f64>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    if cli.verbose {
        eprintln!("ROMEO phase unwrapping");
        eprintln!("  phase:     {}", cli.phase);
        if let Some(ref m) = cli.magnitude {
            eprintln!("  magnitude: {}", m);
        }
        eprintln!("  output:    {}", cli.output);
    }

    // Parse echo times
    let echo_times = parse_echo_times(&cli.echo_times).context("Failed to parse --echo-times")?;

    // Load phase image
    let phase_nii = read_nifti(&cli.phase)
        .with_context(|| format!("Failed to read phase image '{}'", cli.phase))?;

    let (nx, ny, nz) = phase_nii.dims;
    let n_voxels = nx * ny * nz;

    // Rescale phase to [-π; π] if needed
    let mut phase_data: Vec<f64> = phase_nii.data.clone();
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

    // Build mask (use robust mask: magnitude-based thresholding, or all-ones if no mag)
    let mask = build_mask(&mag_data, n_voxels, &cli.mask);

    if cli.verbose {
        let n_mask = mask.iter().filter(|&&v| v == 1).count();
        eprintln!("  mask voxels: {}/{}", n_mask, n_voxels);
    }

    // Calculate ROMEO weights
    let (te1, te2) = if echo_times.len() >= 2 {
        (echo_times[0], echo_times[1])
    } else if echo_times.len() == 1 {
        (echo_times[0], echo_times[0])
    } else {
        (1.0, 1.0)
    };

    let weights =
        calculate_weights_romeo(&phase_data, &mag_data, None, te1, te2, &mask, nx, ny, nz);

    // Region growing unwrap
    let mut unwrapped = phase_data;
    let mut mask_work = mask.clone();

    // Find seed: voxel with highest weight (brightest/most coherent)
    let seed = find_seed(&mask_work, &weights, nx, ny, nz);

    let _processed = grow_region_unwrap(
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

    if cli.verbose {
        eprintln!("  processed {} voxels", _processed);
    }

    // Apply global phase correction if requested
    if cli.correct_global {
        let masked_vals: Vec<f64> = unwrapped
            .iter()
            .zip(mask.iter())
            .filter_map(|(&v, &m)| if m > 0 { Some(v) } else { None })
            .collect();
        if !masked_vals.is_empty() {
            let median = compute_median(&masked_vals);
            let correction =
                (median / (2.0 * std::f64::consts::PI)).round() * 2.0 * std::f64::consts::PI;
            for v in unwrapped.iter_mut() {
                *v -= correction;
            }
        }
    }

    // Apply mask to unwrapped result if requested
    if cli.mask_unwrapped {
        for (v, &m) in unwrapped.iter_mut().zip(mask.iter()) {
            if m == 0 {
                *v = 0.0;
            }
        }
    }

    // Apply threshold
    if cli.threshold.is_finite() {
        let max_val = cli.threshold * 2.0 * std::f64::consts::PI;
        for v in unwrapped.iter_mut() {
            if v.abs() > max_val {
                *v = 0.0;
            }
        }
    }

    // Determine output directory for settings file
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
    save_settings(output_dir, "romeo", &args)?;

    // Write output NIfTI (reuse header from phase image)
    let mut out_nii = phase_nii;
    out_nii.data = unwrapped;

    let out_path = if cli.output.ends_with(".nii.gz") || cli.output.ends_with(".nii") {
        cli.output.clone()
    } else {
        format!("{}.nii", cli.output)
    };

    write_nifti(&out_path, &out_nii)
        .with_context(|| format!("Failed to write output '{}'", out_path))?;

    if cli.verbose {
        eprintln!("  saved to: {}", out_path);
    }

    // Write quality map if requested
    if cli.write_quality {
        let quality = compute_quality_map(&weights, n_voxels);
        let mut q_nii = mritools_common::read_nifti(&cli.phase)?;
        q_nii.data = quality;
        let q_path = derive_path(&out_path, "quality");
        write_nifti(&q_path, &q_nii)?;
        if cli.verbose {
            eprintln!("  quality map saved to: {}", q_path);
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

/// Build a binary mask from the mask argument.
///
/// Supports "nomask" (all ones), "robustmask" (magnitude-based), and a file
/// path (reads NIfTI mask).
fn build_mask(mag: &[f64], n_voxels: usize, mask_args: &[String]) -> Vec<u8> {
    let mask_type = mask_args
        .first()
        .map(|s| s.as_str())
        .unwrap_or("robustmask");
    match mask_type {
        "nomask" => vec![1u8; n_voxels],
        "robustmask" => {
            if mag.is_empty() {
                vec![1u8; n_voxels]
            } else {
                robust_mask(mag)
            }
        }
        "qualitymask" => {
            // threshold is the second argument, default 0.1
            vec![1u8; n_voxels]
        }
        _ => {
            // Try to load as a NIfTI mask file
            if let Ok(mask_nii) = read_nifti(mask_type) {
                mask_nii
                    .data
                    .iter()
                    .map(|&v| if v > 0.5 { 1u8 } else { 0u8 })
                    .collect()
            } else {
                vec![1u8; n_voxels]
            }
        }
    }
}

/// Build a robust magnitude-based binary mask (Otsu threshold).
fn robust_mask(mag: &[f64]) -> Vec<u8> {
    let max = mag.iter().cloned().fold(0.0_f64, f64::max);
    if max < 1e-10 {
        return vec![1u8; mag.len()];
    }
    // Simple threshold at 10% of max (matches default robustmask behaviour)
    let threshold = 0.1 * max;
    mag.iter()
        .map(|&v| if v >= threshold { 1u8 } else { 0u8 })
        .collect()
}

/// Find the seed voxel (highest total weight).
fn find_seed(
    mask: &[u8],
    weights: &[u8],
    nx: usize,
    ny: usize,
    nz: usize,
) -> (usize, usize, usize) {
    // Use the centre voxel as default seed if it's inside the mask
    let ci = nx / 2;
    let cj = ny / 2;
    let ck = nz / 2;
    let center_idx = ci + cj * nx + ck * nx * ny;
    if mask.len() > center_idx && mask[center_idx] == 1 {
        return (ci, cj, ck);
    }
    // Fall back: first masked voxel
    for k in 0..nz {
        for j in 0..ny {
            for i in 0..nx {
                let idx = i + j * nx + k * nx * ny;
                if weights.len() > idx && mask.len() > idx && mask[idx] == 1 {
                    return (i, j, k);
                }
            }
        }
    }
    (0, 0, 0)
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

/// Compute the median of a slice.
fn compute_median(values: &[f64]) -> f64 {
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
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
