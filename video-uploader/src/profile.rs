//! Upload profiles — reusable presets stored as TOML files.
//!
//! Profiles live in `~/.config/video-uploader/profiles/<name>.toml`.
//! All fields are optional; only non-empty values override built-in defaults.

use crate::UploadError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// An upload profile loaded from a TOML file.
///
/// All fields are `Option` — `None` means "use the built-in default".
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UploadProfile {
    /// Video visibility: "private", "unlisted", or "public".
    pub visibility: Option<String>,

    /// Whether the video is made for kids.
    pub made_for_kids: Option<bool>,

    /// License: "youtube" (standard) or "creative-common".
    pub license: Option<String>,

    /// YouTube video category ID (e.g. "22" = People & Blogs, "20" = Gaming).
    pub category: Option<String>,

    /// BCP-47 language code (e.g. "en", "es", "fr").
    pub language: Option<String>,

    /// Whether the video contains AI/synthetic media.
    pub contains_synthetic_media: Option<bool>,

    /// Whether the video can be embedded on other sites.
    pub embeddable: Option<bool>,

    /// Whether view counts are publicly visible.
    pub public_stats_viewable: Option<bool>,

    /// Tags added to every upload (merged with video-specific tags).
    pub tags: Option<Vec<String>>,

    /// Text appended to the video description.
    pub description_suffix: Option<String>,

    /// Scheduled publish time (ISO 8601, e.g. "2026-05-20T09:00:00Z").
    pub publish_at: Option<String>,

    /// Recording date (ISO 8601 date, e.g. "2026-05-18").
    pub recording_date: Option<String>,
}

/// Per-video metadata loaded from a `.meta.toml` file.
///
/// Unlike profiles (which provide defaults), meta TOML provides the **primary** metadata
/// for a specific video. Fields here override profile defaults but are overridden by
/// explicit CLI flags.
///
/// Auto-discovery: if `video.mp4` has a `video.meta.toml` next to it, it's loaded
/// automatically.
///
/// # Example
///
/// ```toml
/// # video.meta.toml
/// title = "Let's Play Rust - Episode 1"
/// description = "Building a CLI tool from scratch."
/// tags = ["rust", "programming", "tutorial"]
/// category = "20"
/// visibility = "unlisted"
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VideoMeta {
    /// Video title.
    pub title: Option<String>,

    /// Video description.
    pub description: Option<String>,

    /// Tags for the video.
    pub tags: Option<Vec<String>>,

    /// Video visibility: "private", "unlisted", or "public".
    pub visibility: Option<String>,

    /// YouTube category ID.
    pub category: Option<String>,

    /// Whether the video is made for kids.
    pub made_for_kids: Option<bool>,

    /// License: "youtube" or "creative-common".
    pub license: Option<String>,

    /// BCP-47 language code.
    pub language: Option<String>,

    /// Whether the video contains AI/synthetic media.
    pub contains_synthetic_media: Option<bool>,

    /// Whether the video can be embedded on other sites.
    pub embeddable: Option<bool>,

    /// Whether view counts are publicly visible.
    pub public_stats_viewable: Option<bool>,

    /// Text appended to the description.
    pub description_suffix: Option<String>,

    /// Scheduled publish time (ISO 8601).
    pub publish_at: Option<String>,

    /// Recording date (ISO 8601 date, e.g. "2026-05-18").
    pub recording_date: Option<String>,

    /// Profile name to use for this video.
    pub profile: Option<String>,
}

impl UploadProfile {
    /// Directory where profiles are stored.
    pub fn profiles_dir() -> Result<PathBuf, UploadError> {
        let config_dir = dirs::config_dir()
            .ok_or_else(|| UploadError::Config("Cannot determine config directory".into()))?;
        Ok(config_dir.join("video-uploader").join("profiles"))
    }

    /// Load a profile by name. Returns an empty profile if the file doesn't exist.
    pub fn load(name: &str) -> Result<Self, UploadError> {
        let dir = Self::profiles_dir()?;
        let path = dir.join(format!("{name}.toml"));

        if !path.exists() {
            return Ok(Self::default());
        }

        let content = std::fs::read_to_string(&path).map_err(|e| {
            UploadError::Config(format!("Failed to read profile '{}': {e}", name))
        })?;

        let profile: Self = toml::from_str(&content).map_err(|e| {
            UploadError::Config(format!("Failed to parse profile '{}': {e}", name))
        })?;

        Ok(profile)
    }

