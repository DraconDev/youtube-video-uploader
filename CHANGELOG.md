# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

## [0.3.1] - 2026-05-18

### Added
- **Upload profiles**: TOML-based presets in `~/.config/youtube-uploader/profiles/<name>.toml`
- **Per-video metadata TOML**: `--meta <path>` flag + auto-discover `video.meta.toml` next to video file
- **`profile` subcommand**: `profile list`, `profile show <name>`, `profile remove <name>`
- **`--output json`**: Machine-readable JSON output for upload results (automation/CI)
- **`--version`**: Print version number
- **`--profile` / `-P`**: Select upload profile globally
- **`--meta`**: Explicit per-video metadata TOML path
- **New metadata fields**: `license`, `language`, `contains_synthetic_media`, `embeddable`, `public_stats_viewable`, `description_suffix`, `publish_at`
- **CLI flags for all new fields**: `--license`, `--language`, `--contains-synthetic-media`, `--embeddable`, `--public-stats-viewable`, `--publish-at`, `--description-suffix`
- **`License` enum**: `youtube` / `creative-common` with `FromStr`/`Display`
- **Pretty-print output**: Boxed headers, key-value layout, ✔/✘/⚠ icons via `output.rs` module
- **Descriptive subcommand help**: All commands have clear descriptions
- **Auth code flow**: Browser-based fallback when device code fails with `invalid_client`
- **Batch CSV `profile` column**: Per-row profile selection in batch uploads
- **Batch meta TOML resolution**: Batch uploads support auto-discovered `.meta.toml` files
- **`dotenvy` integration**: `.env` file for OAuth2 client credentials

### Changed
- **Resolution order**: CLI flags > meta TOML > profile TOML > built-in defaults
- **Default visibility = Private everywhere**: Batch CSV now defaults to `private` (was incorrectly `public`)
- **`--visibility` is now optional**: No default_value in CLI; defaults come from profile/built-in
- **Profile `list` simplified**: Shows names only; use `profile show <name>` for details
- **`UploadResult` now serializable**: Supports `--output json` with `serde::Serialize`
- **`VideoUpload.with_title()`**: Allows meta/profile to override title before CLI
- **Shared `TokenResponse` type**: Unified between device code and auth code flows
- **Removed PKCE from device code flow**: Google TV/Limited Input clients reject `code_verifier`

### Fixed
- Batch CSV visibility default was `public` — now `private` (safety-first)
- CLI tests updated for stderr-based pretty output format

## [0.2.0] - 2026-05-17

### Added
- Multi-channel workspace support: manage and upload to multiple YouTube accounts from one machine
- `--workspace` / `-w` global flag to target a specific workspace
- `workspace` subcommand: `workspace default <name>`, `workspace rename <old> <new>`, `workspace remove <name>`
- Optional `workspace` column in batch CSV manifests for per-video workspace targeting
- `CredentialStore::default_workspace()`, `set_default()`, `clear_default()`, `workspaces()` (replaces `platforms()`)

### Changed
- `CredentialStore` TOML format: `[workspaces.youtube]` sections with top-level `default_workspace` key
- Auto-migration: v0.1 flat format (`[youtube]`) is automatically upgraded on first load
- `YouTubeUploader::new()` now requires a workspace name parameter
- `list` command now shows workspace names with `(default)` marker
- `UploadResult` fields renamed: `platform` → `workspace`, `platform_id` → `video_id`
- `upload_chunks()` and `upload_with_retry()` now accept `total_size` parameter (eliminates redundant `stat()`)
- `tracing::instrument` spans now use dynamic `workspace` field instead of hardcoded `platform = "youtube"`
- All clippy warnings fixed across codebase (`cargo clippy --all-targets --all-features -- -D warnings` passes clean)

### Removed
- `CredentialStore::platforms()` (replaced by `workspaces()`)
- 4 duplicate tests between `config.rs` unit tests and `integration.rs`
- Stale `plans/` directory
- `CredentialStore::platforms()` removed (replaced by `workspaces()`)

## [0.1.2] - 2026-05-17

### Changed
- Replaced `atty` (unmaintained) with `std::io::IsTerminal` for TTY detection
- HTTP client builder now panics on failure instead of silently falling back to a no-timeout client
- `file_size()` is now computed once per upload and cached, eliminating a redundant `stat()` call
- Added `#[non_exhaustive]` to `Visibility`, `UploadResult`, and `UploadError` to prevent semver breaks
- Replaced `UploadError::Other(String)` catch-all with typed `UploadError::NoAttempts` variant
- CI now uses `Swatinem/rust-cache@v2` for build caching
- CI `cargo audit` now runs with `--deny warnings`
- Release workflow now smoke-tests the binary (`youtube-uploader --help`) before packing

### Removed
- **Odysee/LBRY platform support** — LBRY SDK archived since Jan 2023, no public upload API, lbrynet daemon impractical
- **PeerTube platform support** — removed in prior release (low demand, heavy Docker dependency)
- `daemon_url` field from `PlatformCredentials` (only used by Odysee)
- `OdyseeUploader`, `validate_daemon_url` public re-exports
- `odysee_default_daemon_url` from `auth::urls`
- `ODYSEE_MAX_SIZE` constant from `validation`
- `atty` dependency (replaced with std library)

### Fixed
- `ProgressListener::on_error` is now called on upload failure in YouTube uploader
- Removed spurious `#[allow(dead_code)]` attributes from uploader methods
- YouTube upload initiation URL (was appending /videos to upload endpoint)
- YouTube delete URL (was missing /videos path component)
- YouTube upload scope (changed from `youtube.upload` to `youtube` for delete support)

## [0.1.0] - 2024-01-01

### Added
- `youtube-uploader` library with `PlatformUploader` trait for multi-platform uploads
- YouTube uploader with OAuth2 device code flow and resumable uploads
- `UploaderRegistry` for concurrent multi-platform dispatch with configurable concurrency
- AES-GCM encrypted credential storage (V2 format with PBKDF2 key derivation)
- V1 → V2 credential migration on load
- `ProgressListener` trait with `StderrProgressListener` implementation
- `VideoUpload` builder API with visibility, tags, description, category support
- Platform-specific file size validation (YouTube: 128 GiB)
- `youtube-uploader` CLI with `auth`, `upload`, `batch`, and `list` commands
- CSV batch upload support
- Full async/await runtime using Tokio
- Comprehensive test suite with wiremock for HTTP mocking
- CI pipeline (tests, clippy, fmt, docs, audit)

[unreleased]: https://github.com/dracon/youtube-uploader/compare/v0.1.2...HEAD
[0.1.2]: https://github.com/dracon/youtube-uploader/compare/v0.1.0...v0.1.2
[0.1.0]: https://github.com/dracon/youtube-uploader/releases/tag/v0.1.0
