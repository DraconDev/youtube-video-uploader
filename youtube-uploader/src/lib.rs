//! # youtube-uploader
//!
//! A Rust library for uploading videos to YouTube via the **Data API v3** resumable upload
//! endpoint. Run it, it uploads, it exits — no daemon, no background process.
//!
//! ## Quick Start
//!
//! ```no_run
//! use std::sync::Arc;
//! use tokio::sync::Mutex;
//! use youtube_uploader::{
//!     CredentialStore, YouTubeUploader, VideoUpload, Visibility, StderrProgressListener,
//! };
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let store = Arc::new(Mutex::new(CredentialStore::load("my-passphrase")?));
//! let youtube = YouTubeUploader::new(store, "my-passphrase", "youtube");
//! let progress = Arc::new(StderrProgressListener::new());
//!
//! let video = VideoUpload::new("/path/to/video.mp4", "My Video Title")
//!     .with_description("Video description")
//!     .with_tags(vec!["tag1".to_string(), "tag2".to_string()])
//!     .with_visibility(Visibility::Private);
//!
//! let result = youtube.upload(&video, Some(progress)).await?;
//! println!("Uploaded: {} (ID: {})", result.url, result.video_id);
//! # Ok(())
//! # }
//! ```
//!
//! ## Features
//!
//! - **Resumable chunked upload** with 308 resume support and crash recovery
//! - **Multi-channel workspaces** — upload to multiple YouTube accounts from one machine
//! - **Upload profiles** — TOML-based presets for reusable upload defaults
//! - **Per-video metadata TOML** — AI-friendly `.meta.toml` files for automation
//! - **Encrypted credential storage** — AES-256-GCM, PBKDF2 100K, zeroize on drop
//! - **Default visibility = Private** — uploads never accidentally go public
//!
//! ## Architecture
//!
//! | Module | Purpose |
//! |--------|---------|
//! | [`YouTubeUploader`] | Resumable upload, token refresh, delete, channel info |
//! | [`VideoUpload`] | Video metadata builder (title, description, tags, visibility, etc.) |
//! | [`CredentialStore`] | Encrypted on-disk credential storage with workspaces |
//! | [`UploadProfile`] | Named TOML presets for upload defaults |
//! | [`VideoMeta`] | Per-video `.meta.toml` metadata |
//! | [`UploadState`] | Crash recovery state for resumable uploads |
//! | [`ProgressListener`] | Trait for custom upload progress callbacks |
//!
//! ## Resolution Order
//!
//! When multiple sources provide the same field:
//!
//! ```text
//! CLI flags > meta TOML > profile TOML > built-in defaults (private)
//! ```
//!
//! Tags are **merged** (profile + video), not replaced.

pub mod auth;
pub mod config;
pub mod error;
pub mod net;
pub mod profile;
pub mod progress;
pub mod resume;
pub mod upload;
pub mod validation;
pub mod youtube;

pub use config::{CredentialStore, PlatformCredentials};
pub use error::UploadError;
pub use net::is_private_ip;
pub use profile::UploadProfile;
pub use profile::VideoMeta;
pub use progress::{NoopProgressListener, ProgressListener, StderrProgressListener};
pub use resume::UploadState;
pub use upload::{License, UploadResult, VideoUpload, Visibility};
pub use youtube::YouTubeUploader;
pub use zeroize::Zeroizing;