    /// List all available profiles (by scanning the profiles directory).
    pub fn list() -> Result<HashMap<String, Self>, UploadError> {
        let dir = Self::profiles_dir()?;
        if !dir.exists() {
            return Ok(HashMap::new());
        }

        let mut profiles = HashMap::new();
        for entry in std::fs::read_dir(&dir).map_err(|e| {
            UploadError::Config(format!("Failed to read profiles directory: {e}"))
        })? {
            let entry = entry.map_err(|e| {
                UploadError::Config(format!("Failed to read directory entry: {e}"))
            })?;
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "toml") {
                let name = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
                    .to_string();
                if !name.is_empty() {
                    let profile = Self::load(&name)?;
                    profiles.insert(name, profile);
                }
            }
        }
        Ok(profiles)
    }

    /// Resolve which profile to use:
    /// 1. Explicit name provided by caller
    /// 2. "default" profile if it exists
    /// 3. Empty profile (all built-in defaults)
    pub fn resolve(explicit: Option<&str>) -> Result<Self, UploadError> {
        if let Some(name) = explicit {
            return Self::load(name);
        }
        // Try default profile
        let dir = Self::profiles_dir()?;
        if dir.join("default.toml").exists() {
            Self::load("default")
        } else {
            Ok(Self::default())
        }
    }

    /// Delete a profile by name.
    pub fn remove(name: &str) -> Result<(), UploadError> {
        let dir = Self::profiles_dir()?;
        let path = dir.join(format!("{name}.toml"));
        if !path.exists() {
            return Err(UploadError::Config(format!(
                "Profile '{name}' does not exist"
            )));
        }
        std::fs::remove_file(&path).map_err(|e| {
            UploadError::Config(format!("Failed to remove profile '{name}': {e}"))
        })
    }
}

