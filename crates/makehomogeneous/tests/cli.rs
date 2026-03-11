//! Integration tests for the `makehomogeneous` CLI binary.
//!
//! Mirrors the test cases from the original Julia CompileMRI.jl test suite
//! (test/makehomogeneous_test.jl) plus additional tests for comprehensive
//! coverage of the Rust implementation.

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

// ===== Julia CompileMRI.jl parity tests =====

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

// ===== Output file validation tests =====

/// Verify output NIfTI file exists and has valid size.
#[test]
fn makehomogeneous_output_file_exists_and_nonempty() {
    let tmpdir = tempfile::tempdir().unwrap();
    let output = tmpdir.path().join("homogenous");
    let status = makehomogeneous_bin()
        .args(["-m", &mag_file(), "-o", output.to_str().unwrap()])
        .status()
        .expect("failed to execute makehomogeneous");
    assert!(status.success());
    let nii_path = tmpdir.path().join("homogenous.nii");
    assert!(nii_path.exists(), "homogenous.nii was not created");
    let meta = std::fs::metadata(&nii_path).unwrap();
    assert!(
        meta.len() > 352,
        "output NIfTI file is too small ({} bytes), expected at least a NIfTI header",
        meta.len()
    );
}

/// Verify output with .nii extension in output path works.
#[test]
fn makehomogeneous_output_with_nii_extension() {
    let tmpdir = tempfile::tempdir().unwrap();
    let output = tmpdir.path().join("corrected.nii");
    let status = makehomogeneous_bin()
        .args(["-m", &mag_file(), "-o", output.to_str().unwrap()])
        .status()
        .expect("failed to execute makehomogeneous");
    assert!(status.success());
    assert!(output.exists(), "corrected.nii was not created");
}

// ===== Verbose mode =====

/// Verbose output should include processing details.
#[test]
fn makehomogeneous_verbose_output() {
    let tmpdir = tempfile::tempdir().unwrap();
    let output = tmpdir.path().join("homogenous");
    let result = makehomogeneous_bin()
        .args(["-m", &mag_file(), "-v", "-o", output.to_str().unwrap()])
        .output()
        .expect("failed to execute makehomogeneous");
    assert!(result.status.success());
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        stderr.contains("makehomogeneous"),
        "verbose output should contain 'makehomogeneous'"
    );
    assert!(
        stderr.contains("magnitude:"),
        "verbose output should contain magnitude path"
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
        stderr.contains("sigma-bias-field:"),
        "verbose output should contain sigma setting"
    );
    assert!(
        stderr.contains("nbox:"),
        "verbose output should contain nbox setting"
    );
    assert!(
        stderr.contains("homogeneity correction complete"),
        "verbose output should confirm completion"
    );
    assert!(
        stderr.contains("saved to:"),
        "verbose output should confirm save"
    );
}

// ===== Default values =====

/// Default sigma (7.0) and nbox (15) should work.
#[test]
fn makehomogeneous_default_params() {
    let tmpdir = tempfile::tempdir().unwrap();
    let output = tmpdir.path().join("homogenous");
    let status = makehomogeneous_bin()
        .args(["-m", &mag_file(), "-o", output.to_str().unwrap()])
        .status()
        .expect("failed to execute makehomogeneous");
    assert!(status.success());
    assert!(tmpdir.path().join("homogenous.nii").exists());
}

// ===== Combined options =====

/// Test with both sigma and nbox customized together.
#[test]
fn makehomogeneous_combined_sigma_and_nbox() {
    let tmpdir = tempfile::tempdir().unwrap();
    let output = tmpdir.path().join("homogenous");
    let status = makehomogeneous_bin()
        .args([
            "-m",
            &mag_file(),
            "-s",
            "5.0",
            "-n",
            "10",
            "-o",
            output.to_str().unwrap(),
        ])
        .status()
        .expect("failed to execute makehomogeneous");
    assert!(status.success());
    assert!(tmpdir.path().join("homogenous.nii").exists());
}

/// Test with all options together.
#[test]
fn makehomogeneous_all_options() {
    let tmpdir = tempfile::tempdir().unwrap();
    let output = tmpdir.path().join("homogenous");
    let status = makehomogeneous_bin()
        .args([
            "-m",
            &mag_file(),
            "-s",
            "4.5",
            "-n",
            "8",
            "-d",
            "Float32",
            "-v",
            "-o",
            output.to_str().unwrap(),
        ])
        .status()
        .expect("failed to execute makehomogeneous");
    assert!(status.success());
    assert!(tmpdir.path().join("homogenous.nii").exists());
    assert!(tmpdir.path().join("settings_makehomogeneous.txt").exists());
}

// ===== Missing required args =====

/// Without -m (magnitude), the binary should fail.
#[test]
fn makehomogeneous_missing_magnitude_fails() {
    let tmpdir = tempfile::tempdir().unwrap();
    let output = tmpdir.path().join("homogenous");
    let status = makehomogeneous_bin()
        .args(["-o", output.to_str().unwrap()])
        .status()
        .expect("failed to execute makehomogeneous");
    assert!(
        !status.success(),
        "makehomogeneous should fail without --magnitude"
    );
}

// ===== Settings file content =====

/// Verify settings file contains command-line arguments.
#[test]
fn makehomogeneous_settings_file_content() {
    let tmpdir = tempfile::tempdir().unwrap();
    let output = tmpdir.path().join("homogenous");
    let status = makehomogeneous_bin()
        .args(["-m", &mag_file(), "-s", "3", "-o", output.to_str().unwrap()])
        .status()
        .expect("failed to execute makehomogeneous");
    assert!(status.success());
    let settings_path = tmpdir.path().join("settings_makehomogeneous.txt");
    assert!(settings_path.exists());
    let content = std::fs::read_to_string(&settings_path).unwrap();
    assert!(content.contains("Arguments:"));
    assert!(content.contains("-m"));
    assert!(content.contains("-s"));
    assert!(content.contains("3"));
}

// ===== Large sigma =====

/// Test with a very large sigma (low-pass filter approaches the image size).
#[test]
fn makehomogeneous_large_sigma() {
    let tmpdir = tempfile::tempdir().unwrap();
    let output = tmpdir.path().join("homogenous");
    let status = makehomogeneous_bin()
        .args([
            "-m",
            &mag_file(),
            "-s",
            "20",
            "-o",
            output.to_str().unwrap(),
        ])
        .status()
        .expect("failed to execute makehomogeneous");
    assert!(status.success());
    assert!(tmpdir.path().join("homogenous.nii").exists());
}

/// Test with a small sigma (higher-frequency correction).
#[test]
fn makehomogeneous_small_sigma() {
    let tmpdir = tempfile::tempdir().unwrap();
    let output = tmpdir.path().join("homogenous");
    let status = makehomogeneous_bin()
        .args(["-m", &mag_file(), "-s", "1", "-o", output.to_str().unwrap()])
        .status()
        .expect("failed to execute makehomogeneous");
    assert!(status.success());
    assert!(tmpdir.path().join("homogenous.nii").exists());
}
