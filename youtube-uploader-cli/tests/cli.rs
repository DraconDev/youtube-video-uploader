//! CLI contract tests using assert_cmd.
//!
//! Tests the CLI binary as a black box — exit codes, stdout/stderr output, argument parsing.

use std::fs;
use std::process::Command;

fn youtube_uploader() -> Command {
    Command::new(env!("CARGO_BIN_EXE_youtube-uploader"))
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
fn cli_auth_rejects_unexpected_arg() {
    let home = temp_home();
    let pass = make_pass_file(home.path(), "longpassphrase999");
    let mut cmd = with_passphrase(youtube_uploader());
    cmd.env("HOME", home.path());
    cmd.args([
        "--passphrase-file",
        pass.to_str().unwrap(),
        "auth",
        "unexpected",
    ]);
    let output = cmd.output().unwrap();
    assert!(
        !output.status.success(),
        "expected failure for unexpected arg"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("unexpected") || !stderr.is_empty(),
        "unexpected stderr: {}",
        stderr
    );
}

#[test]
fn cli_upload_missing_file_no_passphrase() {
    let home = temp_home();
    let mut cmd = youtube_uploader();
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
    let mut cmd = with_passphrase(youtube_uploader());
    cmd.env("HOME", home.path());
    cmd.args(["upload", "--file", "/nonexistent.mp4", "--title", "Test"]);
    let output = cmd.output().unwrap();
    // File doesn't exist → validation or upload should fail with a clear message
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("No such file")
            || stderr.contains("not exist")
            || stderr.contains("not found")
            || stderr.contains("failed")
            || stderr.contains("error")
            || stderr.contains("workspace"),
        "expected file or workspace error in stderr, got: {}",
        stderr
    );
}

#[test]
fn cli_upload_rejects_unknown_flag() {
    let home = temp_home();
    let mut cmd = with_passphrase(youtube_uploader());
    cmd.env("HOME", home.path());
    cmd.args([
        "upload",
        "--file",
        "/nonexistent.mp4",
        "--title",
        "Test",
        "--unknown-flag",
    ]);
    let output = cmd.output().unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("unexpected") || stderr.contains("unknown"),
        "expected unknown flag error, got: {}",
        stderr
    );
}

#[test]
fn cli_list_no_credentials() {
    let home = temp_home();
    let mut cmd = with_passphrase(youtube_uploader());
    cmd.env("HOME", home.path());
    cmd.args(["list"]);
    let output = cmd.output().unwrap();
    assert!(output.status.success());
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        combined.contains("No workspaces configured"),
        "expected not configured message, got: {}",
        combined
    );
}

#[test]
fn cli_list_with_passphrase_file() {
    let home = temp_home();
    let pass = make_pass_file(home.path(), "longpassphrase999");
    let mut cmd = youtube_uploader();
    cmd.env("HOME", home.path());
    cmd.args(["--passphrase-file", pass.to_str().unwrap(), "list"]);
    let output = cmd.output().unwrap();
    assert!(output.status.success());
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        combined.contains("No workspaces configured"),
        "expected not configured message, got: {}",
        combined
    );
}

#[test]
fn cli_batch_missing_manifest() {
    let home = temp_home();
    let mut cmd = with_passphrase(youtube_uploader());
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

    let mut cmd = with_passphrase(youtube_uploader());
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
            "file,title,description,tags,visibility,workspace\n\
             {},Test Video,Test desc,tag1;tag2,public,youtube\n",
            video_path.to_str().unwrap()
        ),
    );

    let mut cmd = with_passphrase(youtube_uploader());
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
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        combined.contains("Batch manifest loaded"),
        "expected manifest loaded, got: {}",
        combined
    );
    assert!(
        combined.contains("Dry Run") || combined.contains("1 video(s)"),
        "expected dry run confirmation, got: {}",
        combined
    );
}

#[test]
fn cli_help_flag_auth() {
    let mut cmd = youtube_uploader();
    cmd.args(["auth", "--help"]);
    let output = cmd.output().unwrap();
    assert!(output.status.success());
}

#[test]
fn cli_help_flag_upload() {
    let mut cmd = youtube_uploader();
    cmd.args(["upload", "--help"]);
    let output = cmd.output().unwrap();
    assert!(output.status.success());
}