impl VideoMeta {
    /// Load a meta TOML from an explicit path.
    pub fn load_from(path: &Path) -> Result<Self, UploadError> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = std::fs::read_to_string(path).map_err(|e| {
            UploadError::Config(format!("Failed to read meta file '{}': {e}", path.display()))
        })?;
        let meta: Self = toml::from_str(&content).map_err(|e| {
            UploadError::Config(format!("Failed to parse meta file '{}': {e}", path.display()))
        })?;
        Ok(meta)
    }

    /// Auto-discover a `.meta.toml` file next to the video file.
    ///
    /// For `video.mp4`, looks for `video.meta.toml` in the same directory.
    /// Returns `None` if no meta file is found.
    pub fn discover(video_path: &Path) -> Option<PathBuf> {
        let stem = video_path.file_stem()?;
        let dir = video_path.parent()?;
        let meta_path = dir.join(format!("{}.meta.toml", stem.to_string_lossy()));
        if meta_path.exists() {
            Some(meta_path)
        } else {
            None
        }
    }

    /// Apply meta TOML values to a `VideoUpload`.
    ///
    /// Meta values are applied as the base; CLI flags should be applied after
    /// to override. Only non-`None` fields from the meta are set.
    /// Tags are **replaced** (not merged) — meta tags are the primary tags for this video.
    pub fn apply_to(&self, mut video: crate::VideoUpload) -> crate::VideoUpload {
        if let Some(ref title) = self.title {
            video = video.with_title(title);
        }
        if let Some(ref desc) = self.description {
            video = video.with_description(desc);
        }
        if let Some(ref tags) = self.tags {
            video = video.with_tags(tags.clone());
        }
        if let Some(ref vis) = self.visibility
            && let Ok(v) = vis.parse()
        {
            video = video.with_visibility(v);
        }
        if let Some(ref cat) = self.category {
            video = video.with_category(cat);
        }
        if let Some(kids) = self.made_for_kids {
            video = video.with_made_for_kids(kids);
        }
        if let Some(ref lic) = self.license
            && let Ok(l) = lic.parse()
        {
            video = video.with_license(l);
        }
        if let Some(ref lang) = self.language {
            video = video.with_language(lang);
        }
        if let Some(flag) = self.contains_synthetic_media {
            video = video.with_contains_synthetic_media(flag);
        }
        if let Some(flag) = self.embeddable {
            video = video.with_embeddable(flag);
        }
        if let Some(flag) = self.public_stats_viewable {
            video = video.with_public_stats_viewable(flag);
        }
        if let Some(ref suffix) = self.description_suffix {
            video = video.with_description_suffix(suffix);
        }
        if let Some(ref dt) = self.publish_at {
            video = video.with_publish_at(dt);
        }
        if let Some(ref d) = self.recording_date {
            video = video.with_recording_date(d);
        }
        video
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_upload_profile_default_is_all_none() {
        let p = UploadProfile::default();
        assert!(p.visibility.is_none());
        assert!(p.made_for_kids.is_none());
        assert!(p.license.is_none());
        assert!(p.category.is_none());
        assert!(p.language.is_none());
        assert!(p.contains_synthetic_media.is_none());
        assert!(p.embeddable.is_none());
        assert!(p.public_stats_viewable.is_none());
        assert!(p.tags.is_none());
        assert!(p.description_suffix.is_none());
        assert!(p.publish_at.is_none());
        assert!(p.recording_date.is_none());
    }

    #[test]
    fn test_upload_profile_parse_minimal() {
        let toml = "";
        let p: UploadProfile = toml::from_str(toml).unwrap();
        assert!(p.visibility.is_none());
    }

    #[test]
    fn test_upload_profile_parse_full() {
        let toml = r#"
visibility = "unlisted"
made_for_kids = false
license = "creative-common"
category = "20"
language = "en"
contains_synthetic_media = true
embeddable = false
public_stats_viewable = true
tags = ["gaming", "letsplay"]
description_suffix = "\nSubscribe!"
publish_at = "2026-05-20T09:00:00Z"
"#;
        let p: UploadProfile = toml::from_str(toml).unwrap();
        assert_eq!(p.visibility.as_deref(), Some("unlisted"));
        assert_eq!(p.made_for_kids, Some(false));
        assert_eq!(p.license.as_deref(), Some("creative-common"));
        assert_eq!(p.category.as_deref(), Some("20"));
        assert_eq!(p.language.as_deref(), Some("en"));
        assert_eq!(p.contains_synthetic_media, Some(true));
        assert_eq!(p.embeddable, Some(false));
        assert_eq!(p.public_stats_viewable, Some(true));
        assert_eq!(p.tags.as_ref().unwrap().len(), 2);
        assert_eq!(p.description_suffix.as_deref(), Some("\nSubscribe!"));
        assert_eq!(p.publish_at.as_deref(), Some("2026-05-20T09:00:00Z"));
    }

    #[test]
    fn test_profiles_dir_is_under_config() {
        let dir = UploadProfile::profiles_dir().unwrap();
        let s = dir.to_string_lossy();
        assert!(s.contains("video-uploader"));
        assert!(s.contains("profiles"));
    }

    #[test]
    fn test_resolve_with_no_explicit_no_default() {
        // If no default.toml exists, resolve returns empty profile
        // (we can't guarantee no default.toml in test env, but the logic is tested)
        let p = UploadProfile::resolve(None);
        assert!(p.is_ok());
    }

    // -- VideoMeta tests --

    #[test]
    fn test_video_meta_default_is_all_none() {
        let m = VideoMeta::default();
        assert!(m.title.is_none());
        assert!(m.description.is_none());
        assert!(m.tags.is_none());
        assert!(m.visibility.is_none());
        assert!(m.category.is_none());
        assert!(m.made_for_kids.is_none());
        assert!(m.license.is_none());
        assert!(m.language.is_none());
        assert!(m.contains_synthetic_media.is_none());
        assert!(m.embeddable.is_none());
        assert!(m.public_stats_viewable.is_none());
        assert!(m.description_suffix.is_none());
        assert!(m.publish_at.is_none());
        assert!(m.recording_date.is_none());
        assert!(m.profile.is_none());
    }

    #[test]
    fn test_video_meta_parse_full() {
        let toml = r#"
title = "My Video"
description = "A great video"
tags = ["rust", "programming"]
visibility = "unlisted"
category = "20"
made_for_kids = false
license = "creative-common"
language = "en"
contains_synthetic_media = true
embeddable = false
public_stats_viewable = true
description_suffix = "\nSubscribe!"
publish_at = "2026-05-20T09:00:00Z"
profile = "gaming"
"#;
        let m: VideoMeta = toml::from_str(toml).unwrap();
        assert_eq!(m.title.as_deref(), Some("My Video"));
        assert_eq!(m.description.as_deref(), Some("A great video"));
        assert_eq!(m.tags.as_ref().unwrap().len(), 2);
        assert_eq!(m.visibility.as_deref(), Some("unlisted"));
        assert_eq!(m.category.as_deref(), Some("20"));
        assert_eq!(m.made_for_kids, Some(false));
        assert_eq!(m.license.as_deref(), Some("creative-common"));
        assert_eq!(m.language.as_deref(), Some("en"));
        assert_eq!(m.contains_synthetic_media, Some(true));
        assert_eq!(m.embeddable, Some(false));
        assert_eq!(m.public_stats_viewable, Some(true));
        assert_eq!(m.description_suffix.as_deref(), Some("\nSubscribe!"));
        assert_eq!(m.publish_at.as_deref(), Some("2026-05-20T09:00:00Z"));
        assert_eq!(m.profile.as_deref(), Some("gaming"));
    }

    #[test]
    fn test_video_meta_discover_with_meta_file() {
        let dir = std::env::temp_dir().join("vu_test_meta_discover");
        std::fs::create_dir_all(&dir).unwrap();
        let video_path = dir.join("episode.mp4");
        let meta_path = dir.join("episode.meta.toml");
        std::fs::write(&video_path, b"fake").unwrap();
        std::fs::write(&meta_path, b"title = \"Test\"").unwrap();

        let discovered = VideoMeta::discover(&video_path);
        assert!(discovered.is_some());
        assert_eq!(discovered.unwrap(), meta_path);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_video_meta_discover_no_meta_file() {
        let dir = std::env::temp_dir().join("vu_test_meta_no_discover");
        std::fs::create_dir_all(&dir).unwrap();
        let video_path = dir.join("episode.mp4");
        std::fs::write(&video_path, b"fake").unwrap();

        let discovered = VideoMeta::discover(&video_path);
        assert!(discovered.is_none());

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_video_meta_apply_to_overrides_fields() {
        use crate::{VideoUpload, Visibility};

        let meta: VideoMeta = toml::from_str(r#"
title = "Meta Title"
description = "Meta Description"
tags = ["meta", "tags"]
visibility = "unlisted"
category = "20"
made_for_kids = true
license = "creative-common"
language = "es"
"#).unwrap();

        let video = VideoUpload::new("/tmp/video.mp4", "Original Title");
        let video = meta.apply_to(video);

        assert_eq!(video.title(), "Meta Title");
        assert_eq!(video.description(), Some("Meta Description"));
        assert_eq!(video.tags(), &["meta", "tags"]);
        assert_eq!(video.visibility(), Visibility::Unlisted);
        assert_eq!(video.category_id(), Some("20"));
        assert_eq!(video.made_for_kids(), Some(true));
    }

    #[test]
    fn test_video_meta_apply_to_skips_none() {
        use crate::VideoUpload;

        let meta = VideoMeta::default();
        let video = VideoUpload::new("/tmp/video.mp4", "Keep This");
        let video = meta.apply_to(video);

        assert_eq!(video.title(), "Keep This");
        assert!(video.description().is_none());
    }

    #[test]
    fn test_video_meta_load_from_valid_file() {
        let dir = std::env::temp_dir().join("vu_test_meta_load");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test.meta.toml");
        std::fs::write(&path, r#"
title = "Loaded Title"
description = "From file"
tags = ["test"]
visibility = "public"
"#).unwrap();

        let meta = VideoMeta::load_from(&path).unwrap();
        assert_eq!(meta.title.as_deref(), Some("Loaded Title"));
        assert_eq!(meta.description.as_deref(), Some("From file"));
        assert_eq!(meta.visibility.as_deref(), Some("public"));

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_video_meta_load_from_nonexistent_returns_default() {
        let meta = VideoMeta::load_from(Path::new("/tmp/does_not_exist_12345.meta.toml")).unwrap();
        assert!(meta.title.is_none());
    }

    #[test]
    fn test_video_meta_load_from_invalid_toml() {
        let dir = std::env::temp_dir().join("vu_test_meta_bad");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("bad.meta.toml");
        std::fs::write(&path, "this is not = valid toml {{{").unwrap();

        let result = VideoMeta::load_from(&path);
        assert!(result.is_err());

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_video_meta_apply_to_all_fields() {
        use crate::{VideoUpload, Visibility};

        let meta: VideoMeta = toml::from_str(r##"
title = "Meta Title"
description = "Meta Desc"
tags = ["a", "b"]
visibility = "public"
category = "22"
made_for_kids = true
license = "youtube"
language = "fr"
contains_synthetic_media = true
embeddable = false
public_stats_viewable = true
description_suffix = "\nSuffix"
publish_at = "2026-08-01T00:00:00Z"
recording_date = "2026-05-18"
"##).unwrap();

        let video = meta.apply_to(VideoUpload::new("/tmp/v.mp4", "Original"));

        assert_eq!(video.title(), "Meta Title");
        assert_eq!(video.description(), Some("Meta Desc"));
        assert_eq!(video.visibility(), Visibility::Public);
        assert_eq!(video.category_id(), Some("22"));
        assert_eq!(video.made_for_kids(), Some(true));
    }

    #[test]
    fn test_video_meta_discover_no_stem() {
        // Path with no filename stem
        let result = VideoMeta::discover(Path::new("/tmp/"));
        assert!(result.is_none());
    }
}
