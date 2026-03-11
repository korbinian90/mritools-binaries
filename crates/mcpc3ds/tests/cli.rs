//! Integration tests for the `mcpc3ds` CLI binary.
//!
//! Mirrors the test cases from the original Julia CompileMRI.jl test suite
//! (test/mcpc3ds_test.jl) plus additional tests for comprehensive coverage
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

fn mcpc3ds_bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_mcpc3ds"))
}

// ===== Julia CompileMRI.jl parity tests =====

/// Test: `mcpc3ds -p Phase.nii -m Mag.nii -t 1:3 -o <tmpfile>`
///
/// From mcpc3ds_test.jl: `args = ["-p", phasefile, "-m", magfile, "-t", "1:3"]`
#[test]
fn mcpc3ds_phase_mag_echo_range() {
    let tmpdir = tempfile::tempdir().unwrap();
    let output = tmpdir.path().join("output");
    let status = mcpc3ds_bin()
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
        .expect("failed to execute mcpc3ds");
    assert!(status.success(), "mcpc3ds exited with: {}", status);
    assert!(
        tmpdir.path().join("settings_mcpc3ds.txt").exists(),
        "settings_mcpc3ds.txt was not created"
    );
}

/// Test: `mcpc3ds -p Phase.nii -m Mag.nii -t 1:3 --write-phase-offsets -o <tmpfile>`
///
/// From mcpc3ds_test.jl: `args = ["-p", phasefile, "-m", magfile, "-t", "1:3", "--write-phase-offsets"]`
#[test]
fn mcpc3ds_phase_mag_write_phase_offsets() {
    let tmpdir = tempfile::tempdir().unwrap();
    let output = tmpdir.path().join("output");
    let status = mcpc3ds_bin()
        .args([
            "-p",
            &phase_file(),
            "-m",
            &mag_file(),
            "-t",
            "1:3",
            "--write-phase-offsets",
            "-o",
            output.to_str().unwrap(),
        ])
        .status()
        .expect("failed to execute mcpc3ds");
    assert!(status.success(), "mcpc3ds exited with: {}", status);
    assert!(
        tmpdir.path().join("settings_mcpc3ds.txt").exists(),
        "settings_mcpc3ds.txt was not created"
    );
}

// ===== Output file validation tests =====

/// Verify the main output NIfTI exists and has valid size.
#[test]
fn mcpc3ds_output_file_exists_and_nonempty() {
    let tmpdir = tempfile::tempdir().unwrap();
    let output = tmpdir.path().join("output");
    let status = mcpc3ds_bin()
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
        .expect("failed to execute mcpc3ds");
    assert!(status.success());
    let nii_path = tmpdir.path().join("output.nii");
    assert!(nii_path.exists(), "output.nii was not created");
    let meta = std::fs::metadata(&nii_path).unwrap();
    assert!(
        meta.len() > 352,
        "output NIfTI file is too small ({} bytes), expected at least a NIfTI header",
        meta.len()
    );
}

/// Verify phase offset file is created when --write-phase-offsets is specified.
#[test]
fn mcpc3ds_phase_offset_file_created() {
    let tmpdir = tempfile::tempdir().unwrap();
    let output = tmpdir.path().join("output");
    let status = mcpc3ds_bin()
        .args([
            "-p",
            &phase_file(),
            "-m",
            &mag_file(),
            "-t",
            "1:3",
            "--write-phase-offsets",
            "-o",
            output.to_str().unwrap(),
        ])
        .status()
        .expect("failed to execute mcpc3ds");
    assert!(status.success());
    let po_path = tmpdir.path().join("output_phase_offset.nii");
    assert!(
        po_path.exists(),
        "phase offset file was not created at {}",
        po_path.display()
    );
    let meta = std::fs::metadata(&po_path).unwrap();
    assert!(
        meta.len() > 352,
        "phase offset NIfTI is too small ({} bytes)",
        meta.len()
    );
}

/// Verify phase offset file is NOT created without --write-phase-offsets.
#[test]
fn mcpc3ds_no_phase_offset_by_default() {
    let tmpdir = tempfile::tempdir().unwrap();
    let output = tmpdir.path().join("output");
    let status = mcpc3ds_bin()
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
        .expect("failed to execute mcpc3ds");
    assert!(status.success());
    let po_path = tmpdir.path().join("output_phase_offset.nii");
    assert!(
        !po_path.exists(),
        "phase offset file should not be created without --write-phase-offsets"
    );
}

// ===== Verbose mode =====

/// Verbose output should include processing details.
#[test]
fn mcpc3ds_verbose_output() {
    let tmpdir = tempfile::tempdir().unwrap();
    let output = tmpdir.path().join("output");
    let result = mcpc3ds_bin()
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
        .expect("failed to execute mcpc3ds");
    assert!(result.status.success());
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        stderr.contains("MCPC-3D-S"),
        "verbose output should contain 'MCPC-3D-S'"
    );
    assert!(
        stderr.contains("dims:"),
        "verbose output should contain dimensions"
    );
    assert!(
        stderr.contains("echoes"),
        "verbose output should contain echo count"
    );
    assert!(
        stderr.contains("smoothing sigma:"),
        "verbose output should contain smoothing sigma"
    );
    assert!(
        stderr.contains("mask voxels:"),
        "verbose output should contain mask info"
    );
    assert!(
        stderr.contains("phase combination complete"),
        "verbose output should confirm completion"
    );
}

