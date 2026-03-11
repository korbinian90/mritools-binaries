//! Integration tests for the `romeo_mask` CLI binary.
//!
//! Mirrors the test cases from the original Julia CompileMRI.jl test suite
//! (test/romeo_mask_test.jl) plus additional tests for comprehensive coverage
//! of the Rust implementation.

use std::process::Command;

/// Path to the test data directory (relative to workspace root).
fn test_data_dir() -> std::path::PathBuf {
    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest.join("../../test/data/small")
}

fn phase_file() -> String {
    test_data_dir().join("Phase.nii").to_string_lossy().into()
}

fn mag_file() -> String {
    test_data_dir().join("Mag.nii").to_string_lossy().into()
}

fn romeo_mask_bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_romeo_mask"))
}

// ===== Julia CompileMRI.jl parity tests =====

/// Test: `romeo_mask -p Phase.nii -t 1:3 -o <tmpfile>`
///
/// From romeo_mask_test.jl: `args = ["-p", phasefile, "-t", "1:3"]`
#[test]
fn romeo_mask_phase_only() {
    let tmpdir = tempfile::tempdir().unwrap();
    let output = tmpdir.path().join("mask.nii");
    let status = romeo_mask_bin()
        .args([
            "-p",
            &phase_file(),
            "-t",
            "1:3",
            "-o",
            output.to_str().unwrap(),
        ])
        .status()
        .expect("failed to execute romeo_mask");
    assert!(status.success(), "romeo_mask exited with: {}", status);
    assert!(
        tmpdir.path().join("settings_romeo_mask.txt").exists(),
        "settings_romeo_mask.txt was not created"
    );
}

/// Test: `romeo_mask -p Phase.nii -t 1:3 -m Mag.nii -o <tmpfile>`
///
/// From romeo_mask_test.jl: `args = ["-p", phasefile, "-t", "1:3", "-m", magfile]`
#[test]
fn romeo_mask_with_magnitude() {
    let tmpdir = tempfile::tempdir().unwrap();
    let output = tmpdir.path().join("mask.nii");
    let status = romeo_mask_bin()
        .args([
            "-p",
            &phase_file(),
            "-t",
            "1:3",
            "-m",
            &mag_file(),
            "-o",
            output.to_str().unwrap(),
        ])
        .status()
        .expect("failed to execute romeo_mask");
    assert!(status.success(), "romeo_mask exited with: {}", status);
    assert!(
        tmpdir.path().join("settings_romeo_mask.txt").exists(),
        "settings_romeo_mask.txt was not created"
    );
}

/// Test: `romeo_mask -p Phase.nii -t 1:3 -e 1 -o <tmpfile>`
///
/// From romeo_mask_test.jl: `args = ["-p", phasefile, "-t", "1:3", "-e", "1"]`
#[test]
fn romeo_mask_single_echo() {
    let tmpdir = tempfile::tempdir().unwrap();
    let output = tmpdir.path().join("mask.nii");
    let status = romeo_mask_bin()
        .args([
            "-p",
            &phase_file(),
            "-t",
            "1:3",
            "-e",
            "1",
            "-o",
            output.to_str().unwrap(),
        ])
        .status()
        .expect("failed to execute romeo_mask");
    assert!(status.success(), "romeo_mask exited with: {}", status);
    assert!(
        tmpdir.path().join("settings_romeo_mask.txt").exists(),
        "settings_romeo_mask.txt was not created"
    );
}

/// Test: `romeo_mask -p Phase.nii -t 1:3 -e "[1, 2]" -o <tmpfile>`
///
/// From romeo_mask_test.jl: `args = ["-p", phasefile, "-t", "1:3", "-e", "[1, 2]"]`
#[test]
fn romeo_mask_two_echoes() {
    let tmpdir = tempfile::tempdir().unwrap();
    let output = tmpdir.path().join("mask.nii");
    let status = romeo_mask_bin()
        .args([
            "-p",
            &phase_file(),
            "-t",
            "1:3",
            "-e",
            "[1, 2]",
            "-o",
            output.to_str().unwrap(),
        ])
        .status()
        .expect("failed to execute romeo_mask");
    assert!(status.success(), "romeo_mask exited with: {}", status);
    assert!(
        tmpdir.path().join("settings_romeo_mask.txt").exists(),
        "settings_romeo_mask.txt was not created"
    );
}

