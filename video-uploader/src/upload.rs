use std::fmt;
use std::path::{Path, PathBuf};

/// Privacy level for uploaded videos.
///
/// Controls who can see the video on YouTube after upload.
///
/// # Examples
///
/// ```
/// use video_uploader::Visibility;
///
/// assert_eq!(Visibility::default(), Visibility::Private);
/// assert_eq!(Visibility::Unlisted.to_string(), "unlisted");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum Visibility {
    #[default]
    Private,
    Unlisted,
    Public,
}

impl fmt::Display for Visibility {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Visibility::Private => write!(f, "private"),
            Visibility::Unlisted => write!(f, "unlisted"),
            Visibility::Public => write!(f, "public"),
        }
    }
}

/// Metadata and file path for a video upload.
///
/// Use the builder pattern to construct:
///
/// ```
/// use video_uploader::{VideoUpload, Visibility};
///
/// let video = VideoUpload::new("/path/to/video.mp4", "My Video")
///     .with_description("A great video")
///     .with_tags(vec!["rust".into(), "programming".into()])
///     .with_visibility(Visibility::Private);
/// ```
#[derive(Debug, Clone)]
pub struct VideoUpload {
    pub(crate) file_path: PathBuf,
    pub(crate) title: String,
    pub(crate) description: Option<String>,
    pub(crate) tags: Vec<String>,
    pub(crate) visibility: Visibility,
    pub(crate) category_id: Option<String>,
    pub(crate) made_for_kids: Option<bool>,
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
            made_for_kids: None,
        }
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    pub fn with_visibility(mut self, v: Visibility) -> Self {
        self.visibility = v;
        self
    }

    pub fn with_category(mut self, id: impl Into<String>) -> Self {
        self.category_id = Some(id.into());
        self
    }

    pub fn with_made_for_kids(mut self, flag: bool) -> Self {
        self.made_for_kids = Some(flag);
        self
    }

    /// Returns the file size in bytes by calling stat.
    pub async fn file_size(&self) -> std::io::Result<u64> {
        tokio::fs::metadata(&self.file_path).await.map(|m| m.len())
    }

    // -- Getters --

    /// Returns the video file path.
    pub fn file_path(&self) -> &Path {
        &self.file_path
    }

    /// Returns the video title.
    pub fn title(&self) -> &str {
        &self.title
    }

    /// Returns the video description, if set.
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    /// Returns the video tags.
    pub fn tags(&self) -> &[String] {
        &self.tags
    }

    /// Returns the video visibility setting.
    pub fn visibility(&self) -> Visibility {
        self.visibility
    }

    /// Returns the YouTube category ID, if set.
    pub fn category_id(&self) -> Option<&str> {
        self.category_id.as_deref()
    }

    /// Returns whether the video is made for kids.
    pub fn made_for_kids(&self) -> Option<bool> {
        self.made_for_kids
    }
}

/// Result returned after a successful upload.
///
/// Contains the workspace name, YouTube video ID, watch URL, and title.
///
/// # Examples
///
/// ```
/// use video_uploader::UploadResult;
///
/// let result = UploadResult::new(
///     "youtube",
///     "dQw4w9WgXcQ",
///     "https://www.youtube.com/watch?v=dQw4w9WgXcQ",
///     "Never Gonna Give You Up",
/// );
/// assert_eq!(result.workspace, "youtube");
/// assert_eq!(result.video_id, "dQw4w9WgXcQ");
/// ```
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct UploadResult {
    pub workspace: String,
    pub video_id: String,
    pub url: String,
    pub title: String,
}

impl UploadResult {
    /// Create a new upload result.
    pub fn new(workspace: impl Into<String>, video_id: impl Into<String>, url: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            workspace: workspace.into(),
            video_id: video_id.into(),
            url: url.into(),
            title: title.into(),
        }
    }
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
    fn test_visibility_default_is_private() {
        assert_eq!(Visibility::default(), Visibility::Private);
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
            .with_description("A test description")
            .with_tags(vec!["rust".to_string(), "test".to_string()])
            .with_visibility(Visibility::Private)
            .with_category("22");

        assert_eq!(upload.file_path(), Path::new("/tmp/video.mp4"));
        assert_eq!(upload.title(), "Test Title");
        assert_eq!(upload.description(), Some("A test description"));
        assert_eq!(upload.tags(), &["rust".to_string(), "test".to_string()]);
        assert_eq!(upload.visibility(), Visibility::Private);
        assert_eq!(upload.category_id(), Some("22"));
    }

    #[test]
    fn test_video_upload_minimal() {
        let upload = VideoUpload::new("/tmp/video.mp4", "Title Only");
        assert_eq!(upload.title(), "Title Only");
        assert!(upload.description().is_none());
        assert!(upload.tags().is_empty());
        assert_eq!(upload.visibility(), Visibility::Private);
        assert!(upload.category_id().is_none());
    }

    #[test]
    fn test_video_upload_builder_returns_self() {
        let upload = VideoUpload::new("/tmp/video.mp4", "Title");
        let upload2 = VideoUpload::new("/tmp/video.mp4", "Title").with_visibility(Visibility::Unlisted);
        assert_eq!(upload.visibility(), Visibility::Private);
        assert_eq!(upload2.visibility(), Visibility::Unlisted);
    }

    #[tokio::test]
    async fn test_video_upload_file_size_async() {
        let video = VideoUpload::new("/tmp/video.mp4", "Title");
        let result = video.file_size().await;
        assert!(result.is_err()); // /tmp/video.mp4 doesn't exist in test env
    }
}
