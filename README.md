# video-uploader

A Rust library for uploading videos to multiple platforms with a unified API.

## Supported Platforms

| Platform | Auth Method | Status |
|----------|-------------|--------|
| **YouTube** | OAuth2 (device code flow + refresh tokens) | Stable |
| **Odysee** | LBRY SDK daemon | Beta |

## Features

- **Unified API**: Single `VideoUpload` struct + `PlatformUploader` trait for all platforms
- **Encrypted credential storage**: AES-GCM encrypted credentials on disk, passphrase-protected
- **Auto token refresh**: YouTube access tokens automatically refreshed before upload
- **Progress reporting**: `ProgressListener` trait for upload callbacks
- **Multi-platform**: Upload to one or all platforms concurrently

## Installation

```toml
[dependencies]
video-uploader = "0.1"
tokio = { version = "1", features = ["full"] }
```

## Quick Start

### CLI Usage

```bash
# Authenticate with YouTube (one-time setup)
export YOUTUBE_CLIENT_ID=your_client_id
export YOUTUBE_CLIENT_SECRET=your_client_secret
video-uploader auth --platform youtube

# Upload a video
video-uploader upload --file video.mp4 --title "My Video"

# List configured platforms
video-uploader list
```

### Library Usage

```rust
use std::sync::Arc;
use video_uploader::{
    UploaderRegistry, VideoUpload, Visibility, StderrProgressListener,
};

// Load registry from encrypted credential store
let registry = UploaderRegistry::load("my-passphrase")?;
let progress = Arc::new(StderrProgressListener);

// Build upload metadata
let video = VideoUpload::new("/path/to/video.mp4", "My Video Title")
    .description("Video description")
    .tags(vec!["tag1".to_string(), "tag2".to_string()])
    .visibility(Visibility::Public);

// Upload to a single platform
let result = registry.upload_to("youtube", &video, Some(progress.clone())).await?;
println!("Uploaded to {}: {}", result.platform, result.url);

// Or upload to all configured platforms concurrently
let results = registry.upload_to_all(&video, Some(progress)).await;
for (platform, result) in results {
    match result {
        Ok(r) => println!("{}: {}", platform, r.url),
        Err(e) => eprintln!("{} failed: {}", platform, e),
    }
}
```

## Platform Setup

### YouTube

1. Go to [Google Cloud Console](https://console.cloud.google.com/apis/credentials)
2. Create an OAuth 2.0 Client ID (Desktop app or Other)
3. Get your `Client ID` and `Client Secret`
4. Run `video-uploader auth --platform youtube --client-id YOUR_ID --client-secret YOUR_SECRET`
5. Follow the on-screen instructions (open URL, enter code)

### Odysee / LBRY

1. Install and run the LBRY daemon:
   ```
   # Download from https://github.com/lbryio/lbry-sdk/releases
   lbrynet start
   ```
2. Configure the daemon URL (defaults to `http://localhost:5279`)
3. Run `video-uploader auth --platform odysee`

   Set `ODYSEE_DAEMON_URL` env var if using a non-default URL.

## Architecture

```
video-uploader/
├── src/
│   ├── lib.rs              # Public exports, PlatformUploader trait
│   ├── error.rs            # UploadError enum
│   ├── config.rs           # Encrypted CredentialStore
│   ├── upload.rs           # VideoUpload, UploadResult, Visibility
│   ├── progress.rs         # ProgressListener trait
│   ├── registry.rs         # UploaderRegistry for multi-platform dispatch
│   ├── auth/
│   │   ├── device_code.rs  # Google OAuth2 device code flow
│   │   └── refresh_token.rs # Token refresh logic
│   └── platforms/
│       ├── youtube.rs      # YouTube uploader
│       └── odysee.rs       # Odysee/LBRY uploader
```

## Adding New Platforms

Implement `PlatformUploader` for your platform:

```rust
use async_trait::async_trait;
use video_uploader::{PlatformUploader, UploadError, UploadResult, VideoUpload};
use std::sync::Arc;

pub struct MyUploader { /* ... */ }

#[async_trait]
impl PlatformUploader for MyUploader {
    fn platform_name(&self) -> &'static str {
        "my-platform"
    }

    async fn upload(
        &self,
        video: &VideoUpload,
        progress: Option<Arc<dyn ProgressListener>>,
    ) -> Result<UploadResult, UploadError> {
        // Your upload logic here
    }
}
```

## Security

Credentials are encrypted with AES-GCM using a key derived from your passphrase (SHA-256 + AES-256-GCM). The encrypted file is stored at `~/.config/video-uploader/credentials.enc`.

## License

MIT OR Apache-2.0