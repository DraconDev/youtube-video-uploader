use crate::{UploadError, VideoUpload};
use tokio::fs::metadata;

/// YouTube size limit in bytes (128 GiB).
pub const YOUTUBE_MAX_SIZE: u64 = 128 * 1024 * 1024 * 1024;

/// Common video file extensions accepted by YouTube.
pub const VALID_EXTENSIONS: &[&str] = &[
    "mp4", "mov", "avi", "wmv", "flv", "webm", "mkv", "m4v", "mpeg", "mpg", "3gp", "ts",
];

/// Validate a video for upload (file existence, extension, size, title).
///
/// Checks:
/// - File exists and is readable
/// - File size is within YouTube's 128 GiB limit
/// - File extension indicates a supported video format
/// - Title is non-empty
///
/// Returns `Ok(())` if the video passes all checks, or an `UploadError` describing the issue.
pub async fn validate(video: &VideoUpload) -> Result<(), UploadError> {
    let path = &video.file_path;

    // File existence (async)
    let meta = metadata(path).await.map_err(|e| {
        UploadError::Io(std::io::Error::new(
            e.kind(),
            format!("File not found or not readable: {}", path.display()),
        ))
    })?;

    if !meta.is_file() {
        return Err(UploadError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("Not a file: {}", path.display()),
        )));
    }

    // File size
    let size = meta.len();
    if size > YOUTUBE_MAX_SIZE {
        return Err(UploadError::FileTooLarge {
            size,
            max: YOUTUBE_MAX_SIZE,
        });
    }

    if size == 0 {
        return Err(UploadError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "File is empty",
        )));
    }

    // Format validation (extension-based)
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        let ext_lower = ext.to_lowercase();
        if !VALID_EXTENSIONS.contains(&ext_lower.as_str()) {
            return Err(UploadError::UnsupportedFormat(format!(
                ".{} is not a supported video format. Supported: {}",
                ext,
                VALID_EXTENSIONS.join(", ")
            )));
        }
    } else {
        return Err(UploadError::UnsupportedFormat(
            "File has no extension. Cannot determine video format.".into(),
        ));
    }

    // Title validation
    if video.title.trim().is_empty() {
        return Err(UploadError::Config("Video title cannot be empty".into()));
    }

    Ok(())
}

/// Returns a human-readable description of the YouTube file size limit ("128 GiB").
pub fn size_limit_description() -> &'static str {
    "128 GiB"
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_file_with_ext(ext: &str, size: usize) -> (tempfile::TempDir, String) {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let path = temp_dir.path().join(format!("video.{}", ext));
        let data = vec![0u8; size];
        std::fs::write(&path, &data).unwrap();
        (temp_dir, path.to_string_lossy().to_string())
    }

    #[tokio::test]
    async fn test_validate_nonexistent_file() {
        let video = VideoUpload::new("/tmp/does_not_exist_12345.mp4", "Title");
        let result = validate(&video).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), UploadError::Io(_)));
    }

    #[tokio::test]
    async fn test_validate_empty_title() {
        let (_file, path) = temp_file_with_ext("mp4", 1024);
        let video = VideoUpload::new(&path, "   ");
        let result = validate(&video).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), UploadError::Config(_)));
    }

    #[tokio::test]
    async fn test_validate_unsupported_extension() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let path = temp_dir.path().join("video.txt");
        std::fs::write(&path, b"not a video").unwrap();

        let video = VideoUpload::new(&path, "Title");
        let result = validate(&video).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            UploadError::UnsupportedFormat(_)
        ));
    }

    #[tokio::test]
    async fn test_validate_no_extension() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let path = temp_dir.path().join("video_no_ext");
        std::fs::write(&path, b"no ext").unwrap();

        let video = VideoUpload::new(&path, "Title");
        let result = validate(&video).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            UploadError::UnsupportedFormat(_)
        ));
    }

    #[tokio::test]
    async fn test_validate_empty_file() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let path = temp_dir.path().join("video.mp4");
        std::fs::write(&path, b"").unwrap();

        let video = VideoUpload::new(&path, "Title");
        let result = validate(&video).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), UploadError::Io(_)));
    }

    #[tokio::test]
    async fn test_validate_valid_mp4() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let path = temp_dir.path().join("video.mp4");
        std::fs::write(&path, vec![0u8; 1024]).unwrap();

        let video = VideoUpload::new(&path, "Valid Title");
        let result = validate(&video).await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_youtube_size_limit() {
        // We can't easily create a 128 GiB file, so test the error variant directly
        let err = UploadError::FileTooLarge {
            size: 129 * 1024 * 1024 * 1024,
            max: YOUTUBE_MAX_SIZE,
        };
        assert!(!err.is_retryable());
    }

    #[test]
    fn test_size_limit_description() {
        assert_eq!(size_limit_description(), "128 GiB");
    }
}
