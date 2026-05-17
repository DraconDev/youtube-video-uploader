# video-uploader

A Rust library and CLI for uploading videos to YouTube via the Data API v3. Supports resumable chunked uploads, encrypted credential storage, **multi-channel workspaces**, and batch processing.

## Features

- **YouTube API uploads**: Resumable chunked upload with 308 resume support
- **Multi-channel workspaces**: Upload to multiple YouTube accounts from a single installation
- **Encrypted credential storage**: AES-256-GCM encrypted credentials on disk, PBKDF2-derived key
- **Auto token refresh**: YouTube access tokens automatically refreshed before upload
- **Progress reporting**: `ProgressListener` trait for custom upload callbacks
- **Batch uploads**: CSV manifest with configurable concurrency and per-row workspace selection
- **Client-side validation**: File size, extension, and title checked before uploading

## Installation

```toml
[dependencies]
video-uploader = "0.2"
tokio = { version = "1", features = ["full"] }
```

### CLI

```bash
cargo install video-uploader-cli
```

## Quick Start

### CLI Usage

```bash
# Authenticate with YouTube (one-time setup per channel)
export YOUTUBE_CLIENT_ID=your_client_id
export YOUTUBE_CLIENT_SECRET=your_client_secret
video-uploader auth                        # saves to "youtube" workspace (default)

# Upload a video
video-uploader upload --file video.mp4 --title "My Video"

# Upload to a specific workspace
video-uploader -w gaming upload --file gameplay.mp4 --title "Let's Play"

# List configured workspaces
video-uploader list

# Batch upload from CSV manifest
video-uploader batch --manifest videos.csv --concurrency 4

# Manage workspaces
video-uploader workspace default gaming     # set default
video-uploader workspace rename gaming letsplay
video-uploader workspace remove old-channel
```

### Library Usage

```rust
use std::sync::Arc;
use tokio::sync::Mutex;
use video_uploader::{
    CredentialStore, YouTubeUploader, VideoUpload, Visibility, StderrProgressListener,
};

let store = Arc::new(Mutex::new(CredentialStore::load("my-passphrase")?));
let youtube = YouTubeUploader::new(store, "my-passphrase", "youtube");
let progress = Arc::new(StderrProgressListener);

let video = VideoUpload::new("/path/to/video.mp4", "My Video Title")
    .description("Video description")
    .tags(vec!["tag1".to_string(), "tag2".to_string()])
    .visibility(Visibility::Public);

let result = youtube.upload(&video, Some(progress.clone())).await?;
println!("Uploaded: {} (workspace: {})", result.url, result.workspace);
```

## Multiple Channels (Workspaces)

Each YouTube account is stored as a named **workspace**. The first workspace you authenticate becomes the default.

```bash
# Authenticate two channels
video-uploader auth              # → workspace "youtube" (auto-named, becomes default)
video-uploader -w cooking auth   # → workspace "cooking"

# Upload to each
video-uploader upload --file vlog.mp4 --title "Vlog"            # uses default
video-uploader -w cooking upload --file recipe.mp4 --title "Recipe"  # uses cooking

# Switch default
video-uploader workspace default cooking
```

### Batch CSV with Workspaces

```csv
file,title,workspace,description,tags,visibility
gaming1.mp4,Gameplay 1,gaming,Let's play,rust|gaming,public
cooking1.mp4,Recipe,cooking,My recipe,food|cooking,unlisted
default.mp4,Vlog,,,vlog,public
```

The `workspace` column is optional — rows without it use the default workspace.

## YouTube Setup

1. Go to [Google Cloud Console](https://console.cloud.google.com/apis/credentials)
2. Create an OAuth 2.0 Client ID (Desktop app type)
3. Enable the YouTube Data API v3
4. Run `video-uploader auth --client-id YOUR_ID --client-secret YOUR_SECRET`
5. Follow the on-screen instructions (open URL, enter device code)

**Note**: YouTube API quota is 10,000 units/day (~6 uploads/day per channel).

## Architecture

```
video-uploader/               # Library crate
├── src/
│   ├── lib.rs                 # Public exports
│   ├── youtube.rs             # YouTubeUploader (resumable upload, token refresh, delete)
│   ├── error.rs               # UploadError enum
│   ├── config.rs              # Encrypted CredentialStore with workspaces
│   ├── upload.rs              # VideoUpload, UploadResult, Visibility
│   ├── validation.rs          # File validation (size, extension, title)
│   ├── progress.rs            # ProgressListener trait + StderrProgressListener
│   ├── net.rs                 # HTTP client, retry logic, SSRF protection
│   └── auth/
│       ├── device_code.rs     # Google OAuth2 device code flow
│       └── refresh_token.rs   # Token refresh logic

video-uploader-cli/            # Binary crate
├── src/
│   └── main.rs                # CLI with auth, upload, batch, list, workspace subcommands
└── tests/
    └── cli.rs                 # CLI integration tests
```

## Security

Credentials are encrypted with AES-256-GCM using a key derived from your passphrase via PBKDF2 (100K iterations). Sensitive data is zeroized on drop. The encrypted file is stored at `~/.config/video-uploader/credentials.enc`.

## License

This project is dual-licensed:

- **AGPL-3.0-only** — See [LICENSE](LICENSE) for the full text. This is the default license for open source use.
- **Commercial License** — For organizations that prefer not to comply with AGPLv3's source disclosure requirements. See [COMMERCIAL-LICENSE.md](COMMERCIAL-LICENSE.md) for details.

By contributing to this project, you agree to the terms in [CLA.md](CLA.md).
