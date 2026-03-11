//! Shared utilities for mritools CLI binaries.
//!
//! Provides NIfTI I/O helpers and other common functions shared across the
//! romeo, clearswi, mcpc3ds, makehomogeneous and romeo_mask binaries.

pub use qsm_core::nifti_io::{load_nifti, load_nifti_4d, save_nifti, NiftiData};

/// Read a NIfTI file from disk and return a [`NiftiData`] struct.
///
/// Supports both `.nii` and `.nii.gz` files. For 4D data, returns only the
/// first 3D volume; use [`read_nifti_4d`] for multi-echo data.
pub fn read_nifti(path: &str) -> anyhow::Result<NiftiData> {
    let bytes =
        std::fs::read(path).map_err(|e| anyhow::anyhow!("Cannot open '{}': {}", path, e))?;
    load_nifti(&bytes).map_err(|e| anyhow::anyhow!("Failed to parse NIfTI '{}': {}", path, e))
}

/// 4D NIfTI data loaded from bytes.
pub struct NiftiData4D {
    /// Volume data for each time point / echo.
    pub volumes: Vec<Vec<f64>>,
    /// 3D dimensions (nx, ny, nz).
    pub dims: (usize, usize, usize),
    /// Number of time points / echoes.
    pub nt: usize,
    /// Voxel sizes in mm.
    pub voxel_size: (f64, f64, f64),
    /// Affine transformation matrix (4×4, row-major).
    pub affine: [f64; 16],
}

/// Read a 4D NIfTI file from disk and return a [`NiftiData4D`] struct.
///
/// The data is split into per-echo volumes.
pub fn read_nifti_4d(path: &str) -> anyhow::Result<NiftiData4D> {
    let bytes =
        std::fs::read(path).map_err(|e| anyhow::anyhow!("Cannot open '{}': {}", path, e))?;
    let (data, (nx, ny, nz, nt), voxel_size, affine) = load_nifti_4d(&bytes)
        .map_err(|e| anyhow::anyhow!("Failed to parse 4D NIfTI '{}': {}", path, e))?;

    let n_voxels = nx * ny * nz;
    let mut volumes = Vec::with_capacity(nt);
    for t in 0..nt {
        let start = t * n_voxels;
        let end = start + n_voxels;
        volumes.push(data[start..end].to_vec());
    }

    Ok(NiftiData4D {
        volumes,
        dims: (nx, ny, nz),
        nt,
        voxel_size,
        affine,
    })
}

/// Write a NIfTI file to disk.
///
/// The output path is used as-is; callers should append `.nii` or `.nii.gz`
/// as appropriate.
pub fn write_nifti(path: &str, nii: &NiftiData) -> anyhow::Result<()> {
    let bytes = save_nifti(&nii.data, nii.dims, nii.voxel_size, &nii.affine)
        .map_err(|e| anyhow::anyhow!("Failed to encode NIfTI '{}': {}", path, e))?;
    std::fs::write(path, bytes).map_err(|e| anyhow::anyhow!("Cannot write '{}': {}", path, e))?;
    Ok(())
}

/// Write a 3D data array as a NIfTI file, using the header info from a [`NiftiData4D`].
pub fn write_nifti_from_4d(path: &str, data: &[f64], nii4d: &NiftiData4D) -> anyhow::Result<()> {
    let bytes = save_nifti(data, nii4d.dims, nii4d.voxel_size, &nii4d.affine)
        .map_err(|e| anyhow::anyhow!("Failed to encode NIfTI '{}': {}", path, e))?;
    std::fs::write(path, bytes).map_err(|e| anyhow::anyhow!("Cannot write '{}': {}", path, e))?;
    Ok(())
}

