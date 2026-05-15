//! CLI contract tests using assert_cmd.
//!
//! Tests the CLI binary as a black box — exit codes, stdout/stderr output, argument parsing.

use std::fs;
use std::process::Command;

fn video_uploader() -> Command {
    Command::new(env!("CARGO_BIN_EXE_video-uploader"))
}

fn with_passphrase(mut cmd: Command) -> Command {
    cmd.arg("--passphrase").arg("longpassphrase999");
    cmd
}

fn temp_home() -> tempfile::TempDir {
    tempfile::tempdir().unwrap()
}

fn write_file(path: &std::path::Path, contents: &str) {
    fs::write(path, contents).unwrap();
}

fn make_pass_file(dir: &std::path::Path, pass: &str) -> std::path::PathBuf {
    let p = dir.join("pass.txt");
    fs::write(&p, pass).unwrap();
    p
}

#[test]
fn cli_auth_unknown_platform_rejected() {
    let home = temp_home();
    let pass = make_pass_file(home.path(), "longpassphrase999");
    let mut cmd = with_passphrase(video_uploader());
    cmd.env("HOME", home.path());
    cmd.args(["--passphrase-file", pass.to_str().unwrap(), "auth", "vimeo"]);
    let output = cmd.output().unwrap();
    assert!(
        !output.status.success(),
        "expected failure for unknown platform vimeo"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("invalid value") || stderr.contains("vimeo"),
        "unexpected stderr: {}",
        stderr
    );
}

#[test]
fn cli_upload_missing_file_no_passphrase() {
    let home = temp_home();
    let mut cmd = video_uploader();
    cmd.env("HOME", home.path());
    cmd.args(["upload", "--file", "/nonexistent.mp4", "--title", "Test"]);
    let output = cmd.output().unwrap();
    assert!(
        !output.status.success(),
        "expected failure without passphrase"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("passphrase") || stderr.contains("required"),
        "unexpected stderr: {}",
        stderr
    );
}

#[test]
fn cli_upload_missing_file_with_passphrase() {
    let home = temp_home();
    let mut cmd = with_passphrase(video_uploader());
    cmd.env("HOME", home.path());
    cmd.args([
        "upload",
        "--file",
        "/nonexistent.mp4",
        "--title",
        "Test",
        "--platforms",
        "youtube",
    ]);
    let output = cmd.output().unwrap();
    // File doesn't exist → validation or upload should fail with a clear message
    // Note: CLI exits 0 even on platform errors; check stderr for the failure message
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("No such file")
            || stderr.contains("not exist")
            || stderr.contains("not found")
            || stderr.contains("failed"),
        "expected file error in stderr, got: {}",
        stderr
    );
}

#[test]
fn cli_upload_unknown_platform_warns() {
    let home = temp_home();
    let mut cmd = with_passphrase(video_uploader());
    cmd.env("HOME", home.path());
    cmd.args([
        "upload",
        "--file",
        "/nonexistent.mp4",
        "--title",
        "Test",
        "--platforms",
        "yotube",
    ]);
    let output = cmd.output().unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Unknown platform") || stderr.contains("yotube"),
        "expected unknown platform warning, got: {}",
        stderr
    );
}

#[test]
fn cli_list_no_credentials() {
    let home = temp_home();
    let mut cmd = with_passphrase(video_uploader());
    cmd.env("HOME", home.path());
    cmd.args(["list"]);
    let output = cmd.output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("No platforms configured"),
        "expected no platforms message, got: {}",
        stdout
    );
}

#[test]
fn cli_list_with_passphrase_file() {
    let home = temp_home();
    let pass = make_pass_file(home.path(), "longpassphrase999");
    let mut cmd = video_uploader();
    cmd.env("HOME", home.path());
    cmd.args(["--passphrase-file", pass.to_str().unwrap(), "list"]);
    let output = cmd.output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("No platforms configured"),
        "expected no platforms message, got: {}",
        stdout
    );
}

