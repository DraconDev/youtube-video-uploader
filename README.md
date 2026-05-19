# video-uploader

A Rust library and **CLI tool** for uploading videos to YouTube via the Data API v3. You run it, it uploads, it exits. **No daemon, no background process, no service.**

📖 **[Full Guide → GUIDE.md](GUIDE.md)** — setup, CLI reference, profiles, automation, architecture, security, roadmap.

## Quick Start

```bash
# Authenticate (one-time per channel)
video-uploader auth

# Upload a video (private by default)
video-uploader upload --file video.mp4 --title "My Video"

# JSON output for scripts
video-uploader --output json upload --file video.mp4 --title "My Video"

# Multi-channel
video-uploader -w gaming upload --file gameplay.mp4 --title "Let's Play"

# Upload profile
video-uploader -P gaming upload --file gameplay.mp4 --title "Stream"

# Batch
video-uploader batch manifest.csv --concurrency 2
```

## Library Usage

```rust
use std::sync::Arc;
use tokio::sync::Mutex;
use video_uploader::{
    CredentialStore, YouTubeUploader, VideoUpload, Visibility, StderrProgressListener,
};

let store = Arc::new(Mutex::new(CredentialStore::load("my-passphrase")?));
let youtube = YouTubeUploader::new(store, "my-passphrase", "youtube");
let progress = Arc::new(StderrProgressListener::new());

let video = VideoUpload::new("/path/to/video.mp4", "My Video Title")
    .with_description("Video description")
    .with_tags(vec!["tag1".to_string(), "tag2".to_string()])
    .with_visibility(Visibility::Public);

let result = youtube.upload(&video, Some(progress.clone())).await?;
println!("Uploaded: {} (workspace: {})", result.url, result.workspace);
```

## Features

- **YouTube API uploads**: Resumable chunked upload with 308 resume support
- **Multi-channel workspaces**: Upload to multiple YouTube accounts from a single installation
- **Upload profiles**: TOML-based presets for reusable upload defaults
- **Per-video metadata TOML**: AI-friendly `.meta.toml` files for automation pipelines
- **`--output json`**: Machine-readable output for CI/CD and scripting
- **Encrypted credential storage**: AES-256-GCM, PBKDF2 100K, zeroize on drop
- **Default visibility = Private**: Uploads never accidentally go public

## Installation

```bash
cargo install video-uploader-cli
```

```toml
[dependencies]
video-uploader = "0.4"
tokio = { version = "1", features = ["full"] }
```

## License

Dual-licensed:

- **AGPL-3.0-only** — See [LICENSE](LICENSE)
- **Commercial License** — See [COMMERCIAL-LICENSE.md](COMMERCIAL-LICENSE.md)

By contributing, you agree to [CLA.md](CLA.md).
