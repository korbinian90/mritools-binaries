//! CLEAR-SWI — Susceptibility Weighted Imaging CLI
//!
//! Matches the Julia CLI interface from korbinian90/CLEARSWI.jl.
//!
//! Reference:
//!   Eckstein, K., et al. (2024). "CLEAR-SWI: Computational Efficient T2* Weighted Imaging."
//!   Proc. ISMRM.
//!
//! NOTE: Full CLEAR-SWI processing is not yet implemented. This binary accepts
//! all CLI flags for compatibility with existing scripts but prints a warning.

use anyhow::{Context, Result};
use clap::Parser;

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
        "WARNING: CLEAR-SWI full processing is not yet implemented in this Rust port. \
         The binary accepts all CLI flags for compatibility."
    );

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

    Ok(())
}
