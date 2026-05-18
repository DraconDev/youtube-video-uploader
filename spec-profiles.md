# Upload Profiles + Per-Video Metadata — Design

## Problem
Every upload requires passing 8+ CLI flags for metadata that's mostly the same across videos
(made_for_kids, license, language, category, etc). This is tedious and error-prone, especially
for AI/automation workflows where shell escaping and long command lines are fragile.

## Solution

Two complementary TOML-based systems:

1. **Upload Profiles** — reusable defaults for a category of uploads (e.g. "gaming", "cooking")
2. **Per-Video Metadata** (`.meta.toml`) — per-video metadata for automation/AI workflows

### Storage

**Profiles**: `~/.config/video-uploader/profiles/<name>.toml`

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

**Per-Video Metadata**: `<video>.meta.toml` next to the video file

```toml
# video.meta.toml
title = "Let's Play Rust - Episode 1"
description = "Building a CLI tool from scratch in Rust."
tags = ["rust", "programming", "tutorial"]
category = "20"
visibility = "unlisted"
profile = "gaming"  # optional: specify which profile to use
```

### CLI Interface

```bash
# Profile management
video-uploader profile list
video-uploader profile show default
video-uploader profile remove old-profile

# Upload with profile
video-uploader -P gaming upload --file vid.mp4 --title "Stream"

# Upload with per-video metadata (explicit)
video-uploader upload --file video.mp4 --meta video.meta.toml --title "Override"

# Upload with auto-discovered metadata (video.meta.toml next to video.mp4)
video-uploader upload --file video.mp4 --title "Override"

# Machine-readable output for automation
video-uploader --output json upload --file video.mp4 --title "Auto Upload"
```

### Resolution Order (highest priority wins)

1. **CLI flag** (explicit `--visibility public`)
2. **Per-video meta TOML** (`video.meta.toml` visibility = "unlisted")
3. **Profile TOML** (profile sets visibility = "private")
4. **Built-in default** (visibility = private, category = 22, made_for_kids = false)

Tags are handled specially:
- Profile tags are **merged** with video tags (both sets kept, deduplicated)
- Meta TOML tags **replace** video tags (meta is the primary source)
- CLI `--tags` **replace** everything

### Batch CSV + Profiles

```csv
file,title,workspace,profile,description,tags,visibility
vid1.mp4,Video 1,gaming,gaming,,rust|gaming,public
vid2.mp4,Video 2,cooking,,My recipe,food|cooking,unlisted
vid3.mp4,Video 3,,,,,private
```

The `profile` and `workspace` columns are optional. Per-row profiles override the global `--profile` flag.

### Implemented Structs (Rust)

```rust
// Profile: reusable defaults
pub struct UploadProfile {
    pub visibility: Option<String>,
    pub made_for_kids: Option<bool>,
    pub license: Option<String>,
    pub category: Option<String>,
    pub language: Option<String>,
    pub contains_synthetic_media: Option<bool>,
    pub embeddable: Option<bool>,
    pub public_stats_viewable: Option<bool>,
    pub tags: Option<Vec<String>>,
    pub description_suffix: Option<String>,
    pub publish_at: Option<String>,
}

// Per-video metadata: primary source for this specific video
pub struct VideoMeta {
    pub title: Option<String>,
    pub description: Option<String>,
    pub tags: Option<Vec<String>>,
    pub visibility: Option<String>,
    pub category: Option<String>,
    pub made_for_kids: Option<bool>,
    pub license: Option<String>,
    pub language: Option<String>,
    pub contains_synthetic_media: Option<bool>,
    pub embeddable: Option<bool>,
    pub public_stats_viewable: Option<bool>,
    pub description_suffix: Option<String>,
    pub publish_at: Option<String>,
    pub profile: Option<String>,  // meta can specify which profile to use
}
```

### Status: ✅ Fully Implemented

All items from the original implementation plan are done:
- [x] `License` enum (youtube, creative-common)
- [x] All new fields on `VideoUpload`
- [x] `UploadProfile` struct with load/list/resolve/remove
- [x] `VideoMeta` struct with load_from/discover/apply_to
- [x] All fields in YouTube API metadata JSON
- [x] `profile` subcommand (list, show, remove)
- [x] `--profile` flag on upload and batch
- [x] `--meta` flag + auto-discover
- [x] Full resolution: CLI > meta TOML > profile TOML > built-in defaults
- [x] `profile` column in batch CSV
- [x] `--output json` for machine-readable results
- [x] `--version` flag
