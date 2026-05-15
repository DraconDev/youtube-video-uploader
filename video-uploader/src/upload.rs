use std::fmt;
use std::path::PathBuf;

pub use crate::validation::VALID_EXTENSIONS as SUPPORTED_EXTENSIONS;

/// Privacy level for uploaded videos.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Visibility {
    #[default]
    Public,
    Unlisted,
    Private,
}

impl fmt::Display for Visibility {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Visibility::Public => write!(f, "public"),
            Visibility::Unlisted => write!(f, "unlisted"),
            Visibility::Private => write!(f, "private"),
        }
    }
}

/// Metadata and file path for a video upload.
#[derive(Debug, Clone)]
pub struct VideoUpload {
    pub file_path: PathBuf,
    pub title: String,
    pub description: Option<String>,
    pub tags: Vec<String>,
    pub visibility: Visibility,
    pub category_id: Option<String>,
}

impl VideoUpload {
    pub fn new(file_path: impl Into<PathBuf>, title: impl Into<String>) -> Self {
        Self {
            file_path: file_path.into(),
            title: title.into(),
            description: None,
            tags: Vec::new(),
            visibility: Visibility::default(),
            category_id: None,
        }
    }

    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    pub fn tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    pub fn visibility(mut self, v: Visibility) -> Self {
        self.visibility = v;
        self
    }

    pub fn category(mut self, id: impl Into<String>) -> Self {
        self.category_id = Some(id.into());
        self
    }

    /// Returns the file size in bytes by calling stat.
    pub async fn file_size(&self) -> std::io::Result<u64> {
        tokio::fs::metadata(&self.file_path).await.map(|m| m.len())
    }
}

/// Result returned after a successful upload.
#[derive(Debug, Clone)]
pub struct UploadResult {
    pub platform: &'static str,
    pub platform_id: String,
    pub url: String,
    pub title: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_visibility_display() {
        assert_eq!(Visibility::Public.to_string(), "public");
        assert_eq!(Visibility::Unlisted.to_string(), "unlisted");
        assert_eq!(Visibility::Private.to_string(), "private");
    }

    #[test]
    fn test_visibility_default_is_public() {
        assert_eq!(Visibility::default(), Visibility::Public);
    }

    #[test]
    fn test_visibility_serde_public_roundtrip() {
        let json = serde_json::to_string(&Visibility::Public).unwrap();
        let parsed: Visibility = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, Visibility::Public);
    }

    #[test]
    fn test_visibility_serde_private_roundtrip() {
        let json = serde_json::to_string(&Visibility::Private).unwrap();
        let parsed: Visibility = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, Visibility::Private);
    }

    #[test]
    fn test_visibility_serde_unlisted_roundtrip() {
        let json = serde_json::to_string(&Visibility::Unlisted).unwrap();
        let parsed: Visibility = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, Visibility::Unlisted);
    }

    #[test]
    fn test_video_upload_builder_pattern() {
        let upload = VideoUpload::new("/tmp/video.mp4", "Test Title")
            .description("A test description")
            .tags(vec!["rust".to_string(), "test".to_string()])
            .visibility(Visibility::Private)
            .category("22");

        assert_eq!(upload.file_path, PathBuf::from("/tmp/video.mp4"));
        assert_eq!(upload.title, "Test Title");
        assert_eq!(upload.description, Some("A test description".to_string()));
        assert_eq!(upload.tags, vec!["rust", "test"]);
        assert_eq!(upload.visibility, Visibility::Private);
        assert_eq!(upload.category_id, Some("22".to_string()));
    }

    #[test]
    fn test_video_upload_minimal() {
        let upload = VideoUpload::new("/tmp/video.mp4", "Title Only");
        assert_eq!(upload.title, "Title Only");
        assert!(upload.description.is_none());
        assert!(upload.tags.is_empty());
        assert_eq!(upload.visibility, Visibility::Public);
        assert!(upload.category_id.is_none());
    }

    #[test]
    fn test_video_upload_builder_returns_self() {
        let upload = VideoUpload::new("/tmp/video.mp4", "Title");
        let upload2 = VideoUpload::new("/tmp/video.mp4", "Title").visibility(Visibility::Unlisted);
        assert_eq!(upload.visibility, Visibility::Public);
        assert_eq!(upload2.visibility, Visibility::Unlisted);
    }

    #[tokio::test]
    async fn test_video_upload_file_size_async() {
        let video = VideoUpload::new("/tmp/video.mp4", "Title");
        let result = video.file_size().await;
        assert!(result.is_err()); // /tmp/video.mp4 doesn't exist in test env
    }
}
