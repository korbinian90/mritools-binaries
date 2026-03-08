//! makehomogeneous — Homogeneity correction CLI for high-field MRI
//!
//! Matches the Julia CLI interface from korbinian90/CompileMRI.jl (HomogeneityCorrection.jl).
//!
//! Reference:
//!   Eckstein, K., Trattnig, S., Robinson, S.D. (2019). "A Simple Homogeneity
//!   Correction for Neuroimaging at 7T." Proc. ISMRM 27th Annual Meeting.
//!
//! NOTE: Full homogeneity correction processing is not yet implemented. This binary
//! accepts all CLI flags for compatibility with existing scripts but prints a warning.

use anyhow::{Context, Result};
use clap::Parser;

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

    eprintln!(
        "WARNING: makehomogeneous full processing is not yet implemented in this Rust port. \
         The binary accepts all CLI flags for compatibility."
    );

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

    Ok(())
}
