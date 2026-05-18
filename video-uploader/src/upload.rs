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

impl std::str::FromStr for Visibility {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "private" => Ok(Visibility::Private),
            "unlisted" => Ok(Visibility::Unlisted),
            "public" => Ok(Visibility::Public),
            _ => Err(format!("Unknown visibility: {s}")),
        }
    }
}

/// Video license type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum License {
    #[default]
    Youtube,
    CreativeCommon,
}

impl fmt::Display for License {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            License::Youtube => write!(f, "youtube"),
            License::CreativeCommon => write!(f, "creativeCommon"),
        }
    }
}

impl std::str::FromStr for License {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "youtube" | "standard" => Ok(License::Youtube),
            "creative-common" | "creativecommon" | "cc" => Ok(License::CreativeCommon),
            _ => Err(format!("Unknown license: {s}")),
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
    pub(crate) license: Option<License>,
    pub(crate) language: Option<String>,
    pub(crate) contains_synthetic_media: Option<bool>,
    pub(crate) embeddable: Option<bool>,
    pub(crate) public_stats_viewable: Option<bool>,
    pub(crate) description_suffix: Option<String>,
    pub(crate) publish_at: Option<String>,
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
            license: None,
            language: None,
            contains_synthetic_media: None,
            embeddable: None,
            public_stats_viewable: None,
            description_suffix: None,
            publish_at: None,
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

    pub fn with_license(mut self, license: License) -> Self {
        self.license = Some(license);
        self
    }

    pub fn with_language(mut self, lang: impl Into<String>) -> Self {
        self.language = Some(lang.into());
        self
    }

    pub fn with_contains_synthetic_media(mut self, flag: bool) -> Self {
        self.contains_synthetic_media = Some(flag);
        self
    }

    pub fn with_embeddable(mut self, flag: bool) -> Self {
        self.embeddable = Some(flag);
        self
    }

    pub fn with_public_stats_viewable(mut self, flag: bool) -> Self {
        self.public_stats_viewable = Some(flag);
        self
    }

    pub fn with_description_suffix(mut self, suffix: impl Into<String>) -> Self {
        self.description_suffix = Some(suffix.into());
        self
    }

    pub fn with_publish_at(mut self, datetime: impl Into<String>) -> Self {
        self.publish_at = Some(datetime.into());
        self
    }

    /// Apply profile defaults to any unset fields.
    /// Does not overwrite fields that are already set (CLI flags take precedence).
    pub fn apply_profile(mut self, profile: &crate::UploadProfile) -> Self {
        if self.visibility == Visibility::default()
            && let Some(ref vis) = profile.visibility
            && let Ok(v) = vis.parse()
        {
            self.visibility = v;
        }
        if self.category_id.is_none() && profile.category.is_some() {
            self.category_id = profile.category.clone();
        }
        if self.made_for_kids.is_none() && profile.made_for_kids.is_some() {
            self.made_for_kids = profile.made_for_kids;
        }
        if self.license.is_none() && profile.license.is_some() {
            self.license = profile.license.as_ref().and_then(|l| l.parse().ok());
        }
        if self.language.is_none() && profile.language.is_some() {
            self.language = profile.language.clone();
        }
        if self.contains_synthetic_media.is_none() && profile.contains_synthetic_media.is_some() {
            self.contains_synthetic_media = profile.contains_synthetic_media;
        }
        if self.embeddable.is_none() && profile.embeddable.is_some() {
            self.embeddable = profile.embeddable;
        }
        if self.public_stats_viewable.is_none() && profile.public_stats_viewable.is_some() {
            self.public_stats_viewable = profile.public_stats_viewable;
        }
        if self.description_suffix.is_none() && profile.description_suffix.is_some() {
            self.description_suffix = profile.description_suffix.clone();
        }
        if self.publish_at.is_none() && profile.publish_at.is_some() {
            self.publish_at = profile.publish_at.clone();
        }
        // Merge profile tags into video tags
        if let Some(ref profile_tags) = profile.tags {
            let mut merged = profile_tags.clone();
            merged.append(&mut self.tags);
            merged.sort();
            merged.dedup();
            self.tags = merged;
        }
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
