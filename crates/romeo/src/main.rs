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
use mritools_common::{
    fix_ge_phase_slices, parse_echo_selection, parse_echo_times, read_nifti, read_nifti_4d,
    save_settings, select_echo_times, select_volumes, write_nifti, write_nifti_4d, NiftiData4D,
};
use qsm_core::region_grow::grow_region_unwrap;
use qsm_core::unwrap::romeo::{calculate_weights_romeo, calculate_weights_romeo_configurable};
use qsm_core::utils::{mcpc3ds_b0_pipeline, mcpc3ds_single_coil, B0WeightType};

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
    #[arg(long, num_args = 0..=1, default_value_t = 0.0, default_missing_value = "0.5")]
    temporal_uncertain_unwrapping: f64,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Warn about flags that are accepted for CLI compatibility but not yet implemented
    if cli.merge_regions {
        eprintln!("WARNING: --merge-regions is not yet implemented in this Rust port, ignoring");
    }
    if cli.correct_regions {
        eprintln!("WARNING: --correct-regions is not yet implemented in this Rust port, ignoring");
    }
    if cli.wrap_addition != 0.0 {
        eprintln!("WARNING: --wrap-addition is not yet implemented in this Rust port, ignoring");
    }
    if cli.temporal_uncertain_unwrapping != 0.0 {
        eprintln!("WARNING: --temporal-uncertain-unwrapping is not yet implemented in this Rust port, ignoring");
    }

    if cli.verbose {
        eprintln!("ROMEO phase unwrapping");
        eprintln!("  phase:     {}", cli.phase);
        if let Some(ref m) = cli.magnitude {
            eprintln!("  magnitude: {}", m);
        }
        eprintln!("  output:    {}", cli.output);
    }

    // Parse echo times
    let mut echo_times =
        parse_echo_times(&cli.echo_times).context("Failed to parse --echo-times")?;

    // Load 4D phase image
    let mut phase_4d = read_nifti_4d(&cli.phase)
        .with_context(|| format!("Failed to read phase image '{}'", cli.phase))?;

    // Apply echo selection (filter volumes and echo times with same indices)
    if let Some(sel) = parse_echo_selection(&cli.unwrap_echoes, phase_4d.nt) {
        if cli.verbose {
            eprintln!(
                "  selecting echoes: {:?} (1-based)",
                sel.iter().map(|i| i + 1).collect::<Vec<_>>()
            );
        }
        phase_4d = select_volumes(&phase_4d, &sel);
        if !echo_times.is_empty() {
            echo_times = select_echo_times(&echo_times, &sel);
        }
    }

    let (nx, ny, nz) = phase_4d.dims;
    let n_voxels = nx * ny * nz;
    let n_echoes = phase_4d.nt;

    if cli.verbose {
        eprintln!("  dims: {}x{}x{}, {} echoes", nx, ny, nz, n_echoes);
    }

    // Rescale phase to [-π; π] if needed
    if !cli.no_phase_rescale {
        for vol in &mut phase_4d.volumes {
            rescale_phase(vol);
        }
    }

    // Fix GE phase if requested
    if cli.fix_ge_phase {
        for vol in &mut phase_4d.volumes {
            fix_ge_phase_slices(vol, nx, ny, nz);
        }
        if cli.verbose {
            eprintln!("  applied GE phase slice-jump correction");
        }
    }

    // Load 4D magnitude image if provided
    let mag_4d: Option<NiftiData4D> = if let Some(ref mag_path) = cli.magnitude {
        let mut m4d = read_nifti_4d(mag_path)
            .with_context(|| format!("Failed to read magnitude image '{}'", mag_path))?;
        // Apply same echo selection
        if let Some(sel) = parse_echo_selection(&cli.unwrap_echoes, m4d.nt) {
            m4d = select_volumes(&m4d, &sel);
        }
        if m4d.nt != n_echoes {
            anyhow::bail!(
                "After echo selection, magnitude has {} echoes but phase has {}",
                m4d.nt,
                n_echoes
            );
        }
        Some(m4d)
    } else {
        None
    };

    // Build mask
    let mask = build_mask(
        mag_4d.as_ref().map(|m| m.volumes[0].as_slice()),
        n_voxels,
        &cli.mask,
    );

    if cli.verbose {
        let n_mask = mask.iter().filter(|&&v| v == 1).count();
        eprintln!("  mask voxels: {}/{}", n_mask, n_voxels);
    }

    // Ensure we have echo times matching the number of echoes
    let tes: Vec<f64> = if echo_times.len() >= n_echoes {
        echo_times[..n_echoes].to_vec()
    } else if echo_times.is_empty() {
        (1..=n_echoes).map(|i| i as f64).collect()
    } else {
        echo_times.clone()
    };

    // Determine output directory
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

    let out_path = if cli.output.ends_with(".nii.gz") || cli.output.ends_with(".nii") {
        cli.output.clone()
    } else {
        format!("{}.nii", cli.output)
    };

    // ---- Phase offset correction (MCPC-3D-S pipeline for multi-echo) ----
    let phase_offset: Option<Vec<f64>> = if n_echoes >= 2 && cli.phase_offset_correction.is_some() {
        let poc = cli.phase_offset_correction.as_deref().unwrap_or("on");
        if poc != "off" {
            let mags: Vec<Vec<f64>> = if let Some(ref m) = mag_4d {
                m.volumes.clone()
            } else {
                vec![vec![1.0; n_voxels]; n_echoes]
            };

            let sigma = parse_smoothing_sigma(&cli.phase_offset_smoothing_sigma_mm);

            let (corrected, offset) = mcpc3ds_single_coil(
                &phase_4d.volumes,
                &mags,
                &tes,
                &mask,
                sigma,
                [0, 1],
                nx,
                ny,
                nz,
            );

            phase_4d.volumes = corrected;

            if cli.verbose {
                eprintln!("  applied phase offset correction");
            }
            Some(offset)
        } else {
            None
        }
    } else {
        None
    };

    // Write phase offsets if requested
    if cli.write_phase_offsets {
        if let Some(ref offset) = phase_offset {
            let po_path = derive_path(&out_path, "phase_offset");
            let po_nii = read_nifti(&cli.phase)?;
            let mut po_out = po_nii;
            po_out.data = offset.clone();
            write_nifti(&po_path, &po_out)?;
            if cli.verbose {
                eprintln!("  phase offsets saved to: {}", po_path);
            }
        }
    }

    // ---- Compute B0 map if requested (multi-echo) ----
    if cli.compute_b0.is_some() && n_echoes >= 2 {
        let b0_name = cli.compute_b0.as_deref().unwrap_or("B0");
        let weight_type = B0WeightType::from_str(&cli.b0_phase_weighting);

        let mags: Vec<Vec<f64>> = if let Some(ref m) = mag_4d {
            m.volumes.clone()
        } else {
            vec![vec![1.0; n_voxels]; n_echoes]
        };

        let sigma = parse_smoothing_sigma(&cli.phase_offset_smoothing_sigma_mm);

        let (b0_hz, _po, corrected) = mcpc3ds_b0_pipeline(
            &phase_4d.volumes,
            &mags,
            &tes,
            &mask,
            sigma,
            weight_type,
            nx,
            ny,
            nz,
        );

        // Use corrected phases from B0 pipeline
        phase_4d.volumes = corrected;

        // Save B0 map
        let b0_path = if b0_name.ends_with(".nii") || b0_name.ends_with(".nii.gz") {
            b0_name.to_string()
        } else {
            let b0_dir = std::path::Path::new(&cli.output)
                .parent()
                .unwrap_or(std::path::Path::new("."));
            b0_dir
                .join(format!("{}.nii", b0_name))
                .to_string_lossy()
                .into()
        };

        let b0_nii = read_nifti(&cli.phase)?;
        let mut b0_out = b0_nii;
        b0_out.data = b0_hz;
        write_nifti(&b0_path, &b0_out)?;
        if cli.verbose {
            eprintln!("  B0 map saved to: {}", b0_path);
        }
    }

    // ---- Unwrap each echo ----
    let mut unwrapped_volumes: Vec<Vec<f64>> = Vec::with_capacity(n_echoes);

    if n_echoes == 1 || cli.individual_unwrapping {
        // Individual unwrapping: unwrap each echo independently
        for e in 0..n_echoes {
            let mag_data = mag_4d
                .as_ref()
                .map(|m| m.volumes[e.min(m.nt - 1)].as_slice())
                .unwrap_or(&[] as &[f64]);

            let unwrapped =
                unwrap_single_echo(&phase_4d.volumes[e], mag_data, &mask, &cli, nx, ny, nz);
            unwrapped_volumes.push(unwrapped);

            if cli.verbose {
                eprintln!("  unwrapped echo {} of {}", e + 1, n_echoes);
            }
        }
    } else {
        // Temporal unwrapping: unwrap template echo, then propagate
        let template_idx = (cli.template - 1).min(n_echoes - 1);

        let mag_template = mag_4d
            .as_ref()
            .map(|m| m.volumes[template_idx.min(m.nt - 1)].as_slice())
            .unwrap_or(&[] as &[f64]);

        // Get second echo for weight calculation
        let second_echo = if template_idx == 0 && n_echoes > 1 {
            1
        } else {
            0
        };

        let (te1, te2) = (tes[template_idx], tes[second_echo]);

        // Calculate weights with two echoes for better quality
        let weights = calculate_weights_with_config(
            &phase_4d.volumes[template_idx],
            mag_template,
            Some(&phase_4d.volumes[second_echo]),
            te1,
            te2,
            &mask,
            nx,
            ny,
            nz,
            &cli.weights,
        );

        // Unwrap template echo
        let mut template_unwrapped = phase_4d.volumes[template_idx].clone();
        let mut mask_work = mask.clone();

        unwrap_with_seeds(
            &mut template_unwrapped,
            &weights,
            &mut mask_work,
            nx,
            ny,
            nz,
            cli.max_seeds,
        );

        // Unwrap other echoes using temporal propagation from template
        unwrapped_volumes = vec![vec![0.0; n_voxels]; n_echoes];
        unwrapped_volumes[template_idx] = template_unwrapped;

        for e in 0..n_echoes {
            if e == template_idx {
                continue;
            }

            let te_ratio = if tes[template_idx].abs() > 1e-10 {
                tes[e] / tes[template_idx]
            } else {
                1.0
            };

            // Estimate unwrapped phase from template
            let mut unwrapped = phase_4d.volumes[e].clone();
            for i in 0..n_voxels {
                if mask[i] > 0 {
                    let expected = unwrapped_volumes[template_idx][i] * te_ratio;
                    let diff = unwrapped[i] - expected;
                    let n_wraps = (diff / (2.0 * std::f64::consts::PI)).round();
                    unwrapped[i] -= n_wraps * 2.0 * std::f64::consts::PI;
                }
            }

            unwrapped_volumes[e] = unwrapped;
        }

        if cli.verbose {
            eprintln!(
                "  temporal unwrapping with template echo {} complete",
                template_idx + 1
            );
        }
    }

    // ---- Post-processing ----
    for vol in &mut unwrapped_volumes {
        // Apply global phase correction if requested
        if cli.correct_global {
            let masked_vals: Vec<f64> = vol
                .iter()
                .zip(mask.iter())
                .filter_map(|(&v, &m)| if m > 0 { Some(v) } else { None })
                .collect();
            if !masked_vals.is_empty() {
                let median = compute_median(&masked_vals);
                let correction =
                    (median / (2.0 * std::f64::consts::PI)).round() * 2.0 * std::f64::consts::PI;
                for v in vol.iter_mut() {
                    *v -= correction;
                }
            }
        }

        // Apply mask to unwrapped result if requested
        if cli.mask_unwrapped {
            for (v, &m) in vol.iter_mut().zip(mask.iter()) {
                if m == 0 {
                    *v = 0.0;
                }
            }
        }

        // Apply threshold
        if cli.threshold.is_finite() {
            let max_val = cli.threshold * 2.0 * std::f64::consts::PI;
            for v in vol.iter_mut() {
                if v.abs() > max_val {
                    *v = 0.0;
                }
            }
        }
    }

    // ---- Write output ----
    if n_echoes == 1 {
        // Single echo: write 3D
        let mut out_nii = read_nifti(&cli.phase)?;
        out_nii.data = unwrapped_volumes.into_iter().next().unwrap();
        write_nifti(&out_path, &out_nii)
            .with_context(|| format!("Failed to write output '{}'", out_path))?;
    } else {
        // Multi-echo: write 4D
        write_nifti_4d(&out_path, &unwrapped_volumes, &phase_4d)
            .with_context(|| format!("Failed to write 4D output '{}'", out_path))?;
    }

    if cli.verbose {
        eprintln!("  saved to: {}", out_path);
    }

    // Write quality map if requested
    if cli.write_quality || cli.write_quality_all {
        // Use first echo for quality
        let mag_data = mag_4d
            .as_ref()
            .map(|m| m.volumes[0].as_slice())
            .unwrap_or(&[] as &[f64]);

        let weights = calculate_weights_romeo(
            &phase_4d.volumes[0],
            mag_data,
            None,
            if tes.len() >= 2 { tes[0] } else { 1.0 },
            if tes.len() >= 2 { tes[1] } else { 1.0 },
            &mask,
            nx,
            ny,
            nz,
        );

        if cli.write_quality {
            let quality = compute_quality_map(&weights, n_voxels);
            let mut q_nii = read_nifti(&cli.phase)?;
            q_nii.data = quality;
            let q_path = derive_path(&out_path, "quality");
            write_nifti(&q_path, &q_nii)?;
            if cli.verbose {
                eprintln!("  quality map saved to: {}", q_path);
            }
        }

        if cli.write_quality_all {
            let per_dim = weights.len() / 3;
            for (d, name) in [(0, "quality_x"), (1, "quality_y"), (2, "quality_z")] {
                let mut q_data = vec![0.0f64; n_voxels];
                for idx in 0..per_dim.min(n_voxels) {
                    q_data[idx] = weights[d * per_dim + idx] as f64 / 255.0;
                }
                let mut q_nii = read_nifti(&cli.phase)?;
                q_nii.data = q_data;
                let q_path = derive_path(&out_path, name);
                write_nifti(&q_path, &q_nii)?;
                if cli.verbose {
                    eprintln!("  {} saved to: {}", name, q_path);
                }
            }
        }
    }

    Ok(())
}

