//! Integration tests for the `clearswi` CLI binary.
//!
//! Mirrors the test cases from the original Julia CompileMRI.jl test suite
//! (test/clearswi_test.jl) plus additional tests for comprehensive coverage
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

fn clearswi_bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_clearswi"))
}

// ===== Julia CompileMRI.jl parity tests =====

/// Test: `clearswi -p Phase.nii -m Mag.nii -t 1:3 -o <tmpfile>`
///
/// From clearswi_test.jl: `args = ["-p", phasefile, "-m", magfile, "-t", "1:3"]`
#[test]
fn clearswi_phase_mag_echo_range() {
    let tmpdir = tempfile::tempdir().unwrap();
    let output = tmpdir.path().join("clearswi.nii");
    let status = clearswi_bin()
        .args([
            "-p",
            &phase_file(),
            "-m",
            &mag_file(),
            "-t",
            "1:3",
            "-o",
            output.to_str().unwrap(),
        ])
        .status()
        .expect("failed to execute clearswi");
    assert!(status.success(), "clearswi exited with: {}", status);
    assert!(
        tmpdir.path().join("settings_clearswi.txt").exists(),
        "settings_clearswi.txt was not created"
    );
}

/// Test: `clearswi -p Phase.nii -m Mag.nii -t 1:3 --qsm -o <tmpfile>`
///
/// From clearswi_test.jl: `args = ["-p", phasefile, "-m", magfile, "-t", "1:3", "--qsm"]`
#[test]
fn clearswi_phase_mag_echo_range_qsm() {
    let tmpdir = tempfile::tempdir().unwrap();
    let output = tmpdir.path().join("clearswi.nii");
    let status = clearswi_bin()
        .args([
            "-p",
            &phase_file(),
            "-m",
            &mag_file(),
            "-t",
            "1:3",
            "--qsm",
            "-o",
            output.to_str().unwrap(),
        ])
        .status()
        .expect("failed to execute clearswi");
    assert!(status.success(), "clearswi exited with: {}", status);
    assert!(
        tmpdir.path().join("settings_clearswi.txt").exists(),
        "settings_clearswi.txt was not created"
    );
}

// ===== Output file validation tests =====

/// Verify the main SWI output file is a valid NIfTI with non-zero size.
#[test]
fn clearswi_output_file_exists_and_nonempty() {
    let tmpdir = tempfile::tempdir().unwrap();
    let output = tmpdir.path().join("clearswi.nii");
    let status = clearswi_bin()
        .args([
            "-p",
            &phase_file(),
            "-m",
            &mag_file(),
            "-t",
            "1:3",
            "-o",
            output.to_str().unwrap(),
        ])
        .status()
        .expect("failed to execute clearswi");
    assert!(status.success());
    assert!(output.exists(), "output NIfTI file was not created");
    let meta = std::fs::metadata(&output).unwrap();
    assert!(
        meta.len() > 352,
        "output NIfTI file is too small ({} bytes), expected at least a NIfTI header",
        meta.len()
    );
}

/// Verify that a MIP file is created alongside the SWI output.
#[test]
fn clearswi_mip_file_created() {
    let tmpdir = tempfile::tempdir().unwrap();
    let output = tmpdir.path().join("clearswi.nii");
    let status = clearswi_bin()
        .args([
            "-p",
            &phase_file(),
            "-m",
            &mag_file(),
            "-t",
            "1:3",
            "-o",
            output.to_str().unwrap(),
        ])
        .status()
        .expect("failed to execute clearswi");
    assert!(status.success());
    let mip_path = tmpdir.path().join("clearswi_mip.nii");
    assert!(
        mip_path.exists(),
        "MIP file was not created at {}",
        mip_path.display()
    );
    let meta = std::fs::metadata(&mip_path).unwrap();
    assert!(
        meta.len() > 352,
        "MIP NIfTI file is too small ({} bytes)",
        meta.len()
    );
}

