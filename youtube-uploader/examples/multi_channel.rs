//! Multi-channel upload example.
//!
//! Demonstrates uploading to multiple YouTube workspaces (channels)
//! from a single program.
//!
//! Run with: cargo run --example multi_channel

use std::sync::Arc;
use tokio::sync::Mutex;
use youtube_uploader::{
    CredentialStore, NoopProgressListener, VideoUpload, Visibility, YouTubeUploader,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let passphrase = std::env::var("YOUTUBE_UPLOADER_PASSPHRASE")
        .expect("Set YOUTUBE_UPLOADER_PASSPHRASE environment variable");

    let store = Arc::new(Mutex::new(CredentialStore::load(&passphrase)?));

    // List available workspaces
    {
        let s = store.lock().await;
        let default = s.default_workspace();
        println!("Configured workspaces:");
        for ws in s.workspaces() {
            let marker = if default == Some(ws.as_str()) {
                " (default)"
            } else {
                ""
            };
            println!("  - {}{marker}", ws);
        }
    }

    // Upload to the "gaming" workspace
    let gaming_uploader = YouTubeUploader::new(store.clone(), &passphrase, "gaming");
    let gaming_video =
        VideoUpload::new("./gameplay.mp4", "Let's Play").with_visibility(Visibility::Private);

    // Upload to the "cooking" workspace
    let cooking_uploader = YouTubeUploader::new(store.clone(), &passphrase, "cooking");
    let cooking_video =
        VideoUpload::new("./recipe.mp4", "New Recipe").with_visibility(Visibility::Unlisted);

    // Upload both concurrently
    let progress = Arc::new(NoopProgressListener);
    let (gaming_result, cooking_result) = tokio::join!(
        gaming_uploader.upload(&gaming_video, Some(progress.clone())),
        cooking_uploader.upload(&cooking_video, Some(progress.clone())),
    );

    if let Ok(r) = gaming_result {
        println!("✓ Gaming: {} (ID: {})", r.url, r.video_id);
    }
    if let Ok(r) = cooking_result {
        println!("✓ Cooking: {} (ID: {})", r.url, r.video_id);
    }

    Ok(())
}
