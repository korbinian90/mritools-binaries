//! Integration tests for the `clearswi` CLI binary.
//!
//! Mirrors the test cases from the original Julia CompileMRI.jl test suite
//! (test/clearswi_test.jl).

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
}
