use crate::{UploadError, VideoUpload};
use tokio::fs::metadata;

/// Platform-specific size limits in bytes.
pub const YOUTUBE_MAX_SIZE: u64 = 128 * 1024 * 1024 * 1024; // 128 GiB
pub const ODYSEE_MAX_SIZE: u64 = 2 * 1024 * 1024 * 1024; // 2 GiB

/// Common video file extensions accepted by most platforms.
pub const VALID_EXTENSIONS: &[&str] = &[
    "mp4", "mov", "avi", "wmv", "flv", "webm", "mkv", "m4v", "mpeg", "mpg", "3gp", "ts",
];

/// Validate a video file for upload to a specific platform.
///
/// Checks:
/// - File exists and is readable
/// - File size is within platform limits
/// - File extension indicates a supported video format
pub async fn validate(video: &VideoUpload, platform: &str) -> Result<(), UploadError> {
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
    let size = video.file_size().await.map_err(|e| {
        UploadError::Io(std::io::Error::new(
            e.kind(),
            format!("Cannot read file metadata for {}: {}", path.display(), e),
        ))
    })?;

    let max_size = match platform {
        "youtube" => YOUTUBE_MAX_SIZE,
        "odysee" => ODYSEE_MAX_SIZE,
        "" => u64::MAX, // empty string means generic validation (no platform limit)
        _ => {
            return Err(UploadError::Config(format!(
                "Unknown platform: {}. Known platforms: youtube, odysee",
                platform
            )));
        }
    };

    if size > max_size {
        return Err(UploadError::FileTooLarge {
            size,
            max: max_size,
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

/// Validate without a specific platform (generic checks only).
pub async fn validate_generic(video: &VideoUpload) -> Result<(), UploadError> {
    validate(video, "").await
}

/// Get a human-readable description of size limits for a platform.
pub fn size_limit_description(platform: &str) -> &'static str {
    match platform {
        "youtube" => "128 GiB",
        "odysee" => "2 GiB",
        _ => "unknown platform",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn temp_file_with_ext(ext: &str, size: usize) -> (tempfile::NamedTempFile, String) {
        let mut file = tempfile::NamedTempFile::new().unwrap();
        let _path = file.path().with_extension(ext);
        let data = vec![0u8; size];
        file.write_all(&data).unwrap();
        // rename to desired extension
        let new_file = tempfile::NamedTempFile::new().unwrap();
        let new_path = new_file.path().with_extension(ext);
        std::fs::write(&new_path, &data).unwrap();
        (new_file, new_path.to_string_lossy().to_string())
    }

    #[tokio::test]
    async fn test_validate_nonexistent_file() {
        let video = VideoUpload::new("/tmp/does_not_exist_12345.mp4", "Title");
        let result = validate(&video, "youtube").await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), UploadError::Io(_)));
    }

    #[tokio::test]
    async fn test_validate_empty_title() {
        let (_file, path) = temp_file_with_ext("mp4", 1024);
        let video = VideoUpload::new(&path, "   ");
        let result = validate(&video, "youtube").await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), UploadError::Config(_)));
    }

    #[tokio::test]
    async fn test_validate_unsupported_extension() {
        let file = tempfile::NamedTempFile::new().unwrap();
        let path = file.path().with_extension("txt");
        std::fs::write(&path, b"not a video").unwrap();

        let video = VideoUpload::new(&path, "Title");
        let result = validate(&video, "youtube").await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            UploadError::UnsupportedFormat(_)
        ));
    }

    #[tokio::test]
    async fn test_validate_no_extension() {
        let file = tempfile::NamedTempFile::new().unwrap();
        let path = file.path().to_path_buf();
        std::fs::write(&path, b"no ext").unwrap();

        let video = VideoUpload::new(&path, "Title");
        let result = validate(&video, "youtube").await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            UploadError::UnsupportedFormat(_)
        ));
    }

    #[tokio::test]
    async fn test_validate_empty_file() {
        let file = tempfile::NamedTempFile::new().unwrap();
        let path = file.path().with_extension("mp4");
        std::fs::write(&path, b"").unwrap();

        let video = VideoUpload::new(&path, "Title");
        let result = validate(&video, "youtube").await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), UploadError::Io(_)));
    }

    #[tokio::test]
    async fn test_validate_valid_mp4() {
        let file = tempfile::NamedTempFile::new().unwrap();
        let path = file.path().with_extension("mp4");
        std::fs::write(&path, vec![0u8; 1024]).unwrap();

        let video = VideoUpload::new(&path, "Valid Title");
        let result = validate(&video, "youtube").await;
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
    fn test_size_limit_descriptions() {
        assert_eq!(size_limit_description("youtube"), "128 GiB");
        assert_eq!(size_limit_description("odysee"), "2 GiB");
        assert_eq!(size_limit_description("unknown"), "unknown platform");
    }

    #[tokio::test]
    async fn test_validate_generic_ok() {
        let file = tempfile::NamedTempFile::new().unwrap();
        let path = file.path().with_extension("mp4");
        std::fs::write(&path, vec![0u8; 1024]).unwrap();
        let video = VideoUpload::new(&path, "Valid Title");
        assert!(validate_generic(&video).await.is_ok());
    }

    #[tokio::test]
    async fn test_validate_unknown_platform_rejected() {
        let file = tempfile::NamedTempFile::new().unwrap();
        let path = file.path().with_extension("mp4");
        std::fs::write(&path, vec![0u8; 1024]).unwrap();
        let video = VideoUpload::new(&path, "Valid Title");
        let result = validate(&video, "unknown_platform").await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, UploadError::Config(_)));
        let err_str = format!("{}", err);
        assert!(err_str.contains("Unknown platform"));
    }
}
