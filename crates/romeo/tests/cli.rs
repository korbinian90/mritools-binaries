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
    // B0 map should be created
    let b0_path = tmpdir.path().join("B0.nii");
    assert!(b0_path.exists(), "B0 map was not created");
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

// ===== 4D processing tests (Phase.nii is 51x51x41x3) =====

/// 4D output: multi-echo phase should produce a 4D output file.
#[test]
fn romeo_4d_multi_echo_output() {
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
    assert!(status.success());
    assert!(output.exists());
    let meta = std::fs::metadata(&output).unwrap();
    // 4D file should be larger than 3D: 51*51*41*3*4 + 352 ≈ 1.28MB
    assert!(
        meta.len() > 352 + 51 * 51 * 41 * 4, // at least larger than one 3D volume
        "4D output should be larger than a single 3D volume, got {} bytes",
        meta.len()
    );
}

/// 4D with individual unwrapping: each echo unwrapped independently.
#[test]
fn romeo_4d_individual_unwrapping() {
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
            "-i",
            "-o",
            output.to_str().unwrap(),
        ])
        .status()
        .expect("failed to execute romeo");
    assert!(status.success(), "romeo -i (individual unwrapping) failed");
    assert!(output.exists());
}

/// 4D with template echo 2.
#[test]
fn romeo_4d_template_echo() {
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
            "--template",
            "2",
            "-o",
            output.to_str().unwrap(),
        ])
        .status()
        .expect("failed to execute romeo");
    assert!(status.success(), "romeo --template 2 failed");
    assert!(output.exists());
}

/// 4D with echo selection: only unwrap echoes 1 and 2.
#[test]
fn romeo_4d_unwrap_echoes_subset() {
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
            "-e",
            "[1, 2]",
            "-o",
            output.to_str().unwrap(),
        ])
        .status()
        .expect("failed to execute romeo");
    assert!(status.success(), "romeo -e '[1, 2]' failed");
    assert!(output.exists());
}

/// 4D with phase offset correction.
#[test]
fn romeo_4d_phase_offset_correction() {
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
            "--phase-offset-correction",
            "--write-phase-offsets",
            "-o",
            output.to_str().unwrap(),
        ])
        .status()
        .expect("failed to execute romeo");
    assert!(
        status.success(),
        "romeo --phase-offset-correction failed"
    );
    assert!(output.exists());
    // Phase offsets should be written
    let po_path = tmpdir.path().join("unwrapped_phase_offset.nii");
    assert!(
        po_path.exists(),
        "phase offset file was not created"
    );
}

/// 4D with B0 computation and phase-offset correction combined.
#[test]
fn romeo_4d_compute_b0_with_weights() {
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
            "-B",
            "--B0-phase-weighting",
            "average",
            "-o",
            output.to_str().unwrap(),
        ])
        .status()
        .expect("failed to execute romeo");
    assert!(status.success(), "romeo -B --B0-phase-weighting average failed");
    assert!(output.exists());
    let b0_path = tmpdir.path().join("B0.nii");
    assert!(b0_path.exists(), "B0 map was not created");
}

/// 4D with max-seeds > 1.
#[test]
fn romeo_4d_max_seeds() {
    let tmpdir = tempfile::tempdir().unwrap();
    let output = tmpdir.path().join("unwrapped.nii");
    let status = romeo_bin()
        .args([
            "-p",
            &phase_file(),
            "-t",
            "1:3",
            "-s",
            "3",
            "-i",
            "-o",
            output.to_str().unwrap(),
        ])
        .status()
        .expect("failed to execute romeo");
    assert!(status.success(), "romeo -s 3 (max seeds) failed");
    assert!(output.exists());
}

/// 4D with fix-ge-phase.
#[test]
fn romeo_4d_fix_ge_phase() {
    let tmpdir = tempfile::tempdir().unwrap();
    let output = tmpdir.path().join("unwrapped.nii");
    let status = romeo_bin()
        .args([
            "-p",
            &phase_file(),
            "-t",
            "1:3",
            "--fix-ge-phase",
            "-o",
            output.to_str().unwrap(),
        ])
        .status()
        .expect("failed to execute romeo");
    assert!(status.success(), "romeo --fix-ge-phase failed");
    assert!(output.exists());
}

/// 4D with quality maps.
#[test]
fn romeo_4d_write_quality_all() {
    let tmpdir = tempfile::tempdir().unwrap();
    let output = tmpdir.path().join("unwrapped.nii");
    let status = romeo_bin()
        .args([
            "-p",
            &phase_file(),
            "-t",
            "1:3",
            "-q",
            "-Q",
            "-o",
            output.to_str().unwrap(),
        ])
        .status()
        .expect("failed to execute romeo");
    assert!(status.success(), "romeo -q -Q failed");
    assert!(output.exists());
    assert!(tmpdir.path().join("unwrapped_quality.nii").exists());
    assert!(tmpdir.path().join("unwrapped_quality_x.nii").exists());
    assert!(tmpdir.path().join("unwrapped_quality_y.nii").exists());
    assert!(tmpdir.path().join("unwrapped_quality_z.nii").exists());
}

/// 4D combined: all major flags together.
#[test]
fn romeo_4d_combined_flags() {
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
            "--phase-offset-correction",
            "--write-phase-offsets",
            "-g",
            "-u",
            "-v",
            "-o",
            output.to_str().unwrap(),
        ])
        .status()
        .expect("failed to execute romeo");
    assert!(status.success(), "romeo combined 4D flags failed");
    assert!(output.exists());
}