/// Test: `romeo_mask -p Phase.nii -t 1:3 -w romeo4 -o <tmpfile>`
///
/// From romeo_mask_test.jl: `args = ["-p", phasefile, "-t", "1:3", "-w", "romeo4"]`
#[test]
fn romeo_mask_weights_romeo4() {
    let tmpdir = tempfile::tempdir().unwrap();
    let output = tmpdir.path().join("mask.nii");
    let status = romeo_mask_bin()
        .args([
            "-p",
            &phase_file(),
            "-t",
            "1:3",
            "-w",
            "romeo4",
            "-o",
            output.to_str().unwrap(),
        ])
        .status()
        .expect("failed to execute romeo_mask");
    assert!(status.success(), "romeo_mask exited with: {}", status);
    assert!(
        tmpdir.path().join("settings_romeo_mask.txt").exists(),
        "settings_romeo_mask.txt was not created"
    );
}

/// Test: `romeo_mask -p Phase.nii -t 1:3 -w bestpath -o <tmpfile>`
///
/// From romeo_mask_test.jl: `args = ["-p", phasefile, "-t", "1:3", "-w", "bestpath"]`
#[test]
fn romeo_mask_weights_bestpath() {
    let tmpdir = tempfile::tempdir().unwrap();
    let output = tmpdir.path().join("mask.nii");
    let status = romeo_mask_bin()
        .args([
            "-p",
            &phase_file(),
            "-t",
            "1:3",
            "-w",
            "bestpath",
            "-o",
            output.to_str().unwrap(),
        ])
        .status()
        .expect("failed to execute romeo_mask");
    assert!(status.success(), "romeo_mask exited with: {}", status);
    assert!(
        tmpdir.path().join("settings_romeo_mask.txt").exists(),
        "settings_romeo_mask.txt was not created"
    );
}

/// Test: `romeo_mask -p Phase.nii -t 1:3 -w 100011 -o <tmpfile>`
///
/// From romeo_mask_test.jl: `args = ["-p", phasefile, "-t", "1:3", "-w", "100011"]`
#[test]
fn romeo_mask_weights_flags() {
    let tmpdir = tempfile::tempdir().unwrap();
    let output = tmpdir.path().join("mask.nii");
    let status = romeo_mask_bin()
        .args([
            "-p",
            &phase_file(),
            "-t",
            "1:3",
            "-w",
            "100011",
            "-o",
            output.to_str().unwrap(),
        ])
        .status()
        .expect("failed to execute romeo_mask");
    assert!(status.success(), "romeo_mask exited with: {}", status);
    assert!(
        tmpdir.path().join("settings_romeo_mask.txt").exists(),
        "settings_romeo_mask.txt was not created"
    );
}

/// Test: `romeo_mask -p Phase.nii -t 1:3 --no-phase-rescale -o <tmpfile>`
///
/// From romeo_mask_test.jl: `args = ["-p", phasefile, "-t", "1:3", "--no-phase-rescale"]`
#[test]
fn romeo_mask_no_phase_rescale() {
    let tmpdir = tempfile::tempdir().unwrap();
    let output = tmpdir.path().join("mask.nii");
    let status = romeo_mask_bin()
        .args([
            "-p",
            &phase_file(),
            "-t",
            "1:3",
            "--no-phase-rescale",
            "-o",
            output.to_str().unwrap(),
        ])
        .status()
        .expect("failed to execute romeo_mask");
    assert!(status.success(), "romeo_mask exited with: {}", status);
    assert!(
        tmpdir.path().join("settings_romeo_mask.txt").exists(),
        "settings_romeo_mask.txt was not created"
    );
}

/// Test: `romeo_mask -p Phase.nii -t 1:3 -v -o <tmpfile>`
///
/// From romeo_mask_test.jl: `args = ["-p", phasefile, "-t", "1:3", "-v"]`
#[test]
fn romeo_mask_verbose() {
    let tmpdir = tempfile::tempdir().unwrap();
    let output = tmpdir.path().join("mask.nii");
    let status = romeo_mask_bin()
        .args([
            "-p",
            &phase_file(),
            "-t",
            "1:3",
            "-v",
            "-o",
            output.to_str().unwrap(),
        ])
        .status()
        .expect("failed to execute romeo_mask");
    assert!(status.success(), "romeo_mask exited with: {}", status);
    assert!(
        tmpdir.path().join("settings_romeo_mask.txt").exists(),
        "settings_romeo_mask.txt was not created"
    );
}