/// Parse an echo-times argument that may be a Julia-style array (`"[1.5,3.0]"`)
/// or range (`"3.5:3.5:14"`), or a plain list of numbers.
///
/// Returns a vector of echo times in milliseconds.
pub fn parse_echo_times(args: &[String]) -> anyhow::Result<Vec<f64>> {
    if args.is_empty() {
        return Ok(vec![]);
    }
    // Handle "epi" keyword (identical echo times)
    if args[0].eq_ignore_ascii_case("epi") {
        let te = if args.len() > 1 {
            args[1]
                .parse::<f64>()
                .map_err(|_| anyhow::anyhow!("Invalid echo time after 'epi': {}", args[1]))?
        } else {
            1.0
        };
        return Ok(vec![te]);
    }

    let joined = args.join(" ");

    // Julia-style range: start:step:stop or start:stop (step defaults to 1)
    if joined.contains(':') {
        let parts: Vec<&str> = joined.split(':').collect();
        if parts.len() == 3 {
            let start: f64 = parts[0].trim().parse()?;
            let step: f64 = parts[1].trim().parse()?;
            let stop: f64 = parts[2].trim().parse()?;
            if step == 0.0 {
                return Err(anyhow::anyhow!("Echo time range step cannot be zero"));
            }
            let mut tes = vec![];
            let mut t = start;
            if step > 0.0 {
                while t <= stop + 1e-9 {
                    tes.push(t);
                    t += step;
                }
            } else {
                while t >= stop - 1e-9 {
                    tes.push(t);
                    t += step;
                }
            }
            return Ok(tes);
        } else if parts.len() == 2 {
            let start: f64 = parts[0].trim().parse()?;
            let stop: f64 = parts[1].trim().parse()?;
            let mut tes = vec![];
            let mut t = start;
            while t <= stop + 1e-9 {
                tes.push(t);
                t += 1.0;
            }
            return Ok(tes);
        }
    }

    // Julia-style array: [1.5,3.0,...] or bare space/comma separated numbers
    let cleaned = joined
        .trim_start_matches('[')
        .trim_end_matches(']')
        .replace(',', " ");
    let mut tes = vec![];
    for tok in cleaned.split_whitespace() {
        let v: f64 = tok
            .parse()
            .map_err(|_| anyhow::anyhow!("Invalid echo time value: '{}'", tok))?;
        tes.push(v);
    }
    Ok(tes)
}

/// Write a 4D NIfTI file from a list of 3D volumes, using header info from a [`NiftiData4D`].
pub fn write_nifti_4d(path: &str, volumes: &[Vec<f64>], nii4d: &NiftiData4D) -> anyhow::Result<()> {
    // Concatenate all volumes into a flat array
    let n_voxels = nii4d.dims.0 * nii4d.dims.1 * nii4d.dims.2;
    let nt = volumes.len();
    let mut flat = Vec::with_capacity(n_voxels * nt);
    for vol in volumes {
        flat.extend_from_slice(vol);
    }
    let bytes = save_nifti_4d_raw(&flat, nii4d.dims, nt, nii4d.voxel_size, &nii4d.affine)
        .map_err(|e| anyhow::anyhow!("Failed to encode 4D NIfTI '{}': {}", path, e))?;
    std::fs::write(path, bytes).map_err(|e| anyhow::anyhow!("Cannot write '{}': {}", path, e))?;
    Ok(())
}

/// Build a minimal 4D NIfTI file from raw data.
///
/// This writes a NIfTI-1 header with dim[0]=4 so readers see the time dimension.
fn save_nifti_4d_raw(
    data: &[f64],
    dims: (usize, usize, usize),
    nt: usize,
    voxel_size: (f64, f64, f64),
    affine: &[f64; 16],
) -> Result<Vec<u8>, String> {
    use std::io::Write;

    let (nx, ny, nz) = dims;
    let (vsx, vsy, vsz) = voxel_size;

    // Create NIfTI-1 header (348 bytes)
    let mut header = [0u8; 348];

    // sizeof_hdr = 348
    header[0..4].copy_from_slice(&348i32.to_le_bytes());

    // dim[0..7]  – dim[0]=4 signals a 4D dataset
    let dim: [i16; 8] = [4, nx as i16, ny as i16, nz as i16, nt as i16, 1, 1, 1];
    for (i, &d) in dim.iter().enumerate() {
        let offset = 40 + i * 2;
        header[offset..offset + 2].copy_from_slice(&d.to_le_bytes());
    }

    // datatype = 16 (FLOAT32), bitpix = 32
    header[70..72].copy_from_slice(&16i16.to_le_bytes());
    header[72..74].copy_from_slice(&32i16.to_le_bytes());

    // pixdim
    let pixdim: [f32; 8] = [0.0, vsx as f32, vsy as f32, vsz as f32, 1.0, 1.0, 1.0, 1.0];
    for (i, &p) in pixdim.iter().enumerate() {
        let offset = 76 + i * 4;
        header[offset..offset + 4].copy_from_slice(&p.to_le_bytes());
    }

    // vox_offset = 352.0 (data starts right after header + 4 byte extension)
    header[108..112].copy_from_slice(&352.0f32.to_le_bytes());

    // scl_slope = 1.0, scl_inter = 0.0
    header[112..116].copy_from_slice(&1.0f32.to_le_bytes());
    header[116..120].copy_from_slice(&0.0f32.to_le_bytes());

    // sform_code = 1 (scanner anat)
    header[254..256].copy_from_slice(&1i16.to_le_bytes());

    // srow_x, srow_y, srow_z from affine
    for row in 0..3 {
        for col in 0..4 {
            let val = affine[row * 4 + col] as f32;
            let offset = 280 + row * 16 + col * 4;
            header[offset..offset + 4].copy_from_slice(&val.to_le_bytes());
        }
    }

    // magic = "n+1\0"
    header[344..348].copy_from_slice(b"n+1\0");

    // Build file: header (348) + 4-byte extension + float32 data
    let total_voxels = data.len();
    let data_size = total_voxels * 4;
    let file_size = 348 + 4 + data_size;
    let mut buf = Vec::with_capacity(file_size);
    buf.write_all(&header).map_err(|e| e.to_string())?;
    buf.write_all(&[0u8; 4]).map_err(|e| e.to_string())?; // extension bytes

    for &v in data {
        buf.write_all(&(v as f32).to_le_bytes())
            .map_err(|e| e.to_string())?;
    }

    Ok(buf)
}