// ===== Verbose mode =====

/// Verbose output should include key processing details.
#[test]
fn clearswi_verbose_output() {
    let tmpdir = tempfile::tempdir().unwrap();
    let output = tmpdir.path().join("clearswi.nii");
    let result = clearswi_bin()
        .args([
            "-p",
            &phase_file(),
            "-m",
            &mag_file(),
            "-t",
            "1:3",
            "-v",
            "-o",
            output.to_str().unwrap(),
        ])
        .output()
        .expect("failed to execute clearswi");
    assert!(result.status.success());
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        stderr.contains("CLEAR-SWI"),
        "verbose output should contain 'CLEAR-SWI'"
    );
    assert!(
        stderr.contains("dims:"),
        "verbose output should contain image dimensions"
    );
    assert!(
        stderr.contains("voxel size:"),
        "verbose output should contain voxel size"
    );
    assert!(
        stderr.contains("filter size:"),
        "verbose output should contain filter size"
    );
    assert!(
        stderr.contains("phase scaling:"),
        "verbose output should contain phase scaling info"
    );
    assert!(
        stderr.contains("SWI calculation complete"),
        "verbose output should confirm completion"
    );
}

// ===== Phase scaling options =====

/// Test all phase scaling types: tanh, negativetanh, positive, negative, triangular.
#[test]
fn clearswi_phase_scaling_tanh() {
    let tmpdir = tempfile::tempdir().unwrap();
    let output = tmpdir.path().join("clearswi.nii");
    let status = clearswi_bin()
        .args([
            "-p",
            &phase_file(),
            "-m",
            &mag_file(),
            "-t",
            "1:3",
            "--phase-scaling-type",
            "tanh",
            "-o",
            output.to_str().unwrap(),
        ])
        .status()
        .expect("failed to execute clearswi");
    assert!(status.success(), "clearswi with tanh scaling failed");
    assert!(output.exists());
}

#[test]
fn clearswi_phase_scaling_negativetanh() {
    let tmpdir = tempfile::tempdir().unwrap();
    let output = tmpdir.path().join("clearswi.nii");
    let status = clearswi_bin()
        .args([
            "-p",
            &phase_file(),
            "-m",
            &mag_file(),
            "-t",
            "1:3",
            "--phase-scaling-type",
            "negativetanh",
            "-o",
            output.to_str().unwrap(),
        ])
        .status()
        .expect("failed to execute clearswi");
    assert!(
        status.success(),
        "clearswi with negativetanh scaling failed"
    );
}

#[test]
fn clearswi_phase_scaling_positive() {
    let tmpdir = tempfile::tempdir().unwrap();
    let output = tmpdir.path().join("clearswi.nii");
    let status = clearswi_bin()
        .args([
            "-p",
            &phase_file(),
            "-m",
            &mag_file(),
            "-t",
            "1:3",
            "--phase-scaling-type",
            "positive",
            "-o",
            output.to_str().unwrap(),
        ])
        .status()
        .expect("failed to execute clearswi");
    assert!(status.success(), "clearswi with positive scaling failed");
}

#[test]
fn clearswi_phase_scaling_negative() {
    let tmpdir = tempfile::tempdir().unwrap();
    let output = tmpdir.path().join("clearswi.nii");
    let status = clearswi_bin()
        .args([
            "-p",
            &phase_file(),
            "-m",
            &mag_file(),
            "-t",
            "1:3",
            "--phase-scaling-type",
            "negative",
            "-o",
            output.to_str().unwrap(),
        ])
        .status()
        .expect("failed to execute clearswi");
    assert!(status.success(), "clearswi with negative scaling failed");
}

#[test]
fn clearswi_phase_scaling_triangular() {
    let tmpdir = tempfile::tempdir().unwrap();
    let output = tmpdir.path().join("clearswi.nii");
    let status = clearswi_bin()
        .args([
            "-p",
            &phase_file(),
            "-m",
            &mag_file(),
            "-t",
            "1:3",
            "--phase-scaling-type",
            "triangular",
            "-o",
            output.to_str().unwrap(),
        ])
        .status()
        .expect("failed to execute clearswi");
    assert!(status.success(), "clearswi with triangular scaling failed");
}