/// Test: `romeo_mask -p Phase.nii -t 1:3 -Q -o <tmpfile>`
///
/// From romeo_mask_test.jl: `args = ["-p", phasefile, "-t", "1:3", "-Q"]`
#[test]
fn romeo_mask_write_quality_all() {
    let tmpdir = tempfile::tempdir().unwrap();
    let output = tmpdir.path().join("mask.nii");
    let status = romeo_mask_bin()
        .args([
            "-p",
            &phase_file(),
            "-t",
            "1:3",
            "-Q",
            "-o",
            output.to_str().unwrap(),
        ])
        .status()
        .expect("failed to execute romeo_mask");
    assert!(status.success(), "romeo_mask exited with: {}", status);
    assert!(
        tmpdir.path().join("settings_romeo_mask.txt").exists(),
        "settings_romeo_mask.txt was not created"
    );
}

/// Test: `romeo_mask -p Phase.nii -t 1:3 -q -o <tmpfile>`
///
/// From romeo_mask_test.jl: `args = ["-p", phasefile, "-t", "1:3", "-q"]`
#[test]
fn romeo_mask_write_quality() {
    let tmpdir = tempfile::tempdir().unwrap();
    let output = tmpdir.path().join("mask.nii");
    let status = romeo_mask_bin()
        .args([
            "-p",
            &phase_file(),
            "-t",
            "1:3",
            "-q",
            "-o",
            output.to_str().unwrap(),
        ])
        .status()
        .expect("failed to execute romeo_mask");
    assert!(status.success(), "romeo_mask exited with: {}", status);
    assert!(
        tmpdir.path().join("settings_romeo_mask.txt").exists(),
        "settings_romeo_mask.txt was not created"
    );
}

// ===== Output file validation tests =====

/// Verify the mask output NIfTI file exists and has valid size.
#[test]
fn romeo_mask_output_file_exists_and_nonempty() {
    let tmpdir = tempfile::tempdir().unwrap();
    let output = tmpdir.path().join("mask.nii");
    let status = romeo_mask_bin()
        .args([
            "-p",
            &phase_file(),
            "-t",
            "1:3",
            "-o",
            output.to_str().unwrap(),
        ])
        .status()
        .expect("failed to execute romeo_mask");
    assert!(status.success());
    assert!(output.exists(), "mask.nii was not created");
    let meta = std::fs::metadata(&output).unwrap();
    assert!(
        meta.len() > 352,
        "output NIfTI file is too small ({} bytes), expected at least a NIfTI header",
        meta.len()
    );
}

/// Verify the quality map file is created when -q is given.
#[test]
fn romeo_mask_quality_file_created() {
    let tmpdir = tempfile::tempdir().unwrap();
    let output = tmpdir.path().join("mask.nii");
    let status = romeo_mask_bin()
        .args([
            "-p",
            &phase_file(),
            "-t",
            "1:3",
            "-q",
            "-o",
            output.to_str().unwrap(),
        ])
        .status()
        .expect("failed to execute romeo_mask");
    assert!(status.success());
    let quality_path = tmpdir.path().join("mask_quality.nii");
    assert!(
        quality_path.exists(),
        "quality map file was not created at {}",
        quality_path.display()
    );
    let meta = std::fs::metadata(&quality_path).unwrap();
    assert!(
        meta.len() > 352,
        "quality NIfTI file is too small ({} bytes)",
        meta.len()
    );
}

