# TODO

## ✅ Done

### v0.1 — Initial Release
- [x] Core upload, auth, credential storage, batch, CI

### v0.2 — Multi-Channel Workspaces
- [x] Full workspace support, zeroization, resume, builder pattern, examples

### v0.3 — Profiles + Pretty Output + Metadata + Automation
- [x] Upload profiles, per-video meta TOML, `--output json`, pretty-print, all metadata fields
- [x] Progress bar with speed + ETA, `recording_date`, batch improvements
- [x] CHANGELOG, README, spec updated

### v0.4 — Channel Selection + Channel Info
- [x] `channel_id` + `channel_name` fields in `PlatformCredentials`
- [x] `YouTubeUploader::fetch_channel_info()` — calls `channels.list?mine=true`
- [x] Channel info auto-fetched after auth, stored in credentials
- [x] `channel` subcommand — show channel details for a workspace
- [x] `video-uploader list` shows channel names alongside workspaces
- [x] Auth success shows channel name and ID
- [x] CLI test for channel subcommand help (175 tests total)

---

## 🔲 Remaining

- [ ] Channel switching during auth (guiding user to switch Google account in browser before completing device code flow)
- [ ] `onBehalfOfContentOwner` / `onBehalfOfContentOwnerChannel` for MCN/partner accounts
- [ ] Batch CSV validation: warn about missing optional columns vs required ones

---

## Current Status

- **175 tests**, 0 failures, clippy clean (`-D warnings`)
- **Version 0.4.x**, Edition 2024, Rust 1.82+
- **Full automation pipeline**: profile TOML → meta TOML → CLI → JSON output
- **Channel info** stored and displayed per workspace
