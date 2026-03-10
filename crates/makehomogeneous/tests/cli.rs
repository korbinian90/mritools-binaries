//! Integration tests for the `makehomogeneous` CLI binary.
//!
//! Mirrors the test cases from the original Julia CompileMRI.jl test suite
//! (test/makehomogeneous_test.jl).

use std::process::Command;

/// Path to the test data directory (relative to workspace root).
fn test_data_dir() -> std::path::PathBuf {
    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest.join("../../test/data/small")
}

fn mag_file() -> String {
    test_data_dir().join("Mag.nii").to_string_lossy().into()
}

fn makehomogeneous_bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_makehomogeneous"))
}

/// Test: `makehomogeneous -m Mag.nii -s 3 -o <tmpfile>`
///
/// From makehomogeneous_test.jl: `args = ["-m", magfile, "-s", "3"]`
#[test]
fn makehomogeneous_sigma_int() {
    let tmpdir = tempfile::tempdir().unwrap();
    let output = tmpdir.path().join("homogenous");
    let status = makehomogeneous_bin()
        .args(["-m", &mag_file(), "-s", "3", "-o", output.to_str().unwrap()])
        .status()
        .expect("failed to execute makehomogeneous");
    assert!(status.success(), "makehomogeneous exited with: {}", status);
    assert!(
        tmpdir.path().join("settings_makehomogeneous.txt").exists(),
        "settings_makehomogeneous.txt was not created"
    );
}

/// Test: `makehomogeneous -m Mag.nii -s 3.5 -o <tmpfile>`
///
/// From makehomogeneous_test.jl: `args = ["-m", magfile, "-s", "3.5"]`
#[test]
fn makehomogeneous_sigma_float() {
    let tmpdir = tempfile::tempdir().unwrap();
    let output = tmpdir.path().join("homogenous");
    let status = makehomogeneous_bin()
        .args([
            "-m",
            &mag_file(),
            "-s",
            "3.5",
            "-o",
            output.to_str().unwrap(),
        ])
        .status()
        .expect("failed to execute makehomogeneous");
    assert!(status.success(), "makehomogeneous exited with: {}", status);
    assert!(
        tmpdir.path().join("settings_makehomogeneous.txt").exists(),
        "settings_makehomogeneous.txt was not created"
    );
}

/// Test: `makehomogeneous -m Mag.nii -n 4 -o <tmpfile>`
///
/// From makehomogeneous_test.jl: `args = ["-m", magfile, "-n", "4"]`
#[test]
fn makehomogeneous_nbox() {
    let tmpdir = tempfile::tempdir().unwrap();
    let output = tmpdir.path().join("homogenous");
    let status = makehomogeneous_bin()
        .args(["-m", &mag_file(), "-n", "4", "-o", output.to_str().unwrap()])
        .status()
        .expect("failed to execute makehomogeneous");
    assert!(status.success(), "makehomogeneous exited with: {}", status);
    assert!(
        tmpdir.path().join("settings_makehomogeneous.txt").exists(),
        "settings_makehomogeneous.txt was not created"
    );
}

/// Test: `makehomogeneous -m Mag.nii -d Float64 -o <tmpfile>`
///
/// From makehomogeneous_test.jl: `args = ["-m", magfile, "-d", "Float64"]`
#[test]
fn makehomogeneous_datatype_float64() {
    let tmpdir = tempfile::tempdir().unwrap();
    let output = tmpdir.path().join("homogenous");
    let status = makehomogeneous_bin()
        .args([
            "-m",
            &mag_file(),
            "-d",
            "Float64",
            "-o",
            output.to_str().unwrap(),
        ])
        .status()
        .expect("failed to execute makehomogeneous");
    assert!(status.success(), "makehomogeneous exited with: {}", status);
    assert!(
        tmpdir.path().join("settings_makehomogeneous.txt").exists(),
        "settings_makehomogeneous.txt was not created"
    );
}

/// Test: `makehomogeneous -m Mag.nii -d Int32 -o <tmpfile>`
///
/// From makehomogeneous_test.jl: `args = ["-m", magfile, "-d", "Int32"]`
#[test]
fn makehomogeneous_datatype_int32() {
    let tmpdir = tempfile::tempdir().unwrap();
    let output = tmpdir.path().join("homogenous");
    let status = makehomogeneous_bin()
        .args([
            "-m",
            &mag_file(),
            "-d",
            "Int32",
            "-o",
            output.to_str().unwrap(),
        ])
        .status()
        .expect("failed to execute makehomogeneous");
    assert!(status.success(), "makehomogeneous exited with: {}", status);
    assert!(
        tmpdir.path().join("settings_makehomogeneous.txt").exists(),
        "settings_makehomogeneous.txt was not created"
    );
}
