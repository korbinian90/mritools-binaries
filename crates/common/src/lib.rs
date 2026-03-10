//! Shared utilities for mritools CLI binaries.
//!
//! Provides NIfTI I/O helpers and other common functions shared across the
//! romeo, clearswi, mcpc3ds, makehomogeneous and romeo_mask binaries.

pub use qsm_core::nifti_io::{load_nifti, save_nifti, NiftiData};

/// Read a NIfTI file from disk and return a [`NiftiData`] struct.
///
/// Supports both `.nii` and `.nii.gz` files.
pub fn read_nifti(path: &str) -> anyhow::Result<NiftiData> {
    let bytes =
        std::fs::read(path).map_err(|e| anyhow::anyhow!("Cannot open '{}': {}", path, e))?;
    load_nifti(&bytes).map_err(|e| anyhow::anyhow!("Failed to parse NIfTI '{}': {}", path, e))
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
}
