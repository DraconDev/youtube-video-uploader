## Rename `video-uploader` → `youtube-uploader` + MIT License

### Checklist
- [x] Rename crate `video-uploader` → `youtube-uploader` in all Cargo.toml files
- [x] Rename crate `video-uploader-cli` → `youtube-uploader-cli` in all Cargo.toml files
- [x] Rename fuzz crate `video-uploader-fuzz` → `youtube-uploader-fuzz`
- [x] Update all `use video_uploader::` → `use youtube_uploader::` in source, tests, examples
- [x] Update all `video-uploader` string references in docs, configs, CLI help text
- [x] Update config dir paths `~/.config/video-uploader/` → `~/.config/youtube-uploader/`
- [x] Update env var `VIDEO_UPLOADER_PASSPHRASE` → `YOUTUBE_UPLOADER_PASSPHRASE`
- [x] Change license from AGPL-3.0-only to MIT (LICENSE file, Cargo.toml, deny.toml)
- [x] Remove COMMERCIAL-LICENSE.md (no longer needed)
- [x] Remove CLA.md (not needed for MIT)
- [x] Update README.md, GUIDE.md, CHANGELOG.md
- [x] Update CI workflows if they reference old names
- [x] 213 tests pass, clippy clean, deny clean, fmt clean

### Verified
- 213 tests, 0 failures
- clippy clean (`-D warnings`)
- `cargo deny` clean (advisories ok, bans ok, licenses ok, sources ok)
- `cargo fmt` clean
- Real upload verified: `MUc1fBpg9wY` uploaded as private via `youtube-uploader` CLI