/// Parse an echo-selection argument (Julia-style indices/ranges).
///
/// Supports:
/// - `":"` → select all echoes (returns `None`)
/// - `"3"` → select echo 3 (1-based)
/// - `"[1,3]"` or `"1:3"` → select echoes 1,2,3 (1-based)
///
/// Returns 0-based indices.
pub fn parse_echo_selection(args: &[String], n_echoes: usize) -> Option<Vec<usize>> {
    if args.is_empty() {
        return None;
    }
    let joined = args.join(" ").trim().to_string();
    if joined == ":" {
        return None; // all echoes
    }

    // Try as a Julia range  "start:stop"  or  "start:step:stop"
    if joined.contains(':') {
        let parts: Vec<&str> = joined.split(':').collect();
        if parts.len() == 2 {
            if let (Ok(start), Ok(stop)) = (
                parts[0].trim().parse::<usize>(),
                parts[1].trim().parse::<usize>(),
            ) {
                let indices: Vec<usize> = (start..=stop)
                    .filter(|&i| i >= 1 && i <= n_echoes)
                    .map(|i| i - 1)
                    .collect();
                if !indices.is_empty() {
                    return Some(indices);
                }
            }
        } else if parts.len() == 3 {
            if let (Ok(start), Ok(step), Ok(stop)) = (
                parts[0].trim().parse::<usize>(),
                parts[1].trim().parse::<usize>(),
                parts[2].trim().parse::<usize>(),
            ) {
                let mut indices = Vec::new();
                let mut i = start;
                while i <= stop {
                    if i >= 1 && i <= n_echoes {
                        indices.push(i - 1);
                    }
                    i += step;
                }
                if !indices.is_empty() {
                    return Some(indices);
                }
            }
        }
    }

    // Try as a Julia-style array "[1,3]" or bare number(s)
    let cleaned = joined
        .trim_start_matches('[')
        .trim_end_matches(']')
        .replace(',', " ");
    let indices: Vec<usize> = cleaned
        .split_whitespace()
        .filter_map(|s| s.parse::<usize>().ok())
        .filter(|&i| i >= 1 && i <= n_echoes)
        .map(|i| i - 1)
        .collect();
    if !indices.is_empty() {
        Some(indices)
    } else {
        None
    }
}

/// Fix GE phase slice-jump artefacts.
///
/// Walks through z-slices from k=1 onward, comparing each slice's mean phase
/// to the *previous* slice (k−1).  When the difference exceeds π, the current
/// slice is corrected by the closest integer multiple of 2π.  Corrections are
/// cumulative: the updated mean is used for subsequent comparisons.
///
/// Operates in-place on the `phase` array.
pub fn fix_ge_phase_slices(phase: &mut [f64], nx: usize, ny: usize, nz: usize) {
    let pi = std::f64::consts::PI;
    let two_pi = 2.0 * pi;
    let n_xy = nx * ny;

    // Calculate mean phase per slice
    let mut means: Vec<f64> = Vec::with_capacity(nz);
    for k in 0..nz {
        let start = k * n_xy;
        let end = start + n_xy;
        let sum: f64 = phase[start..end].iter().sum();
        means.push(sum / n_xy as f64);
    }

    // Detect and correct 2π jumps between adjacent slices
    for k in 1..nz {
        let diff = means[k] - means[k - 1];
        if diff.abs() > pi {
            let correction = -(diff / two_pi).round() * two_pi;
            let start = k * n_xy;
            let end = start + n_xy;
            for v in &mut phase[start..end] {
                *v += correction;
            }
            // Update mean for subsequent comparisons
            means[k] += correction;
        }
    }
}

