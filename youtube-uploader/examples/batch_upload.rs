//! Batch upload example.
//!
//! Demonstrates programmatic batch upload with concurrency control.
//!
//! Run with: cargo run --example batch_upload

use std::sync::Arc;
use tokio::sync::{Mutex, Semaphore};
use youtube_uploader::{
    CredentialStore, NoopProgressListener, VideoUpload, Visibility, YouTubeUploader,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let passphrase = std::env::var("YOUTUBE_UPLOADER_PASSPHRASE")
        .expect("Set YOUTUBE_UPLOADER_PASSPHRASE environment variable");

    let store = Arc::new(Mutex::new(CredentialStore::load(&passphrase)?));

    // Define videos to upload (file, title, workspace)
    let entries = vec![
        ("./video1.mp4", "First Video", "youtube"),
        ("./video2.mp4", "Second Video", "youtube"),
        ("./gameplay.mp4", "Gameplay", "gaming"),
    ];

    let concurrency = 2;
    let semaphore = Arc::new(Semaphore::new(concurrency));
    let progress = Arc::new(NoopProgressListener);
    let mut handles = Vec::new();

    for (file, title, workspace) in entries {
        let store = store.clone();
        let passphrase = passphrase.clone();
        let semaphore = semaphore.clone();
        let progress = progress.clone();
        let file = file.to_string();
        let title = title.to_string();
        let workspace = workspace.to_string();

        let handle = tokio::spawn(async move {
            let _permit = semaphore.acquire().await.unwrap();
            let uploader = YouTubeUploader::new(store, &passphrase, &workspace);
            let video = VideoUpload::new(&file, &title).with_visibility(Visibility::Private);
            uploader.upload(&video, Some(progress)).await
        });
        handles.push(handle);
    }

    let mut successes = 0;
    let mut errors = 0;
    for handle in handles {
        match handle.await.unwrap() {
            Ok(result) => {
                println!(
                    "✓ [{}] {} (ID: {})",
                    result.workspace, result.url, result.video_id
                );
                successes += 1;
            }
            Err(e) => {
                eprintln!("✗ Upload failed: {e}");
                errors += 1;
            }
        }
    }

    println!("\nBatch complete: {successes} succeeded, {errors} failed");
    Ok(())
}
