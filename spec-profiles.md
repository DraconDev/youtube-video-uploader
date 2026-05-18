# Upload Profiles Design

## Problem
Every upload requires passing 8+ CLI flags for metadata that's mostly the same across videos
(made_for_kids, license, language, category, etc). This is tedious and error-prone.

## Solution: Upload Profiles

A profile is a named set of upload defaults stored in the config directory.

### Storage

`~/.config/video-uploader/profiles/<name>.toml`

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

# Optional per-profile tags (appended to video-specific tags)
tags = ["rust", "programming"]

# Optional default description suffix/appended text
description_suffix = "\n\nUploaded with video-uploader"
```

### CLI Interface

```bash
# Create/edit a profile
video-uploader profile set default \
  --visibility private \
  --made-for-kids false \
  --license youtube \
  --category 22 \
  --language en

video-uploader profile set gaming \
  --visibility unlisted \
  --made-for-kids false \
  --category 20 \
  --tags "gaming,letsplay"

# List profiles
video-uploader profile list

# Show a profile
video-uploader profile show default

# Delete a profile
video-uploader profile remove gaming

# Use a profile when uploading
video-uploader upload \
  --file vid.mp4 \
  --title "My Video" \
  --profile gaming

# Explicit flags override profile defaults
video-uploader upload \
  --file vid.mp4 \
  --title "My Video" \
  --profile gaming \
  --visibility public    # overrides gaming's "unlisted"
```

### Resolution Order (highest priority wins)

1. **CLI flag** (explicit `--visibility public`)
2. **Profile value** (profile sets `--visibility unlisted`)
3. **Built-in default** (visibility = private, category = 22)

### Batch CSV + Profiles

```csv
file,title,profile,visibility,tags
vid1.mp4,Video 1,gaming,,
vid2.mp4,Video 2,gaming,public,
vid3.mp4,Video 3,,,"rust,testing"
```

The `profile` column applies the profile defaults. Per-row flags still override.

### Profile struct (Rust)

```rust
pub struct UploadProfile {
    pub name: String,
    pub visibility: Option<Visibility>,
    pub made_for_kids: Option<bool>,
    pub license: Option<License>,
    pub category: Option<String>,
    pub language: Option<String>,
    pub contains_synthetic_media: Option<bool>,
    pub embeddable: Option<bool>,
    pub public_stats_viewable: Option<bool>,
    pub tags: Option<Vec<String>>,
    pub description_suffix: Option<String>,
    pub publish_at: Option<String>,  // ISO 8601 or "auto"
}
```

All fields `Option` — only non-None values override built-in defaults.

### Implementation Plan

1. Add `License` enum (youtube, creative-common) to `upload.rs`
2. Add missing fields to `VideoUpload` (license, language, contains_synthetic_media, embeddable, public_stats_viewable, publish_at, description_suffix)
3. Create `video-uploader/src/profile.rs` — `UploadProfile` struct, load/save/list
4. Update `youtube.rs` `initiate_resumable_with_url` to include all new fields in metadata JSON
5. Update `YOUTUBE_API_PART` to include `recordingDetails` if needed
6. Add `profile` subcommand to CLI
7. Add `--profile` flag to `upload` and `batch` commands
8. Add resolution logic: cli flag > profile > built-in default
9. Add `profile` column to batch CSV
