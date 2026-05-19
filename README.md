# video-uploader

A Rust library and **CLI tool** for uploading videos to YouTube via the Data API v3. You run it, it uploads, it exits. **No daemon, no background process, no service.**

Supports resumable chunked uploads, encrypted credential storage, **multi-channel workspaces**, **upload profiles**, and **AI-friendly per-video metadata**.

## How It Works

```
$ video-uploader upload --file video.mp4 --title "My Video"
  → Loads encrypted credentials
  → Refreshes OAuth2 access token
  → Initiates resumable upload session
  → Uploads chunks with progress bar
  → Prints result and EXITS
```

Every invocation is a one-shot process. There is no daemon, no socket, no PID file, no watcher. The binary runs, does its job, and exits. Persistent state between runs lives on disk:

| Path | Purpose |
|------|---------|
| `~/.config/video-uploader/credentials.enc` | Encrypted OAuth2 tokens |
| `~/.config/video-uploader/profiles/*.toml` | Upload preset files |
| `~/.config/video-uploader/resume/` | In-progress upload state (crash recovery) |

## Features

- **YouTube API uploads**: Resumable chunked upload with 308 resume support
- **Multi-channel workspaces**: Upload to multiple YouTube accounts from a single installation
- **Upload profiles**: TOML-based presets for reusable upload defaults
- **Per-video metadata TOML**: AI-friendly `.meta.toml` files for automation pipelines
- **`--output json`**: Machine-readable output for CI/CD and scripting
- **Encrypted credential storage**: AES-256-GCM encrypted credentials on disk, PBKDF2-derived key
- **Auto token refresh**: YouTube access tokens automatically refreshed before upload
- **Progress reporting**: `ProgressListener` trait for custom upload callbacks
- **Batch uploads**: CSV manifest with configurable concurrency, per-row workspace and profile
- **Client-side validation**: File size, extension, and title checked before uploading

## Installation

```toml
[dependencies]
video-uploader = "0.4"
tokio = { version = "1", features = ["full"] }
```

### CLI Binary

```bash
cargo install video-uploader-cli
```

## Quick Start

### CLI Usage

```bash
# Authenticate with YouTube (one-time setup per channel)
video-uploader auth

# Upload a video (private by default)
video-uploader upload --file video.mp4 --title "My Video"

# Upload with JSON output (for scripts/CI)
video-uploader --output json upload --file video.mp4 --title "My Video"

# Upload to a specific workspace (channel)
video-uploader -w gaming upload --file gameplay.mp4 --title "Let's Play"

# Use an upload profile
video-uploader -P gaming upload --file gameplay.mp4 --title "Stream"

# Use per-video metadata TOML
video-uploader upload --file video.mp4 --meta video.meta.toml

# Auto-discover: if video.mp4 has a video.meta.toml next to it, it's used automatically
video-uploader upload --file video.mp4 --title "Override Title"
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
let progress = Arc::new(StderrProgressListener::new());

let video = VideoUpload::new("/path/to/video.mp4", "My Video Title")
    .with_description("Video description")
    .with_tags(vec!["tag1".to_string(), "tag2".to_string()])
    .with_visibility(Visibility::Public);

let result = youtube.upload(&video, Some(progress.clone())).await?;
println!("Uploaded: {} (workspace: {})", result.url, result.workspace);
```

## Upload Profiles

Profiles are TOML files stored in `~/.config/video-uploader/profiles/`. They provide reusable defaults so you don't repeat the same flags every time.

```toml
# ~/.config/video-uploader/profiles/default.toml
visibility = "private"
made_for_kids = false
license = "youtube"
category = "22"
language = "en"
contains_synthetic_media = false
embeddable = true
public_stats_viewable = false

tags = ["rust", "programming"]
description_suffix = "\n\nUploaded with video-uploader"
```

```bash
# List profiles
video-uploader profile list

# Show profile details
video-uploader profile show default

# Delete a profile
video-uploader profile remove old-profile

# Use a profile when uploading
video-uploader -P gaming upload --file vid.mp4 --title "Stream"

# CLI flags override profile defaults
video-uploader -P gaming upload --file vid.mp4 --title "Stream" --visibility public
```

## Per-Video Metadata (`.meta.toml`)

For automation and AI workflows, write a `.meta.toml` file next to your video:

```toml
# video.meta.toml — AI-generated per-video metadata
title = "Let's Play Rust - Episode 1"
description = "Building a CLI tool from scratch in Rust."
tags = ["rust", "programming", "tutorial"]
category = "20"
visibility = "unlisted"
```

```bash
# Auto-discovered: video.meta.toml next to video.mp4
video-uploader upload --file video.mp4 --title "CLI Override"

# Explicit path
video-uploader upload --file video.mp4 --meta /path/to/custom.meta.toml
```

### Resolution Order (highest priority wins)

```
CLI flags  >  meta TOML  >  profile TOML  >  built-in defaults (private)
```

### AI Automation Loop

```bash
# 1. AI writes the metadata file
cat > video.meta.toml << 'EOF'
title = "Automated Upload"
description = "Generated by AI"
tags = ["automated"]
made_for_kids = false
EOF

# 2. Upload with JSON output for programmatic result parsing
video-uploader --output json upload --file video.mp4 --title "Automated Upload"
# → {"workspace":"youtube","video_id":"dQw4w9WgXcQ","url":"https://...","title":"Automated Upload"}
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

### Batch CSV with Workspaces & Profiles

```csv
file,title,workspace,profile,description,tags,visibility
gaming1.mp4,Gameplay 1,gaming,gaming,Let's play,rust|gaming,public
cooking1.mp4,Recipe,cooking,cooking,My recipe,food|cooking,unlisted
default.mp4,Vlog,,,,private
```

The `workspace` and `profile` columns are optional — rows without them use defaults.

## YouTube Setup

1. Go to [Google Cloud Console](https://console.cloud.google.com/apis/credentials)
2. Create an OAuth 2.0 Client ID (Desktop app type)
3. Enable the YouTube Data API v3
4. Run `video-uploader auth --client-id YOUR_ID --client-secret YOUR_SECRET`
5. Follow the on-screen instructions (open URL, enter device code)

**Note**: YouTube API quota is 10,000 units/day (~6 uploads/day per project).

## Architecture

```
video-uploader/               # Library crate
├── src/
│   ├── lib.rs                 # Public exports
│   ├── youtube.rs             # YouTubeUploader (resumable upload, token refresh, delete)
│   ├── error.rs               # UploadError enum
│   ├── config.rs              # Encrypted CredentialStore with workspaces
│   ├── upload.rs              # VideoUpload, UploadResult, Visibility, License
│   ├── profile.rs             # UploadProfile, VideoMeta (TOML-based presets + per-video metadata)
│   ├── validation.rs          # File validation (size, extension, title)
│   ├── progress.rs            # ProgressListener trait + StderrProgressListener
│   ├── net.rs                 # HTTP client, retry logic, SSRF protection
│   ├── resume.rs              # UploadState (save/load/delete for resume)
│   └── auth/
│       ├── mod.rs              # Shared TokenResponse type
│       ├── device_code.rs     # Google OAuth2 device code flow
│       ├── auth_code.rs       # Browser-based authorization code flow
│       └── refresh_token.rs   # Token refresh logic

video-uploader-cli/            # Binary crate (run-and-exit CLI)
├── src/
│   ├── main.rs                # CLI with auth, upload, batch, list, workspace, profile subcommands
│   └── output.rs              # Pretty-print output (headers, key-value, icons)
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