// ===== Phase-only mode (no magnitude) =====

/// mcpc3ds should work with phase only (magnitude defaults to uniform).
#[test]
fn mcpc3ds_phase_only() {
    let tmpdir = tempfile::tempdir().unwrap();
    let output = tmpdir.path().join("output");
    let status = mcpc3ds_bin()
        .args([
            "-p",
            &phase_file(),
            "-t",
            "1:3",
            "-o",
            output.to_str().unwrap(),
        ])
        .status()
        .expect("failed to execute mcpc3ds");
    assert!(status.success(), "mcpc3ds should work without magnitude");
    let nii_path = tmpdir.path().join("output.nii");
    assert!(nii_path.exists());
}

// ===== --no-phase-rescale =====

/// Test with --no-phase-rescale flag.
#[test]
fn mcpc3ds_no_phase_rescale() {
    let tmpdir = tempfile::tempdir().unwrap();
    let output = tmpdir.path().join("output");
    let status = mcpc3ds_bin()
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
        .expect("failed to execute mcpc3ds");
    assert!(status.success());
    assert!(tmpdir.path().join("output.nii").exists());
}

// ===== Echo times formats =====

/// Test with Julia-style array echo times.
#[test]
fn mcpc3ds_echo_times_array() {
    let tmpdir = tempfile::tempdir().unwrap();
    let output = tmpdir.path().join("output");
    let status = mcpc3ds_bin()
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
        .expect("failed to execute mcpc3ds");
    assert!(status.success());
    assert!(tmpdir.path().join("output.nii").exists());
}

/// Test with 3-part range echo times.
#[test]
fn mcpc3ds_echo_times_range_3part() {
    let tmpdir = tempfile::tempdir().unwrap();
    let output = tmpdir.path().join("output");
    let status = mcpc3ds_bin()
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
        .expect("failed to execute mcpc3ds");
    assert!(status.success());
    assert!(tmpdir.path().join("output.nii").exists());
}

// ===== Smoothing sigma =====

/// Test with custom smoothing sigma.
#[test]
fn mcpc3ds_custom_smoothing_sigma() {
    let tmpdir = tempfile::tempdir().unwrap();
    let output = tmpdir.path().join("output");
    let status = mcpc3ds_bin()
        .args([
            "-p",
            &phase_file(),
            "-m",
            &mag_file(),
            "-t",
            "1:3",
            "-s",
            "[5,5,3]",
            "-o",
            output.to_str().unwrap(),
        ])
        .status()
        .expect("failed to execute mcpc3ds");
    assert!(
        status.success(),
        "mcpc3ds with custom smoothing sigma failed"
    );
    assert!(tmpdir.path().join("output.nii").exists());
}

// ===== Missing required args =====

/// Without -p (phase), the binary should fail.
#[test]
fn mcpc3ds_missing_phase_fails() {
    let tmpdir = tempfile::tempdir().unwrap();
    let output = tmpdir.path().join("output");
    let status = mcpc3ds_bin()
        .args([
            "-m",
            &mag_file(),
            "-t",
            "1:3",
            "-o",
            output.to_str().unwrap(),
        ])
        .status()
        .expect("failed to execute mcpc3ds");
    assert!(!status.success(), "mcpc3ds should fail without --phase");
}

// ===== Settings file content =====

/// Verify settings file contains the command-line arguments.
#[test]
fn mcpc3ds_settings_file_content() {
    let tmpdir = tempfile::tempdir().unwrap();
    let output = tmpdir.path().join("output");
    let status = mcpc3ds_bin()
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
        .expect("failed to execute mcpc3ds");
    assert!(status.success());
    let settings_path = tmpdir.path().join("settings_mcpc3ds.txt");
    assert!(settings_path.exists());
    let content = std::fs::read_to_string(&settings_path).unwrap();
    assert!(content.contains("Arguments:"));
    assert!(content.contains("-p"));
    assert!(content.contains("-m"));
}

// ===== Combined options =====

/// Test with multiple options combined.
#[test]
fn mcpc3ds_combined_options() {
    let tmpdir = tempfile::tempdir().unwrap();
    let output = tmpdir.path().join("output");
    let status = mcpc3ds_bin()
        .args([
            "-p",
            &phase_file(),
            "-m",
            &mag_file(),
            "-t",
            "[1.5,3.0,4.5]",
            "--write-phase-offsets",
            "-v",
            "-o",
            output.to_str().unwrap(),
        ])
        .status()
        .expect("failed to execute mcpc3ds");
    assert!(status.success());
    assert!(tmpdir.path().join("output.nii").exists());
    assert!(tmpdir.path().join("output_phase_offset.nii").exists());
    assert!(tmpdir.path().join("settings_mcpc3ds.txt").exists());
}
