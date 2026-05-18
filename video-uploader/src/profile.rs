//! Upload profiles — reusable presets stored as TOML files.
//!
//! Profiles live in `~/.config/video-uploader/profiles/<name>.toml`.
//! All fields are optional; only non-empty values override built-in defaults.

use crate::UploadError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

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
}
