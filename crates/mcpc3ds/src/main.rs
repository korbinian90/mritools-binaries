//! MCPC-3D-S — Multi-Channel Phase Combination CLI
//!
//! Matches the Julia CLI interface from korbinian90/CompileMRI.jl (Mcpc3ds.jl).
//!
//! Reference:
//!   Eckstein, K., et al. (2018). "Computationally Efficient Combination of
//!   Multi-channel Phase Data From Multi-echo Acquisitions (ASPIRE)."
//!   MRM, 79:2996-3006. https://doi.org/10.1002/mrm.26963
//!
//! NOTE: Full MCPC-3D-S processing is not yet implemented. This binary accepts
//! all CLI flags for compatibility with existing scripts but prints a warning.

use anyhow::{Context, Result};
use clap::Parser;

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

    eprintln!(
        "WARNING: MCPC-3D-S full processing is not yet implemented in this Rust port. \
         The binary accepts all CLI flags for compatibility."
    );

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

    Ok(())
}
