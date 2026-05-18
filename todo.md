# TODO

## ✅ Done

### v0.1 — Initial Release
- [x] Core upload, auth, credential storage, batch, CI

### v0.2 — Multi-Channel Workspaces
- [x] Full workspace support, zeroization, resume, builder pattern, examples

### v0.3 — Profiles + Pretty Output + Metadata + Automation
- [x] Upload profiles system — `UploadProfile` struct with TOML storage
- [x] `profile` subcommand (list, show, remove)
- [x] `--profile` / `-P` flag on upload and batch
- [x] `VideoUpload.apply_profile()` merges profile defaults
- [x] Profile tags merge (profile tags + video tags, not replace)
- [x] New metadata fields: `license`, `language`, `contains_synthetic_media`, `embeddable`, `public_stats_viewable`, `description_suffix`, `publish_at`, `recording_date`
- [x] `License` enum with `FromStr`/`Display`
- [x] All new fields sent to YouTube API (`status`, `snippet`, `recordingDetails`)
- [x] CLI flags for all new fields including `--recording-date`
- [x] Per-video `.meta.toml` — `--meta` flag, auto-discover, `VideoMeta` struct
- [x] Meta can specify a `profile` name
- [x] Full resolution: CLI flag > meta TOML > profile TOML > built-in default
- [x] `--output json` — machine-readable JSON for single upload + batch (with individual results)
- [x] `--version` flag
- [x] Pretty-print output (`output.rs`) — boxed headers, key-value layout, icons
- [x] Progress bar with upload speed + ETA + duration on completion
- [x] `StderrProgressListener` tracks timing (start instant, speed calc)
- [x] Descriptive subcommand help text
- [x] Batch CSV `profile` column — per-row profile selection
- [x] Batch supports meta TOML + profile resolution (same as single upload)
- [x] Batch visibility default = Private (was incorrectly Public)
- [x] Batch CSV validation: warn about missing optional columns (description, tags, visibility, workspace, profile)
- [x] Batch JSON output includes individual `results` array with per-video UploadResult
- [x] Auth code flow as fallback, shared `TokenResponse`, retry for transient errors
- [x] Default visibility = Private everywhere
- [x] `dotenvy` for `.env` OAuth2 credentials
- [x] Real YouTube auth + upload verified working
- [x] CHANGELOG, README, spec-profiles.md all updated for v0.3

---

## 🔲 Remaining

- [ ] Channel selection within a Google account (brand accounts under same login)
  - Requires YouTube API `channels.list?mine=true` + interactive picker
  - Significant new API surface; deferred to v0.4

---

## Current Status

- **174 tests**, 0 failures, clippy clean (`-D warnings`)
- **Version 0.3.x**, Edition 2024, Rust 1.82+
- **Full automation pipeline**: profile TOML → meta TOML → CLI → JSON output