/// Verify per-axis quality map files are created when -Q is given.
#[test]
fn romeo_mask_quality_all_files_created() {
    let tmpdir = tempfile::tempdir().unwrap();
    let output = tmpdir.path().join("mask.nii");
    let status = romeo_mask_bin()
        .args([
            "-p",
            &phase_file(),
            "-t",
            "1:3",
            "-Q",
            "-o",
            output.to_str().unwrap(),
        ])
        .status()
        .expect("failed to execute romeo_mask");
    assert!(status.success());
    for suffix in &["quality_x", "quality_y", "quality_z"] {
        let q_path = tmpdir.path().join(format!("mask_{}.nii", suffix));
        assert!(
            q_path.exists(),
            "{} quality file was not created at {}",
            suffix,
            q_path.display()
        );
        let meta = std::fs::metadata(&q_path).unwrap();
        assert!(meta.len() > 352, "{} NIfTI file is too small", suffix);
    }
}

/// Verify quality map is NOT created without -q flag.
#[test]
fn romeo_mask_no_quality_by_default() {
    let tmpdir = tempfile::tempdir().unwrap();
    let output = tmpdir.path().join("mask.nii");
    let status = romeo_mask_bin()
        .args([
            "-p",
            &phase_file(),
            "-t",
            "1:3",
            "-o",
            output.to_str().unwrap(),
        ])
        .status()
        .expect("failed to execute romeo_mask");
    assert!(status.success());
    let quality_path = tmpdir.path().join("mask_quality.nii");
    assert!(
        !quality_path.exists(),
        "quality map should not be created without -q"
    );
}

// ===== Verbose output content =====

/// Verbose output should include key processing details.
#[test]
fn romeo_mask_verbose_output_content() {
    let tmpdir = tempfile::tempdir().unwrap();
    let output = tmpdir.path().join("mask.nii");
    let result = romeo_mask_bin()
        .args([
            "-p",
            &phase_file(),
            "-t",
            "1:3",
            "-v",
            "-o",
            output.to_str().unwrap(),
        ])
        .output()
        .expect("failed to execute romeo_mask");
    assert!(result.status.success());
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        stderr.contains("romeo_mask"),
        "verbose output should contain 'romeo_mask'"
    );
    assert!(
        stderr.contains("phase:"),
        "verbose output should contain phase path"
    );
    assert!(
        stderr.contains("output:"),
        "verbose output should contain output path"
    );
    assert!(
        stderr.contains("factor:"),
        "verbose output should contain factor value"
    );
    assert!(
        stderr.contains("quality range:"),
        "verbose output should contain quality range"
    );
    assert!(
        stderr.contains("threshold:"),
        "verbose output should contain threshold"
    );
    assert!(
        stderr.contains("mask voxels:"),
        "verbose output should contain mask voxel count"
    );
    assert!(
        stderr.contains("saved to:"),
        "verbose output should confirm save"
    );
}

// ===== Factor parameter =====

/// Test with a different factor value (0.5 = more restrictive mask).
#[test]
fn romeo_mask_factor_high() {
    let tmpdir = tempfile::tempdir().unwrap();
    let output = tmpdir.path().join("mask.nii");
    let status = romeo_mask_bin()
        .args([
            "-p",
            &phase_file(),
            "-t",
            "1:3",
            "-f",
            "0.5",
            "-o",
            output.to_str().unwrap(),
        ])
        .status()
        .expect("failed to execute romeo_mask");
    assert!(status.success());
    assert!(output.exists());
}

/// Test with a very low factor (0.01 = very permissive mask).
#[test]
fn romeo_mask_factor_low() {
    let tmpdir = tempfile::tempdir().unwrap();
    let output = tmpdir.path().join("mask.nii");
    let status = romeo_mask_bin()
        .args([
            "-p",
            &phase_file(),
            "-t",
            "1:3",
            "-f",
            "0.01",
            "-o",
            output.to_str().unwrap(),
        ])
        .status()
        .expect("failed to execute romeo_mask");
    assert!(status.success());
    assert!(output.exists());
}

// ===== Echo times formats =====

/// Test with Julia-style array echo times.
#[test]
fn romeo_mask_echo_times_array() {
    let tmpdir = tempfile::tempdir().unwrap();
    let output = tmpdir.path().join("mask.nii");
    let status = romeo_mask_bin()
        .args([
            "-p",
            &phase_file(),
            "-t",
            "[1.5,3.0,4.5]",
            "-o",
            output.to_str().unwrap(),
        ])
        .status()
        .expect("failed to execute romeo_mask");
    assert!(status.success());
    assert!(output.exists());
}

