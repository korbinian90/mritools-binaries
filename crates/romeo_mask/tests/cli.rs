//! Integration tests for the `romeo_mask` CLI binary.
//!
//! Mirrors the test cases from the original Julia CompileMRI.jl test suite
//! (test/romeo_mask_test.jl).

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