// ===== Phase scaling strength =====

/// Test custom phase scaling strength parameter.
#[test]
fn clearswi_phase_scaling_strength() {
    let tmpdir = tempfile::tempdir().unwrap();
    let output = tmpdir.path().join("clearswi.nii");
    let status = clearswi_bin()
        .args([
            "-p",
            &phase_file(),
            "-m",
            &mag_file(),
            "-t",
            "1:3",
            "--phase-scaling-strength",
            "8",
            "-o",
            output.to_str().unwrap(),
        ])
        .status()
        .expect("failed to execute clearswi");
    assert!(status.success());
    assert!(output.exists());
}

// ===== Filter size =====

/// Test custom high-pass filter size.
#[test]
fn clearswi_custom_filter_size() {
    let tmpdir = tempfile::tempdir().unwrap();
    let output = tmpdir.path().join("clearswi.nii");
    let status = clearswi_bin()
        .args([
            "-p",
            &phase_file(),
            "-m",
            &mag_file(),
            "-t",
            "1:3",
            "--filter-size",
            "[2,2,1]",
            "-o",
            output.to_str().unwrap(),
        ])
        .status()
        .expect("failed to execute clearswi");
    assert!(status.success(), "clearswi with custom filter size failed");
    assert!(output.exists());
}

// ===== --no-phase-rescale =====

/// Test with --no-phase-rescale flag.
#[test]
fn clearswi_no_phase_rescale() {
    let tmpdir = tempfile::tempdir().unwrap();
    let output = tmpdir.path().join("clearswi.nii");
    let status = clearswi_bin()
        .args([
            "-p",
            &phase_file(),
            "-m",
            &mag_file(),
            "-t",
            "1:3",
            "--no-phase-rescale",
            "-o",
            output.to_str().unwrap(),
        ])
        .status()
        .expect("failed to execute clearswi");
    assert!(status.success());
    assert!(output.exists());
}

// ===== Softplus scaling =====

/// Test with softplus scaling disabled.
#[test]
fn clearswi_softplus_scaling_off() {
    let tmpdir = tempfile::tempdir().unwrap();
    let output = tmpdir.path().join("clearswi.nii");
    let status = clearswi_bin()
        .args([
            "-p",
            &phase_file(),
            "-m",
            &mag_file(),
            "-t",
            "1:3",
            "--mag-softplus-scaling",
            "off",
            "-o",
            output.to_str().unwrap(),
        ])
        .status()
        .expect("failed to execute clearswi");
    assert!(status.success());
    assert!(output.exists());
}

// ===== MIP slices =====

/// Test with a different number of MIP slices.
#[test]
fn clearswi_custom_mip_slices() {
    let tmpdir = tempfile::tempdir().unwrap();
    let output = tmpdir.path().join("clearswi.nii");
    let status = clearswi_bin()
        .args([
            "-p",
            &phase_file(),
            "-m",
            &mag_file(),
            "-t",
            "1:3",
            "-s",
            "4",
            "-o",
            output.to_str().unwrap(),
        ])
        .status()
        .expect("failed to execute clearswi");
    assert!(status.success());
    let mip_path = tmpdir.path().join("clearswi_mip.nii");
    assert!(mip_path.exists(), "MIP file should be created with -s 4");
}

// ===== Echo times formats =====

/// Test with Julia-style array echo times.
#[test]
fn clearswi_echo_times_array() {
    let tmpdir = tempfile::tempdir().unwrap();
    let output = tmpdir.path().join("clearswi.nii");
    let status = clearswi_bin()
        .args([
            "-p",
            &phase_file(),
            "-m",
            &mag_file(),
            "-t",
            "[1.5,3.0,4.5]",
            "-o",
            output.to_str().unwrap(),
        ])
        .status()
        .expect("failed to execute clearswi");
    assert!(status.success());
    assert!(output.exists());
}

