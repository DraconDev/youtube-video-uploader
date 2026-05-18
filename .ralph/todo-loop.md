## Goal
Work through the remaining TODO items for video-uploader, prioritized by impact.

## Checklist

### 🔥 High Priority
- [ ] Per-video `.meta.toml` support: `--meta <path>` flag, auto-discover (`video.meta.toml` next to `video.mp4`), Meta TOML fields map to `VideoUpload`, resolution: CLI flag > meta TOML > profile TOML > built-in default
- [ ] `profile show <name>` — display full profile contents
- [ ] `profile remove <name>` — delete a profile file
- [ ] `--output json` flag — machine-readable JSON output for upload results (for automation pipelines)

### Medium Priority
- [ ] Batch CSV `profile` column — per-row profile selection in batch uploads
- [ ] `--version` flag — print version number
- [ ] CI runs with `--features test-utils` — currently missing from GitHub Actions

### Low Priority
- [ ] Progress bar improvement — show ETA and upload speed
- [ ] `recordingDate` field on VideoUpload + YouTube API
- [ ] Channel selection within a Google account (brand accounts)

## Constraints
- Zero clippy warnings (`-D warnings`)
- All tests must pass (currently 168)
- No inline TODOs in source — track in `todo.md` only
- Default visibility = Private
- YouTube only
- Pretty-print output via `output.rs` module

## Key Files
- Library crate: `video-uploader/src/` (upload.rs, profile.rs, youtube.rs, lib.rs)
- CLI crate: `video-uploader-cli/src/` (main.rs, output.rs)
- Tests: `video-uploader/tests/`, `video-uploader-cli/tests/cli.rs`
- Spec: `spec-profiles.md`, `spec.md`, `todo.md`
