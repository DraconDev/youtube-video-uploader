## Goal
Work through the remaining TODO items for video-uploader, prioritized by impact.

## Checklist

### 🔥 High Priority
- [x] Per-video `.meta.toml` support
- [x] `profile show <name>`
- [x] `profile remove <name>`
- [x] `--output json`

### Medium Priority
- [x] Batch CSV `profile` column
- [x] `--version` flag
- [x] CI runs with `--features test-utils`

### Low Priority
- [x] Progress bar improvement — show ETA and upload speed
- [x] `--output json` for batch results
- [x] `recordingDate` field on VideoUpload + YouTube API
- [ ] Channel selection within a Google account (brand accounts)
- [ ] Batch CSV validation: warn about missing optional columns vs required ones
- [ ] Collect individual `UploadResult`s from batch tasks for full JSON output

### Cleanup
- [ ] `.env` tracking — managed by dracon-warden, leave as-is
- [x] Update `spec-profiles.md` — updated for TOML approach
- [x] Update `CHANGELOG.md` for v0.3
- [x] Update `README.md` with meta.toml and --output json docs
- [x] Version bump (0.3.x)

## Constraints
- Zero clippy warnings (`-D warnings`)
- All tests must pass (currently 174)
- No inline TODOs in source — track in `todo.md` only
- Default visibility = Private
- YouTube only
- Pretty-print output via `output.rs` module

## Key Files
- Library crate: `video-uploader/src/` (upload.rs, profile.rs, youtube.rs, lib.rs)
- CLI crate: `video-uploader-cli/src/` (main.rs, output.rs)
- Tests: `video-uploader/tests/`, `video-uploader-cli/tests/cli.rs`
- Spec: `spec-profiles.md`, `spec.md`, `todo.md`
