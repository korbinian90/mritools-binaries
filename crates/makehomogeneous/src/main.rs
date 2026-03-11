//! makehomogeneous — Homogeneity correction CLI for high-field MRI
//!
//! Matches the Julia CLI interface from korbinian90/CompileMRI.jl (HomogeneityCorrection.jl).
//!
//! Reference:
//!   Eckstein, K., Trattnig, S., Robinson, S.D. (2019). "A Simple Homogeneity
//!   Correction for Neuroimaging at 7T." Proc. ISMRM 27th Annual Meeting.

use anyhow::{Context, Result};
use clap::Parser;
use mritools_common::{read_nifti, write_nifti};

/// Homogeneity correction for high-field MRI.
///
/// Removes intensity bias (bias field) from magnitude images using a simple
/// Gaussian smoothing approach. Matches the Julia CLI interface from
/// korbinian90/CompileMRI.jl.
#[derive(Parser, Debug)]
#[command(
    name = "makehomogeneous",
    about = "Homogeneity correction for high-field MRI magnitude images",
    version
)]
struct Cli {
    /// The magnitude image (single or multi-echo)
    #[arg(short = 'm', long)]
    magnitude: Option<String>,

    /// The output path or filename [default: homogenous]
    #[arg(short = 'o', long, default_value = "homogenous")]
    output: String,

    /// Sigma size [mm] for bias field smoothing [default: 7.0]
    #[arg(short = 's', long = "sigma-bias-field", default_value_t = 7.0)]
    sigma_bias_field: f64,

    /// Number of boxes for box-segmentation step [default: 15]
    #[arg(short = 'n', long, default_value_t = 15)]
    nbox: i32,

    /// Output datatype (e.g. Float32, Int16) [default: same as input]
    #[arg(short = 'd', long)]
    datatype: Option<String>,

    /// Verbose output
    #[arg(short = 'v', long)]
    verbose: bool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Warn about flags that are accepted for CLI compatibility but not yet implemented
    if cli.datatype.is_some() {
        eprintln!("WARNING: --datatype is not yet implemented in this Rust port, using input type");
    }

    let magnitude = cli
        .magnitude
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("--magnitude / -m is required"))?;

    if cli.verbose {
        eprintln!("makehomogeneous");
        eprintln!("  magnitude:        {}", magnitude);
        eprintln!("  output:           {}", cli.output);
        eprintln!("  sigma-bias-field: {}", cli.sigma_bias_field);
        eprintln!("  nbox:             {}", cli.nbox);
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
    mritools_common::save_settings(output_dir, "makehomogeneous", &args)?;

    // Load magnitude image
    let mag_nii = read_nifti(magnitude)
        .with_context(|| format!("Failed to read magnitude image '{}'", magnitude))?;

    let (nx, ny, nz) = mag_nii.dims;
    let (vx, vy, vz) = mag_nii.voxel_size;

    if cli.verbose {
        eprintln!("  dims: {}x{}x{}", nx, ny, nz);
        eprintln!("  voxel size: {:.3}x{:.3}x{:.3} mm", vx, vy, vz);
    }

    // Apply homogeneity correction
    let corrected = qsm_core::utils::makehomogeneous(
        &mag_nii.data,
        nx,
        ny,
        nz,
        vx,
        vy,
        vz,
        cli.sigma_bias_field,
        cli.nbox.max(1) as usize,
    );

    if cli.verbose {
        eprintln!("  homogeneity correction complete");
    }

    // Write output
    let out_path = if cli.output.ends_with(".nii.gz") || cli.output.ends_with(".nii") {
        cli.output.clone()
    } else {
        format!("{}.nii", cli.output)
    };

    let mut out_nii = mag_nii;
    out_nii.data = corrected;
    write_nifti(&out_path, &out_nii)
        .with_context(|| format!("Failed to write output '{}'", out_path))?;

    if cli.verbose {
        eprintln!("  saved to: {}", out_path);
    }

    Ok(())
}
