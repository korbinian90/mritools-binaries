//! makehomogeneous — Homogeneity correction CLI for high-field MRI
//!
//! Matches the Julia CLI interface from korbinian90/CompileMRI.jl (HomogeneityCorrection.jl).
//!
//! Reference:
//!   Eckstein, K., Trattnig, S., Robinson, S.D. (2019). "A Simple Homogeneity
//!   Correction for Neuroimaging at 7T." Proc. ISMRM 27th Annual Meeting.

use anyhow::{Context, Result};
use clap::Parser;
use mritools_common::{read_nifti, read_nifti_4d, write_nifti, write_nifti_4d, save_settings};

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

    /// Output datatype (e.g. Float32, Float64, Int16, Int32) [default: same as input]
    #[arg(short = 'd', long)]
    datatype: Option<String>,

    /// Verbose output
    #[arg(short = 'v', long)]
    verbose: bool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

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
        if let Some(ref dt) = cli.datatype {
            eprintln!("  datatype:         {}", dt);
        }
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
    save_settings(output_dir, "makehomogeneous", &args)?;

    // Load magnitude image as 4D to detect multi-echo
    let mag_4d = read_nifti_4d(magnitude)
        .with_context(|| format!("Failed to read magnitude image '{}'", magnitude))?;

    let (nx, ny, nz) = mag_4d.dims;
    let (vx, vy, vz) = mag_4d.voxel_size;
    let n_echoes = mag_4d.nt;

    if cli.verbose {
        eprintln!("  dims: {}x{}x{}, {} echoes", nx, ny, nz, n_echoes);
        eprintln!("  voxel size: {:.3}x{:.3}x{:.3} mm", vx, vy, vz);
    }

    let out_path = if cli.output.ends_with(".nii.gz") || cli.output.ends_with(".nii") {
        cli.output.clone()
    } else {
        format!("{}.nii", cli.output)
    };

    if n_echoes == 1 {
        // Single echo: process as 3D
        let corrected = qsm_core::utils::makehomogeneous(
            &mag_4d.volumes[0],
            nx, ny, nz,
            vx, vy, vz,
            cli.sigma_bias_field,
            cli.nbox.max(1) as usize,
        );

        if cli.verbose {
            eprintln!("  homogeneity correction complete");
        }

        // Apply datatype conversion
        let output_data = apply_datatype_conversion(&corrected, cli.datatype.as_deref());

        let mag_nii = read_nifti(magnitude)?;
        let mut out_nii = mag_nii;
        out_nii.data = output_data;
        write_nifti(&out_path, &out_nii)
            .with_context(|| format!("Failed to write output '{}'", out_path))?;
    } else {
        // Multi-echo: process each echo independently
        let mut corrected_volumes = Vec::with_capacity(n_echoes);

        for e in 0..n_echoes {
            let corrected = qsm_core::utils::makehomogeneous(
                &mag_4d.volumes[e],
                nx, ny, nz,
                vx, vy, vz,
                cli.sigma_bias_field,
                cli.nbox.max(1) as usize,
            );

            let output_data = apply_datatype_conversion(&corrected, cli.datatype.as_deref());
            corrected_volumes.push(output_data);

            if cli.verbose {
                eprintln!("  echo {} of {} corrected", e + 1, n_echoes);
            }
        }

        if cli.verbose {
            eprintln!("  homogeneity correction complete");
        }

        write_nifti_4d(&out_path, &corrected_volumes, &mag_4d)
            .with_context(|| format!("Failed to write 4D output '{}'", out_path))?;
    }

    if cli.verbose {
        eprintln!("  saved to: {}", out_path);
    }

    Ok(())
}

/// Apply datatype conversion to output data.
///
/// Supported types: Float32, Float64, Int16, Int32, UInt8, UInt16.
/// The output NIfTI is always stored as float64 internally, but we round/clip
/// to emulate the requested integer types.
fn apply_datatype_conversion(data: &[f64], datatype: Option<&str>) -> Vec<f64> {
    match datatype {
        Some(dt) => match dt.to_lowercase().as_str() {
            "float32" | "f32" => data.iter().map(|&v| v as f32 as f64).collect(),
            "float64" | "f64" => data.to_vec(),
            "int16" | "i16" => data
                .iter()
                .map(|&v| v.round().clamp(i16::MIN as f64, i16::MAX as f64))
                .collect(),
            "int32" | "i32" => data
                .iter()
                .map(|&v| v.round().clamp(i32::MIN as f64, i32::MAX as f64))
                .collect(),
            "uint8" | "u8" => data
                .iter()
                .map(|&v| v.round().clamp(0.0, 255.0))
                .collect(),
            "uint16" | "u16" => data
                .iter()
                .map(|&v| v.round().clamp(0.0, u16::MAX as f64))
                .collect(),
            _ => data.to_vec(), // Unknown type, keep as-is
        },
        None => data.to_vec(),
    }
}
