//! Basic single-video upload example.
//!
//! Run with: cargo run --example basic_upload
//!
//! Requires a configured workspace (run `youtube-uploader auth` first).

use std::sync::Arc;
use tokio::sync::Mutex;
use youtube_uploader::{
    CredentialStore, StderrProgressListener, VideoUpload, Visibility, YouTubeUploader,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let passphrase = std::env::var("YOUTUBE_UPLOADER_PASSPHRASE")
        .expect("Set YOUTUBE_UPLOADER_PASSPHRASE environment variable");

    let store = Arc::new(Mutex::new(CredentialStore::load(&passphrase)?));
    let uploader = YouTubeUploader::new(store, &passphrase, "youtube");

    let video = VideoUpload::new("./my_video.mp4", "My Awesome Video")
        .with_description("Uploaded with youtube-uploader")
        .with_tags(vec!["rust".into(), "programming".into()])
        .with_visibility(Visibility::Private);

    let progress = Arc::new(StderrProgressListener::new());
    let result = uploader.upload(&video, Some(progress)).await?;

    println!("✓ Uploaded: {} (ID: {})", result.url, result.video_id);
    Ok(())
}
