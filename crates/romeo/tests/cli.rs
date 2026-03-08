//! Integration tests for the `romeo` CLI binary.
//!
//! Mirrors the test cases from the original Julia CompileMRI.jl test suite
//! (test/romeo_test.jl).

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

fn romeo_bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_romeo"))
}

/// Test: `romeo -p Phase.nii -B -t 1:3 -o <tmpfile>`
///
/// From romeo_test.jl line: `args = ["-p", phasefile, "-B", "-t", "1:3"]`
#[test]
fn romeo_phase_b0_echo_range() {
    let tmpdir = tempfile::tempdir().unwrap();
    let output = tmpdir.path().join("unwrapped.nii");
    let status = romeo_bin()
        .args([
            "-p",
            &phase_file(),
            "-B",
            "-t",
            "1:3",
            "-o",
            output.to_str().unwrap(),
        ])
        .status()
        .expect("failed to execute romeo");
    assert!(status.success(), "romeo exited with: {}", status);
    assert!(output.exists(), "output file was not created");
}

/// Test: `romeo -p Phase.nii -m Mag.nii -t 1:3 -o <tmpfile>`
///
/// From romeo_test.jl line: `args = ["-p", phasefile, "-m", magfile, "-t", "1:3"]`
#[test]
fn romeo_phase_mag_echo_range() {
    let tmpdir = tempfile::tempdir().unwrap();
    let output = tmpdir.path().join("unwrapped.nii");
    let status = romeo_bin()
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
        .expect("failed to execute romeo");
    assert!(status.success(), "romeo exited with: {}", status);
    assert!(output.exists(), "output file was not created");
}