/// Test with 3-part range echo times.
#[test]
fn clearswi_echo_times_range_3part() {
    let tmpdir = tempfile::tempdir().unwrap();
    let output = tmpdir.path().join("clearswi.nii");
    let status = clearswi_bin()
        .args([
            "-p",
            &phase_file(),
            "-m",
            &mag_file(),
            "-t",
            "3.5:3.5:10.5",
            "-o",
            output.to_str().unwrap(),
        ])
        .status()
        .expect("failed to execute clearswi");
    assert!(status.success());
    assert!(output.exists());
}

// ===== Missing required args =====

/// Without -m (magnitude), the binary should fail.
#[test]
fn clearswi_missing_magnitude_fails() {
    let tmpdir = tempfile::tempdir().unwrap();
    let output = tmpdir.path().join("clearswi.nii");
    let status = clearswi_bin()
        .args([
            "-p",
            &phase_file(),
            "-t",
            "1:3",
            "-o",
            output.to_str().unwrap(),
        ])
        .status()
        .expect("failed to execute clearswi");
    assert!(
        !status.success(),
        "clearswi should fail without --magnitude"
    );
}

// ===== Settings file content =====

/// Verify the settings file contains the correct command-line arguments.
#[test]
fn clearswi_settings_file_content() {
    let tmpdir = tempfile::tempdir().unwrap();
    let output = tmpdir.path().join("clearswi.nii");
    let status = clearswi_bin()
        .args([
            "-p",
            &phase_file(),
            "-m",
            &mag_file(),
            "-t",
            "1:3",
            "-o",
            output.to_str().unwrap(),
        ])
        .status()
        .expect("failed to execute clearswi");
    assert!(status.success());
    let settings_path = tmpdir.path().join("settings_clearswi.txt");
    assert!(settings_path.exists());
    let content = std::fs::read_to_string(&settings_path).unwrap();
    assert!(
        content.contains("Arguments:"),
        "settings file should contain 'Arguments:'"
    );
    assert!(
        content.contains("-p"),
        "settings file should contain the phase flag"
    );
    assert!(
        content.contains("-m"),
        "settings file should contain the magnitude flag"
    );
}

// ===== Magnitude-only mode (no phase) =====

/// Test running with only magnitude (no phase). Phase defaults to zeros.
#[test]
fn clearswi_magnitude_only() {
    let tmpdir = tempfile::tempdir().unwrap();
    let output = tmpdir.path().join("clearswi.nii");
    let status = clearswi_bin()
        .args([
            "-m",
            &mag_file(),
            "-t",
            "1:3",
            "-o",
            output.to_str().unwrap(),
        ])
        .status()
        .expect("failed to execute clearswi");
    assert!(
        status.success(),
        "clearswi should succeed with magnitude only"
    );
    assert!(output.exists());
}

// ===== Combined options =====

/// Test with multiple options combined (like a real workflow).
#[test]
fn clearswi_combined_options() {
    let tmpdir = tempfile::tempdir().unwrap();
    let output = tmpdir.path().join("clearswi.nii");
    let status = clearswi_bin()
        .args([
            "-p",
            &phase_file(),
            "-m",
            &mag_file(),
            "-t",
            "1:3",
            "--phase-scaling-type",
            "negativetanh",
            "--phase-scaling-strength",
            "6",
            "--filter-size",
            "[3,3,0]",
            "--mag-softplus-scaling",
            "off",
            "-s",
            "5",
            "-v",
            "-o",
            output.to_str().unwrap(),
        ])
        .status()
        .expect("failed to execute clearswi");
    assert!(status.success());
    assert!(output.exists());
    let mip_path = tmpdir.path().join("clearswi_mip.nii");
    assert!(mip_path.exists());
}
