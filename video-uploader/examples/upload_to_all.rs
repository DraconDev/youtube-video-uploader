//! Example: Upload a video to all configured platforms concurrently using UploaderRegistry.
//!
//! Usage:
//! ```ignore
//! VIDEO_UPLOADER_PASSPHRASE=your-passphrase cargo run --example upload_to_all
//! ```
use std::sync::Arc;
use video_uploader::{StderrProgressListener, UploaderRegistry, VideoUpload, Visibility};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let passphrase =
        std::env::var("VIDEO_UPLOADER_PASSPHRASE").expect("VIDEO_UPLOADER_PASSPHRASE must be set");

    let video = VideoUpload::new("/tmp/test.mp4", "Hello from Rust!")
        .description("Uploaded using the video-uploader Rust library")
        .tags(vec!["rust".to_string(), "demo".to_string()])
        .visibility(Visibility::Public);

    let registry = UploaderRegistry::load(&passphrase)?;
    let progress: Arc<dyn video_uploader::ProgressListener> = Arc::new(StderrProgressListener);

    let results = registry.upload_to_all(&video, Some(progress)).await;
    let mut succeeded = 0;
    for (platform, result) in results {
        match result {
            Ok(r) => {
                println!("{}: {} ({})", platform, r.url, r.platform_id);
                succeeded += 1;
            }
            Err(e) => eprintln!("{} failed: {}", platform, e),
        }
    }

    println!("\nDone. {} platform(s) succeeded.", succeeded);
    Ok(())
}