/// Select a subset of volumes from a [`NiftiData4D`] by 0-based indices.
pub fn select_volumes(nii: &NiftiData4D, indices: &[usize]) -> NiftiData4D {
    let volumes: Vec<Vec<f64>> = indices
        .iter()
        .filter(|&&i| i < nii.nt)
        .map(|&i| nii.volumes[i].clone())
        .collect();
    let nt = volumes.len();
    NiftiData4D {
        volumes,
        dims: nii.dims,
        nt,
        voxel_size: nii.voxel_size,
        affine: nii.affine,
    }
}

/// Save a human-readable settings file to `<dir>/settings_<tool>.txt`.
pub fn save_settings(dir: &str, tool: &str, args: &[String]) -> anyhow::Result<()> {
    let path = std::path::Path::new(dir).join(format!("settings_{}.txt", tool));
    let content = format!("Arguments: {}\n", args.join(" "));
    std::fs::write(&path, content)
        .map_err(|e| anyhow::anyhow!("Cannot write settings file '{}': {}", path.display(), e))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_echo_times_array() {
        let args = vec!["[1.5,3.0,4.5]".to_string()];
        let tes = parse_echo_times(&args).unwrap();
        assert_eq!(tes, vec![1.5, 3.0, 4.5]);
    }

    #[test]
    fn parse_echo_times_range() {
        let args = vec!["3.5:3.5:10.5".to_string()];
        let tes = parse_echo_times(&args).unwrap();
        assert_eq!(tes.len(), 3);
        assert!((tes[0] - 3.5).abs() < 1e-9);
        assert!((tes[1] - 7.0).abs() < 1e-9);
        assert!((tes[2] - 10.5).abs() < 1e-9);
    }

    #[test]
    fn parse_echo_times_two_part_range() {
        let args = vec!["1:3".to_string()];
        let tes = parse_echo_times(&args).unwrap();
        assert_eq!(tes.len(), 3);
        assert!((tes[0] - 1.0).abs() < 1e-9);
        assert!((tes[1] - 2.0).abs() < 1e-9);
        assert!((tes[2] - 3.0).abs() < 1e-9);
    }

    #[test]
    fn parse_echo_times_range_zero_step_error() {
        let args = vec!["1:0:3".to_string()];
        let result = parse_echo_times(&args);
        assert!(result.is_err(), "expected error for zero step");
    }

    #[test]
    fn parse_echo_times_epi() {
        let args = vec!["epi".to_string(), "5.3".to_string()];
        let tes = parse_echo_times(&args).unwrap();
        assert_eq!(tes, vec![5.3]);
    }

    #[test]
    fn parse_echo_selection_colon() {
        let args = vec![":".to_string()];
        assert!(parse_echo_selection(&args, 5).is_none());
    }

    #[test]
    fn parse_echo_selection_single() {
        let args = vec!["2".to_string()];
        let sel = parse_echo_selection(&args, 5).unwrap();
        assert_eq!(sel, vec![1]); // 0-based
    }

    #[test]
    fn parse_echo_selection_range() {
        let args = vec!["1:3".to_string()];
        let sel = parse_echo_selection(&args, 5).unwrap();
        assert_eq!(sel, vec![0, 1, 2]);
    }

    #[test]
    fn parse_echo_selection_array() {
        let args = vec!["[1, 3]".to_string()];
        let sel = parse_echo_selection(&args, 5).unwrap();
        assert_eq!(sel, vec![0, 2]);
    }

    #[test]
    fn fix_ge_phase_basic() {
        let pi = std::f64::consts::PI;
        let nx = 2;
        let ny = 2;
        let nz = 3;
        // Slice 0: mean ~0, Slice 1: mean ~2π (jump), Slice 2: mean ~0
        let mut phase = vec![
            0.0, 0.0, 0.0, 0.0, // slice 0
            6.28, 6.28, 6.28, 6.28, // slice 1 (2π jump)
            0.1, 0.1, 0.1, 0.1, // slice 2
        ];
        fix_ge_phase_slices(&mut phase, nx, ny, nz);
        // After correction, slice 1 should be close to 0
        let mean_slice1: f64 = phase[4..8].iter().sum::<f64>() / 4.0;
        assert!(
            mean_slice1.abs() < pi,
            "slice 1 mean should be close to 0, got {}",
            mean_slice1
        );
    }
}
