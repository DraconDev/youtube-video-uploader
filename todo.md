# TODO

## ✅ Done

### v0.1 — Initial Release
- [x] Replace `atty` with `std::io::IsTerminal`
- [x] HTTP client builder panics on failure
- [x] Cache `file_size()` — computed once in `upload()`
- [x] Add `#[non_exhaustive]` to `Visibility`, `UploadResult`, `UploadError`
- [x] Add `UploadResult::new()` constructor
- [x] Replace `UploadError::Other(String)` with typed `UploadError::NoAttempts`
- [x] CI: `Swatinem/rust-cache@v2`, `cargo audit --deny warnings`
- [x] Release: smoke-test binary before packing
- [x] Update CHANGELOG with `[0.1.2]` section
- [x] Remove all inline TODOs — consolidated to this file

### v0.2 — Multi-Channel Workspaces
- [x] Full multi-channel workspace support (`workspaces: HashMap` + `default_workspace`)
- [x] `UploadResult.platform` → `workspace`, `platform_id` → `video_id`
- [x] Fix `upload_chunks()` double `stat()` — `total_size` threaded through
- [x] `tracing::instrument` fields: `platform` → `workspace`
- [x] Rename integration tests: `platforms` → `workspaces`
- [x] Remove 4 duplicate tests between `config.rs` and `integration.rs`
- [x] CLI batch CSV `workspace` column
- [x] Remove stale `plans/` directory
- [x] Simplify `CredentialStore::zeroize()`
- [x] All clippy warnings fixed (`-D warnings` passes clean)
- [x] Version bumped to 0.2.0, CHANGELOG updated
- [x] CredentialStore Zeroize + Drop implemented
- [x] PlatformCredentials Zeroize + Drop implemented
- [x] Passphrase wrapped in `Zeroizing<String>` in `YouTubeUploader`
- [x] Token refresh reuses HTTP client — `refresh_access_token()` accepts `&reqwest::Client`
- [x] Added `refresh_access_token_standalone()` for device code flow
- [x] Added rustdoc with examples to all public types
- [x] CLI `workspace` subcommand tests
- [x] CSV pre-validation in `batch`
- [x] All sensitive fields in `PlatformCredentials` wrapped with `Zeroizing<String>`
- [x] Added `PlatformCredentials::new()` constructor
- [x] Re-exported `Zeroizing` from library crate
- [x] CLI passphrase zeroization — `get_passphrase()` returns `Zeroizing<String>`
- [x] Upload resume support — `UploadState` with save/load/delete, `YouTubeUploader::resume()`, `extract_resume_state()`
- [x] `VideoUpload` field encapsulation — fields `pub(crate)`, getter methods, builder methods renamed to `with_*`
- [x] README updated for v0.2
- [x] `examples/` directory: `basic_upload.rs`, `batch_upload.rs`, `custom_progress.rs`, `multi_channel.rs`
- [x] Release targets: `aarch64-unknown-linux-gnu` (via cross), `x86_64-apple-darwin` (macOS-13 runner)
- [x] E2E wiremock upload flow test
- [x] Batch upload test with workspace markers

### v0.2.1 — Profiles + Pretty Output + Metadata
- [x] Upload profiles system — `UploadProfile` struct with TOML storage in `~/.config/video-uploader/profiles/`
- [x] Profile resolution: CLI flag > profile > built-in default (private)
- [x] `--profile` / `-P` flag on upload and batch
- [x] `profile` subcommand to list available profiles
- [x] `VideoUpload.apply_profile()` merges profile defaults
- [x] Profile tags merge (profile tags + video tags, not replace)
- [x] New metadata fields on `VideoUpload`: `license`, `language`, `contains_synthetic_media`, `embeddable`, `public_stats_viewable`, `description_suffix`, `publish_at`
- [x] `License` enum with `FromStr`/`Display` (youtube, creative-common)
- [x] All new fields sent to YouTube API in `status` and `snippet` objects
- [x] CLI flags for all new fields: `--license`, `--language`, `--contains-synthetic-media`, `--embeddable`, `--public-stats-viewable`, `--publish-at`, `--description-suffix`
- [x] Pretty-print output module (`output.rs`) — consistent formatting with headers, key-value layout, success/warn/error icons
- [x] Descriptive subcommand help text (auth, upload, list, batch, workspace, profile)
- [x] Auth code flow as fallback (device code fails → browser-based auth)
- [x] Shared `TokenResponse` type in `auth/mod.rs`
- [x] Removed PKCE from device code flow (Google TV clients reject it)
- [x] Retry for transient token errors (`internal_failure`/`server_error` → 2s backoff)
- [x] Default visibility = Private (safety-first)
- [x] `--made-for-kids` CLI flag
- [x] `.env` file with `dotenvy` for OAuth2 client credentials
- [x] Real YouTube auth + upload verified working
- [x] CLI tests updated for stderr-based pretty output (168 tests pass)

---

## 🔲 Remaining

### Per-video metadata TOML files (AI/automation ready)
- [ ] `--meta <path>` flag to load per-video metadata from a TOML file
- [ ] Auto-discover: if `video.mp4` has a `video.meta.toml` next to it, use it automatically
- [ ] Meta TOML fields: `title`, `description`, `tags`, `category`, `publish_at`, and any `VideoUpload` field
- [ ] Resolution: CLI flag > meta TOML > profile TOML > built-in default
- [ ] Makes AI automation trivial: write `.meta.toml`, run `video-uploader upload --file video.mp4`

### Profile enhancements
- [ ] `profile show <name>` — display full profile contents
- [ ] `profile remove <name>` — delete a profile
- [ ] `profile` column in batch CSV — per-row profile selection

### Additional YouTube metadata
- [ ] `recordingDate` field (ISO 8601 date of recording)
- [ ] `YOUTUBE_API_PART` update to include `status` + `recordingDetails` when those fields are used
- [ ] Channel selection within a Google account (brand accounts under same login)

### Polish
- [ ] `--version` flag output (currently no version command)
- [ ] Progress bar improvement — show ETA and upload speed
- [ ] Batch CSV validation: warn about missing optional columns vs required ones
- [ ] Upload result JSON output mode (`--output json`) for machine-readable results
- [ ] GitHub Actions: test with `--features test-utils` in CI (currently missing)

---

## Current Status

- **168 tests**, 0 failures, clippy clean (`-D warnings`)
- **Version 0.2.1**, Edition 2024, Rust 1.82+
- **Real YouTube auth + upload verified** with TV-type OAuth2 client
