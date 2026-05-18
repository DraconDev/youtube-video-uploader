## Goal
Work through the remaining TODO items for video-uploader, prioritized by impact.

## Checklist

### 🔥 High Priority — ALL DONE
- [x] Per-video `.meta.toml` support
- [x] `profile show <name>`
- [x] `profile remove <name>`
- [x] `--output json`

### Medium Priority — ALL DONE
- [x] Batch CSV `profile` column
- [x] `--version` flag
- [x] CI runs with `--features test-utils`

### Low Priority
- [x] Progress bar improvement — show ETA and upload speed
- [x] `--output json` for batch results (with individual UploadResult array)
- [x] `recordingDate` field on VideoUpload + YouTube API
- [ ] Channel selection within a Google account (brand accounts) — complex, deferred
- [x] Batch CSV validation: warn about missing optional columns

### Cleanup — ALL DONE
- [x] `.env` tracking — managed by dracon-warden, leave as-is
- [x] Update `spec-profiles.md`
- [x] Update `CHANGELOG.md` for v0.3
- [x] Update `README.md`
- [x] Version bump (0.3.x)

## Constraints
- Zero clippy warnings (`-D warnings`)
- All tests must pass (currently 174)
- No inline TODOs in source — track in `todo.md` only
- Default visibility = Private
- YouTube only