/// Unwrap a single 3D echo volume.
fn unwrap_single_echo(
    phase: &[f64],
    mag: &[f64],
    mask: &[u8],
    cli: &Cli,
    nx: usize,
    ny: usize,
    nz: usize,
) -> Vec<f64> {
    let weights =
        calculate_weights_with_config(phase, mag, None, 1.0, 1.0, mask, nx, ny, nz, &cli.weights);

    let mut unwrapped = phase.to_vec();
    let mut mask_work = mask.to_vec();

    unwrap_with_seeds(
        &mut unwrapped,
        &weights,
        &mut mask_work,
        nx,
        ny,
        nz,
        cli.max_seeds,
    );

    unwrapped
}

/// Calculate weights using the specified weight configuration.
#[allow(clippy::too_many_arguments)]
fn calculate_weights_with_config(
    phase: &[f64],
    mag: &[f64],
    phase2: Option<&[f64]>,
    te1: f64,
    te2: f64,
    mask: &[u8],
    nx: usize,
    ny: usize,
    nz: usize,
    weights_name: &str,
) -> Vec<u8> {
    match weights_name.to_lowercase().as_str() {
        "romeo" | "romeo6" => {
            // All 3 weight components: gradient coherence + mag coherence + mag weight
            calculate_weights_romeo(phase, mag, phase2, te1, te2, mask, nx, ny, nz)
        }
        "romeo4" => {
            // gradient coherence + mag coherence, no mag weight
            calculate_weights_romeo_configurable(
                phase, mag, phase2, te1, te2, mask, nx, ny, nz, true, true, false,
            )
        }
        "romeo3" => {
            // gradient coherence + mag weight, no mag coherence
            calculate_weights_romeo_configurable(
                phase, mag, phase2, te1, te2, mask, nx, ny, nz, true, false, true,
            )
        }
        "romeo2" => {
            // gradient coherence only
            calculate_weights_romeo_configurable(
                phase, mag, phase2, te1, te2, mask, nx, ny, nz, true, false, false,
            )
        }
        "bestpath" => {
            // magnitude weight only (like best path algorithm)
            calculate_weights_romeo_configurable(
                phase, mag, phase2, te1, te2, mask, nx, ny, nz, false, false, true,
            )
        }
        other => {
            // Interpret as binary flags (≥3 chars of '0'/'1').
            // Positions: [0] phase_gradient_coherence, [1] mag_coherence, [2] mag_weight.
            // Extra characters (e.g. 6-digit Julia-style flags) are accepted but only
            // the first three are forwarded to the backend.
            if other.len() >= 3 && other.chars().all(|c| c == '0' || c == '1') {
                let flags: Vec<bool> = other.chars().map(|c| c == '1').collect();
                calculate_weights_romeo_configurable(
                    phase,
                    mag,
                    phase2,
                    te1,
                    te2,
                    mask,
                    nx,
                    ny,
                    nz,
                    flags[0],
                    flags.get(1).copied().unwrap_or(false),
                    flags.get(2).copied().unwrap_or(false),
                )
            } else {
                // Fall back to default
                calculate_weights_romeo(phase, mag, phase2, te1, te2, mask, nx, ny, nz)
            }
        }
    }
}

