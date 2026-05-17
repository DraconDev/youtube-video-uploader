//! Client-side upload resume support.
//!
//! When an upload is interrupted (network timeout, connection loss),
//! the state can be saved and resumed later without re-uploading
//! already-transmitted chunks.

use crate::UploadError;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// State captured from an interrupted upload, sufficient to resume.
///
/// # Examples
///
/// ```no_run
/// use video_uploader::{YouTubeUploader, VideoUpload, CredentialStore};
/// use std::sync::Arc;
/// use tokio::sync::Mutex;
///
/// # #[tokio::main]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let store = Arc::new(Mutex::new(CredentialStore::load("pass")?));
/// let uploader = YouTubeUploader::new(store, "pass", "youtube");
/// let video = VideoUpload::new("/path/to/video.mp4", "My Video");
///
/// match uploader.upload(&video, None).await {
///     Ok(result) => println!("Uploaded: {}", result.url),
///     Err(e) => {
///         if let Some(state) = YouTubeUploader::extract_resume_state(&e) {
///             println!("Interrupted at {} bytes. Save state to resume later.", state.uploaded_bytes);
///             state.save()?;
///         }
///     }
/// }
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadState {
    /// The resumable upload URL from Google's initiate endpoint.
    pub upload_url: String,
    /// Bytes already acknowledged by the server.
    pub uploaded_bytes: u64,
    /// Total file size in bytes.
    pub total_size: u64,
    /// Path to the local video file.
    pub file_path: PathBuf,
    /// Video title (for display/logging).
    pub title: String,
    /// Workspace name.
    pub workspace: String,
}

impl UploadState {
    /// Directory where resume state files are stored.
    pub fn resume_dir() -> Result<PathBuf, UploadError> {
        let config_dir = dirs::config_dir()
            .ok_or_else(|| UploadError::Config("Cannot determine config directory".into()))?;
        Ok(config_dir.join("video-uploader").join("resume"))
    }

    /// Generate a unique filename for this upload state based on file path + title.
    fn state_filename(&self) -> String {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.file_path.hash(&mut hasher);
        self.title.hash(&mut hasher);
        format!("{:016x}.json", hasher.finish())
    }

    /// Save the resume state to disk.
    ///
    /// State is written to `~/.config/video-uploader/resume/{hash}.json`.
    pub fn save(&self) -> Result<PathBuf, UploadError> {
        let dir = Self::resume_dir()?;
        std::fs::create_dir_all(&dir).map_err(|e| UploadError::Config(format!(
            "Failed to create resume directory: {e}"
        )))?;

        let path = dir.join(self.state_filename());
        let json = serde_json::to_string_pretty(self).map_err(|e| UploadError::Config(format!(
            "Failed to serialize resume state: {e}"
        )))?;

        std::fs::write(&path, json).map_err(|e| UploadError::Config(format!(
            "Failed to write resume state: {e}"
        )))?;

        Ok(path)
    }

    /// Load the most recent resume state for a given file path.
    pub fn load_for_file(file_path: &Path) -> Result<Option<Self>, UploadError> {
        let dir = Self::resume_dir()?;
        if !dir.exists() {
            return Ok(None);
        }

        // Look for any state file and check if it matches
        for entry in std::fs::read_dir(&dir).map_err(|e| UploadError::Config(format!(
            "Failed to read resume directory: {e}"
        )))? {
            let entry = entry.map_err(|e| UploadError::Config(format!(
                "Failed to read directory entry: {e}"
            )))?;
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "json") {
                let content = std::fs::read_to_string(&path).map_err(|e| UploadError::Config(format!(
                    "Failed to read resume state: {e}"
                )))?;
                if let Ok(state) = serde_json::from_str::<Self>(&content)
                    && state.file_path == file_path
                {
                }
            }
        }
        Ok(None)
    }

    /// Delete the saved resume state for this upload.
    pub fn delete(&self) -> Result<(), UploadError> {
        let dir = Self::resume_dir()?;
        let path = dir.join(self.state_filename());
        if path.exists() {
            std::fs::remove_file(&path).map_err(|e| UploadError::Config(format!(
                "Failed to delete resume state: {e}"
            )))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_upload_state_serialization_roundtrip() {
        let state = UploadState {
            upload_url: "https://storage.googleapis.com/upload/abc123".to_string(),
            uploaded_bytes: 8_388_608,
            total_size: 25_000_000,
            file_path: PathBuf::from("/tmp/video.mp4"),
            title: "Test Video".to_string(),
            workspace: "youtube".to_string(),
        };

        let json = serde_json::to_string(&state).unwrap();
        let deserialized: UploadState = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.upload_url, state.upload_url);
        assert_eq!(deserialized.uploaded_bytes, state.uploaded_bytes);
        assert_eq!(deserialized.total_size, state.total_size);
        assert_eq!(deserialized.file_path, state.file_path);
        assert_eq!(deserialized.title, state.title);
        assert_eq!(deserialized.workspace, state.workspace);
    }

    #[test]
    fn test_upload_state_filename_is_deterministic() {
        let state = UploadState {
            upload_url: "https://example.com".to_string(),
            uploaded_bytes: 0,
            total_size: 100,
            file_path: PathBuf::from("/tmp/video.mp4"),
            title: "Test".to_string(),
            workspace: "youtube".to_string(),
        };

        let name1 = state.state_filename();
        let name2 = state.state_filename();
        assert_eq!(name1, name2);
        assert!(name1.ends_with(".json"));
    }

    #[test]
    fn test_upload_state_different_files_different_names() {
        let state1 = UploadState {
            upload_url: String::new(),
            uploaded_bytes: 0,
            total_size: 100,
            file_path: PathBuf::from("/tmp/video1.mp4"),
            title: "Video 1".to_string(),
            workspace: "youtube".to_string(),
        };
        let state2 = UploadState {
            upload_url: String::new(),
            uploaded_bytes: 0,
            total_size: 100,
            file_path: PathBuf::from("/tmp/video2.mp4"),
            title: "Video 2".to_string(),
            workspace: "youtube".to_string(),
        };

        assert_ne!(state1.state_filename(), state2.state_filename());
    }

    #[test]
    fn test_resume_dir_is_under_config() {
        let dir = UploadState::resume_dir().unwrap();
        assert!(dir.to_string_lossy().contains("video-uploader"));
        assert!(dir.to_string_lossy().contains("resume"));
    }
}
