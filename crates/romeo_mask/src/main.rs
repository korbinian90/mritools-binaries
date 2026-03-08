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

    eprintln!(
        "WARNING: romeo_mask full processing is not yet implemented in this Rust port. \
         The binary accepts all CLI flags for compatibility."
    );

    Ok(())
}
