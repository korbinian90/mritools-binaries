//! Integration tests for the `mcpc3ds` CLI binary.
//!
//! Mirrors the test cases from the original Julia CompileMRI.jl test suite
//! (test/mcpc3ds_test.jl).

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

/// Test: `mcpc3ds -p Phase.nii -m Mag.nii -t 1:3 -N -o <tmpfile>`
///
/// From mcpc3ds_test.jl: `args = ["-p", phasefile, "-m", magfile, "-t", "1:3", "-N"]`
#[test]
fn mcpc3ds_phase_mag_echo_range_no_mmap() {
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
            "-N",
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