/// Test with 3-part range echo times.
#[test]
fn romeo_mask_echo_times_range_3part() {
    let tmpdir = tempfile::tempdir().unwrap();
    let output = tmpdir.path().join("mask.nii");
    let status = romeo_mask_bin()
        .args([
            "-p",
            &phase_file(),
            "-t",
            "3.5:3.5:10.5",
            "-o",
            output.to_str().unwrap(),
        ])
        .status()
        .expect("failed to execute romeo_mask");
    assert!(status.success());
    assert!(output.exists());
}

// ===== Missing required args =====

/// Without -p (phase), the binary should fail.
#[test]
fn romeo_mask_missing_phase_fails() {
    let tmpdir = tempfile::tempdir().unwrap();
    let output = tmpdir.path().join("mask.nii");
    let status = romeo_mask_bin()
        .args(["-t", "1:3", "-o", output.to_str().unwrap()])
        .status()
        .expect("failed to execute romeo_mask");
    assert!(!status.success(), "romeo_mask should fail without --phase");
}

// ===== Settings file content =====

/// Verify settings file contains command-line arguments.
#[test]
fn romeo_mask_settings_file_content() {
    let tmpdir = tempfile::tempdir().unwrap();
    let output = tmpdir.path().join("mask.nii");
    let status = romeo_mask_bin()
        .args([
            "-p",
            &phase_file(),
            "-t",
            "1:3",
            "-o",
            output.to_str().unwrap(),
        ])
        .status()
        .expect("failed to execute romeo_mask");
    assert!(status.success());
    let settings_path = tmpdir.path().join("settings_romeo_mask.txt");
    assert!(settings_path.exists());
    let content = std::fs::read_to_string(&settings_path).unwrap();
    assert!(content.contains("Arguments:"));
    assert!(content.contains("-p"));
    assert!(content.contains("-t"));
}

// ===== Combined options =====

/// Test with multiple options combined (like a real workflow).
#[test]
fn romeo_mask_combined_options() {
    let tmpdir = tempfile::tempdir().unwrap();
    let output = tmpdir.path().join("mask.nii");
    let status = romeo_mask_bin()
        .args([
            "-p",
            &phase_file(),
            "-m",
            &mag_file(),
            "-t",
            "1:3",
            "-f",
            "0.2",
            "-q",
            "-Q",
            "-v",
            "-o",
            output.to_str().unwrap(),
        ])
        .status()
        .expect("failed to execute romeo_mask");
    assert!(status.success());
    assert!(output.exists());
    assert!(tmpdir.path().join("mask_quality.nii").exists());
    assert!(tmpdir.path().join("mask_quality_x.nii").exists());
    assert!(tmpdir.path().join("mask_quality_y.nii").exists());
    assert!(tmpdir.path().join("mask_quality_z.nii").exists());
    assert!(tmpdir.path().join("settings_romeo_mask.txt").exists());
}

// ===== Mask with magnitude produces different result than without =====

/// The mask should be valid (non-empty output) both with and without magnitude.
#[test]
fn romeo_mask_with_and_without_magnitude_both_succeed() {
    let tmpdir1 = tempfile::tempdir().unwrap();
    let output1 = tmpdir1.path().join("mask.nii");
    let status1 = romeo_mask_bin()
        .args([
            "-p",
            &phase_file(),
            "-t",
            "1:3",
            "-o",
            output1.to_str().unwrap(),
        ])
        .status()
        .expect("failed to execute romeo_mask");
    assert!(status1.success());
    assert!(output1.exists());

    let tmpdir2 = tempfile::tempdir().unwrap();
    let output2 = tmpdir2.path().join("mask.nii");
    let status2 = romeo_mask_bin()
        .args([
            "-p",
            &phase_file(),
            "-m",
            &mag_file(),
            "-t",
            "1:3",
            "-o",
            output2.to_str().unwrap(),
        ])
        .status()
        .expect("failed to execute romeo_mask");
    assert!(status2.success());
    assert!(output2.exists());

    // Both should produce valid NIfTI files with data
    let size1 = std::fs::metadata(&output1).unwrap().len();
    let size2 = std::fs::metadata(&output2).unwrap().len();
    assert!(size1 > 352);
    assert!(size2 > 352);
}
