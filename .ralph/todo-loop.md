## Goal
Work through the remaining TODO items for video-uploader, prioritized by impact.

## Checklist

### 🔥 High Priority
- [x] Per-video `.meta.toml` support: `--meta <path>` flag, auto-discover (`video.meta.toml` next to `video.mp4`), Meta TOML fields map to `VideoUpload`, resolution: CLI flag > meta TOML > profile TOML > built-in default
- [x] `profile show <name>` — display full profile contents
- [x] `profile remove <name>` — delete a profile file
- [x] `--output json` flag — machine-readable JSON output for upload results (for automation pipelines)

### Medium Priority
- [x] Batch CSV `profile` column — per-row profile selection in batch uploads
- [x] `--version` flag — print version number
- [x] CI runs with `--features test-utils` — was already fixed

### Low Priority
- [ ] Progress bar improvement — show ETA and upload speed
- [ ] `--output json` for batch results (currently only single upload)
- [ ] `recordingDate` field on VideoUpload + YouTube API
- [ ] Channel selection within a Google account (brand accounts)
- [ ] Batch CSV validation: warn about missing optional columns vs required ones

### Cleanup
- [ ] Remove `.env` from tracking (contains OAuth2 secrets)
- [ ] Update `spec-profiles.md` to reflect TOML-based approach (spec describes CLI commands)
- [ ] Update `CHANGELOG.md` for v0.3
- [ ] Update `README.md` with meta.toml and --output json docs
- [ ] Version bump to 0.3.0 (many new features since 0.2)

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