#[test]
fn cli_batch_missing_manifest() {
    let home = temp_home();
    let mut cmd = with_passphrase(video_uploader());
    cmd.env("HOME", home.path());
    cmd.args(["batch", "--manifest", "/nonexistent.csv"]);
    let output = cmd.output().unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("No such file") || stderr.contains("not accessible"),
        "unexpected stderr: {}",
        stderr
    );
}

#[test]
fn cli_batch_malformed_csv() {
    let home = temp_home();
    let bad_csv = home.path().join("bad.csv");
    write_file(&bad_csv, "file,title\n/notvalid");

    let mut cmd = with_passphrase(video_uploader());
    cmd.env("HOME", home.path());
    cmd.args(["batch", "--manifest", bad_csv.to_str().unwrap()]);
    let output = cmd.output().unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("CSV") || stderr.contains("parse") || stderr.contains("missing"),
        "unexpected stderr: {}",
        stderr
    );
}

#[test]
fn cli_batch_dry_run_valid_manifest() {
    let home = temp_home();

    let video_path = home.path().join("video.mp4");
    let fixture =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/minimal.mp4");
    if fixture.exists() {
        fs::copy(&fixture, &video_path).unwrap();
    } else {
        fs::write(&video_path, b"fake video content").unwrap();
    }

    let csv_path = home.path().join("manifest.csv");
    write_file(
        &csv_path,
        &format!(
            "file,title,description,tags,visibility,platforms\n\
             {},Test Video,Test desc,tag1;tag2,public,youtube\n",
            video_path.to_str().unwrap()
        ),
    );

    let mut cmd = with_passphrase(video_uploader());
    cmd.env("HOME", home.path());
    cmd.args([
        "batch",
        "--manifest",
        csv_path.to_str().unwrap(),
        "--dry-run",
    ]);
    let output = cmd.output().unwrap();
    assert!(
        output.status.success(),
        "dry run should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Batch manifest loaded"),
        "expected manifest loaded, got: {}",
        stdout
    );
    assert!(
        stdout.contains("Dry run") || stdout.contains("1 video(s)"),
        "expected dry run confirmation, got: {}",
        stdout
    );
}

#[test]
fn cli_help_flag_auth() {
    let mut cmd = video_uploader();
    cmd.args(["auth", "--help"]);
    let output = cmd.output().unwrap();
    assert!(output.status.success());
}

#[test]
fn cli_help_flag_upload() {
    let mut cmd = video_uploader();
    cmd.args(["upload", "--help"]);
    let output = cmd.output().unwrap();
    assert!(output.status.success());
}

#[test]
fn cli_help_flag_batch() {
    let mut cmd = video_uploader();
    cmd.args(["batch", "--help"]);
    let output = cmd.output().unwrap();
    assert!(output.status.success());
}

#[test]
fn cli_help_flag_list() {
    let mut cmd = video_uploader();
    cmd.args(["list", "--help"]);
    let output = cmd.output().unwrap();
    assert!(output.status.success());
}

#[test]
fn cli_passphrase_too_short() {
    let home = temp_home();
    let mut cmd = video_uploader();
    cmd.env("HOME", home.path());
    cmd.arg("--passphrase").arg("short");
    cmd.args(["list"]);
    let output = cmd.output().unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("8 characters"),
        "expected 8 char error, got: {}",
        stderr
    );
}

#[test]
fn cli_passphrase_file_empty_rejected() {
    let home = temp_home();
    let empty_pass = home.path().join("empty.txt");
    fs::write(&empty_pass, "").unwrap();

    let mut cmd = video_uploader();
    cmd.env("HOME", home.path());
    cmd.arg("--passphrase-file")
        .arg(empty_pass.to_str().unwrap());
    cmd.args(["list"]);
    let output = cmd.output().unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("empty"),
        "expected empty passphrase error, got: {}",
        stderr
    );
}
