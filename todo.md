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
- [x] Full multi-channel workspace support
- [x] `UploadResult.platform` → `workspace`, `platform_id` → `video_id`
- [x] Zeroization of all secrets
- [x] Upload resume support
- [x] `VideoUpload` field encapsulation + builder pattern
- [x] README, examples, wiremock tests, multi-platform release

### v0.3 — Profiles + Pretty Output + Metadata + Automation
- [x] Upload profiles system — `UploadProfile` struct with TOML storage
- [x] Profile resolution: CLI flag > profile > built-in default (private)
- [x] `--profile` / `-P` flag on upload and batch
- [x] `profile` subcommand (list, show, remove)
- [x] `VideoUpload.apply_profile()` merges profile defaults
- [x] Profile tags merge (profile tags + video tags, not replace)
- [x] New metadata fields: `license`, `language`, `contains_synthetic_media`, `embeddable`, `public_stats_viewable`, `description_suffix`, `publish_at`
- [x] `License` enum with `FromStr`/`Display`
- [x] All new fields sent to YouTube API in `status` and `snippet` objects
- [x] CLI flags for all new fields
- [x] **Per-video `.meta.toml`** support:
  - [x] `--meta <path>` flag to load per-video metadata from TOML
  - [x] Auto-discover: `video.meta.toml` next to `video.mp4`
  - [x] `VideoMeta` struct with `load_from()`, `discover()`, `apply_to()`
  - [x] Meta can specify a `profile` name
  - [x] Full resolution order: CLI flag > meta TOML > profile TOML > built-in default
- [x] **`--output json`** flag — machine-readable JSON output for automation
- [x] **`--version`** flag
- [x] **Pretty-print output** (`output.rs`) — boxed headers, key-value layout, ✔/✘/⚠ icons
- [x] **Descriptive subcommand help** text
- [x] **Batch CSV `profile` column** — per-row profile selection
- [x] **Batch visibility default = Private** (was incorrectly defaulting to Public)
- [x] **Batch supports meta TOML + profile resolution** (same as single upload)
- [x] Auth code flow as fallback
- [x] Shared `TokenResponse` type
- [x] Removed PKCE from device code flow
- [x] Retry for transient token errors
- [x] Default visibility = Private everywhere
- [x] `--made-for-kids` CLI flag
- [x] `.env` file with `dotenvy`
- [x] Real YouTube auth + upload verified working
- [x] CLI tests updated for stderr-based pretty output (174 tests pass)

---

## 🔲 Remaining

### Polish
- [ ] Progress bar improvement — show ETA and upload speed
- [ ] `--output json` for batch results (currently only single upload)
- [ ] Batch CSV validation: warn about missing optional columns vs required ones

### Additional YouTube metadata
- [ ] `recordingDate` field (ISO 8601 date of recording)
- [ ] `YOUTUBE_API_PART` update to include `status` + `recordingDetails` when those fields are used
- [ ] Channel selection within a Google account (brand accounts under same login)

---

## Current Status

- **174 tests**, 0 failures, clippy clean (`-D warnings`)
- **Version 0.3.1**, Edition 2024, Rust 1.82+
- **Real YouTube auth + upload verified** with TV-type OAuth2 client
- **Full automation pipeline**: profile TOML → meta TOML → CLI → JSON output
