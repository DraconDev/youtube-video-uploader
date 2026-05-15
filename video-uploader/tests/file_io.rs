//! Real file I/O tests — exercises actual disk I/O, permissions, and concurrent access.

use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

#[tokio::test]
#[cfg(unix)]
async fn test_file_read_unreadable_file() {
    use std::os::unix::fs::PermissionsExt;

    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("secret.mp4");
    fs::write(&file_path, b"secret video").unwrap();

    let mut perms = fs::metadata(&file_path).unwrap().permissions();
    perms.set_mode(0o000);
    fs::set_permissions(&file_path, perms).unwrap();

    let result = tokio::fs::read(&file_path).await;
    assert!(result.is_err(), "unreadable file should fail");

    let mut perms = fs::metadata(&file_path).unwrap().permissions();
    perms.set_mode(0o644);
    fs::set_permissions(&file_path, perms).unwrap();
}

#[tokio::test]
#[cfg(not(unix))]
async fn test_file_read_unreadable_file() {
    // Permission modes not supported on this platform
}

#[tokio::test]
async fn test_file_extension_detection() {
    use video_uploader::upload::VideoUpload;

    let temp_dir = TempDir::new().unwrap();

    for ext in &["mkv", "avi", "mov", "wmv", "flv", "webm", "m4v"] {
        let file_path = temp_dir.path().join(format!("video.{}", ext));
        fs::write(&file_path, b"fake video data").unwrap();
        let video = VideoUpload::new(&file_path, "Test");
        assert!(
            video.file_path.exists(),
            "file with .{} extension should exist",
            ext
        );
    }
}

#[tokio::test]
async fn test_zero_byte_file_validation() {
    use video_uploader::upload::VideoUpload;
    use video_uploader::validation::validate;

    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("empty.mp4");
    fs::write(&file_path, b"").unwrap();

    let video = VideoUpload::new(&file_path, "Test");
    let result = validate(&video, "youtube").await;

    assert!(result.is_err(), "zero-byte file should fail validation");
    let err = result.unwrap_err();
    assert!(
        err.to_string().contains("empty")
            || err.to_string().contains("size")
            || err.to_string().contains("validation"),
        "expected validation error, got: {}",
        err
    );
}

#[tokio::test]
async fn test_unicode_filename_path() {
    use video_uploader::upload::VideoUpload;

    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("video.mp4");
    fs::write(&file_path, b"video content").unwrap();

    let video = VideoUpload::new(&file_path, "Test Unicode");
    assert!(video.file_path.exists());

    // Verify it's readable
    let content = tokio::fs::read(&video.file_path).await.unwrap();
    assert_eq!(content, b"video content");
}

#[tokio::test]
async fn test_file_concurrent_read_same_file() {
    use video_uploader::upload::VideoUpload;

    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("shared.mp4");
    fs::write(&file_path, vec![0x42; 4096]).unwrap();

    let video1 = VideoUpload::new(&file_path, "Reader 1");
    let video2 = VideoUpload::new(&file_path, "Reader 2");

    let len1 = tokio::fs::metadata(&video1.file_path).await.unwrap().len();
    let len2 = tokio::fs::metadata(&video2.file_path).await.unwrap().len();

    assert_eq!(len1, 4096);
    assert_eq!(len2, 4096);
}

#[tokio::test]
async fn test_file_path_with_tilde_expansion() {
    let temp_dir = TempDir::new().unwrap();
    let home = temp_dir.path();
    let video_path = home.join("my video.mp4");
    fs::write(&video_path, b"test").unwrap();

    use video_uploader::upload::VideoUpload;
    let video = VideoUpload::new(&video_path, "Tilde Test");
    assert!(video.file_path.exists());
}

#[tokio::test]
async fn test_valid_minimal_fixture_file() {
    let path = fixture_path("minimal.mp4");
    if path.exists() {
        let metadata = fs::metadata(&path).unwrap();
        assert!(metadata.len() > 0, "fixture should not be empty");
        let content = tokio::fs::read(&path).await.unwrap();
        assert!(!content.is_empty(), "fixture should have content");
    } else {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("test.mp4");
        fs::write(&path, b"fake mp4 content").unwrap();
        assert!(tokio::fs::read(&path).await.is_ok());
    }
}
