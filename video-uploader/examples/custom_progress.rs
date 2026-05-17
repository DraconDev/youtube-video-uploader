//! Custom progress listener example.
//!
//! Shows how to implement the ProgressListener trait
//! for custom upload progress reporting.
//!
//! Run with: cargo run --example custom_progress

use std::sync::Arc;
use tokio::sync::Mutex;
use video_uploader::{
    CredentialStore, ProgressListener, UploadError, UploadResult, VideoUpload, YouTubeUploader,
};

/// A progress listener that prints a simple percentage bar.
struct ProgressBar;

impl ProgressListener for ProgressBar {
    fn on_progress(&self, uploaded: u64, total: u64) {
        let pct = uploaded as f64 / total as f64 * 100.0;
        let filled = (pct / 5.0) as usize;
        let bar: String = "█".repeat(filled) + &"░".repeat(20 - filled);
        eprint!("\r  [{bar}] {pct:5.1}%  ");
    }

    fn on_complete(&self, result: &UploadResult) {
        eprintln!("\n✓ Done: {} (ID: {})", result.url, result.video_id);
    }

    fn on_error(&self, error: &UploadError) {
        eprintln!("\n✗ Error: {error}");
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let passphrase = std::env::var("VIDEO_UPLOADER_PASSPHRASE")
        .expect("Set VIDEO_UPLOADER_PASSPHRASE environment variable");

    let store = Arc::new(Mutex::new(CredentialStore::load(&passphrase)?));
    let uploader = YouTubeUploader::new(store, &passphrase, "youtube");

    let video = VideoUpload::new("./my_video.mp4", "Custom Progress Demo");
    let progress = Arc::new(ProgressBar);

    let result = uploader.upload(&video, Some(progress)).await?;
    println!("Workspace: {}", result.workspace);
    Ok(())
}