#[test]
fn cli_help_flag_batch() {
    let mut cmd = youtube_uploader();
    cmd.args(["batch", "--help"]);
    let output = cmd.output().unwrap();
    assert!(output.status.success());
}

#[test]
fn cli_help_flag_list() {
    let mut cmd = youtube_uploader();
    cmd.args(["list", "--help"]);
    let output = cmd.output().unwrap();
    assert!(output.status.success());
}

#[test]
fn cli_passphrase_too_short() {
    let home = temp_home();
    let mut cmd = youtube_uploader();
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

    let mut cmd = youtube_uploader();
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

// ---------------------------------------------------------------------------
// Workspace subcommand tests
// ---------------------------------------------------------------------------

#[test]
fn cli_workspace_help() {
    let mut cmd = youtube_uploader();
    cmd.args(["workspace", "--help"]);
    let output = cmd.output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("default") && stdout.contains("rename") && stdout.contains("remove"),
        "expected workspace subcommands in help, got: {}",
        stdout
    );
}

#[test]
fn cli_workspace_default_nonexistent() {
    let home = temp_home();
    let mut cmd = with_passphrase(youtube_uploader());
    cmd.env("HOME", home.path());
    cmd.args(["workspace", "default", "nonexistent"]);
    let output = cmd.output().unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("does not exist"),
        "expected 'does not exist' error, got: {}",
        stderr
    );
}

#[test]
fn cli_workspace_remove_nonexistent() {
    let home = temp_home();
    let mut cmd = with_passphrase(youtube_uploader());
    cmd.env("HOME", home.path());
    cmd.args(["workspace", "remove", "nonexistent"]);
    let output = cmd.output().unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("does not exist"),
        "expected 'does not exist' error, got: {}",
        stderr
    );
}

#[test]
fn cli_batch_dry_run_multi_row_with_workspaces() {
    let home = temp_home();

    // Create video files
    let fixture =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/minimal.mp4");
    let video1 = home.path().join("video1.mp4");
    let video2 = home.path().join("video2.mp4");
    let video3 = home.path().join("video3.mp4");
    if fixture.exists() {
        fs::copy(&fixture, &video1).unwrap();
        fs::copy(&fixture, &video2).unwrap();
        fs::copy(&fixture, &video3).unwrap();
    } else {
        fs::write(&video1, b"fake video").unwrap();
        fs::write(&video2, b"fake video").unwrap();
        fs::write(&video3, b"fake video").unwrap();
    }

    let csv_path = home.path().join("manifest.csv");
    write_file(
        &csv_path,
        &format!(
            "file,title,workspace,visibility\n\
            {},Gaming Video,gaming,public\n\
            {},Cooking Video,cooking,unlisted\n\
            {},Default Video,,public\n",
            video1.to_str().unwrap(),
            video2.to_str().unwrap(),
            video3.to_str().unwrap(),
        ),
    );

    let mut cmd = with_passphrase(youtube_uploader());
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
        "multi-row dry run should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        combined.contains("3 video(s)"),
        "expected 3 videos, got: {}",
        combined
    );
    assert!(
        combined.contains("[gaming]"),
        "expected gaming workspace marker, got: {}",
        combined
    );
    assert!(
        combined.contains("[cooking]"),
        "expected cooking workspace marker, got: {}",
        combined
    );
    assert!(
        combined.contains("(default)"),
        "expected default workspace marker, got: {}",
        combined
    );
}

#[test]
fn cli_workspace_rename_nonexistent() {
    let home = temp_home();
    let mut cmd = with_passphrase(youtube_uploader());
    cmd.env("HOME", home.path());
    cmd.args(["workspace", "rename", "old", "new"]);
    let output = cmd.output().unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("does not exist"),
        "expected 'does not exist' error, got: {}",
        stderr
    );
}

#[test]
fn cli_channel_help() {
    let mut cmd = youtube_uploader();
    cmd.args(["channel", "--help"]);
    let output = cmd.output().unwrap();
    assert!(output.status.success());
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        combined.contains("Workspace"),
        "expected Workspace in channel help, got: {}",
        combined
    );
}