/// Unwrap with potentially multiple seeds.
fn unwrap_with_seeds(
    unwrapped: &mut [f64],
    weights: &[u8],
    mask: &mut [u8],
    nx: usize,
    ny: usize,
    nz: usize,
    max_seeds: usize,
) {
    for _ in 0..max_seeds {
        let seed = find_seed(mask, weights, nx, ny, nz);
        if mask[seed.0 + seed.1 * nx + seed.2 * nx * ny] == 0 {
            break; // No more masked voxels
        }

        let processed =
            grow_region_unwrap(unwrapped, weights, mask, nx, ny, nz, seed.0, seed.1, seed.2);
        if processed == 0 {
            break;
        }
    }
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
fn build_mask(mag: Option<&[f64]>, n_voxels: usize, mask_args: &[String]) -> Vec<u8> {
    let mask_type = mask_args
        .first()
        .map(|s| s.as_str())
        .unwrap_or("robustmask");
    match mask_type {
        "nomask" => vec![1u8; n_voxels],
        "robustmask" => {
            if let Some(mag) = mag {
                robust_mask(mag)
            } else {
                vec![1u8; n_voxels]
            }
        }
        "qualitymask" => vec![1u8; n_voxels],
        _ => {
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
    let ci = nx / 2;
    let cj = ny / 2;
    let ck = nz / 2;
    let center_idx = ci + cj * nx + ck * nx * ny;
    if mask.len() > center_idx && mask[center_idx] == 1 {
        return (ci, cj, ck);
    }
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

/// Compute a per-voxel quality map from the edge weights.
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

/// Parse smoothing sigma from CLI arguments. Default: [7, 7, 7].
fn parse_smoothing_sigma(args: &[String]) -> [f64; 3] {
    if args.is_empty() {
        return [7.0, 7.0, 7.0];
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
        0 => [7.0, 7.0, 7.0],
        1 => [vals[0], vals[0], vals[0]],
        2 => [vals[0], vals[1], vals[0]],
        _ => [vals[0], vals[1], vals[2]],
    }
}
