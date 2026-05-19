## Rename `video-uploader` → `youtube-uploader` + MIT License

### Checklist
- [ ] Rename crate `video-uploader` → `youtube-uploader` in all Cargo.toml files
- [ ] Rename crate `video-uploader-cli` → `youtube-uploader-cli` in all Cargo.toml files
- [ ] Rename fuzz crate `video-uploader-fuzz` → `youtube-uploader-fuzz`
- [ ] Update all `use video_uploader::` → `use youtube_uploader::` in source, tests, examples
- [ ] Update all `video-uploader` string references in docs, configs, CLI help text
- [ ] Update config dir paths `~/.config/video-uploader/` → `~/.config/youtube-uploader/`
- [ ] Update env var `VIDEO_UPLOADER_PASSPHRASE` → `YOUTUBE_UPLOADER_PASSPHRASE`
- [ ] Change license from AGPL-3.0-only to MIT (LICENSE file, Cargo.toml, deny.toml)
- [ ] Remove COMMERCIAL-LICENSE.md (no longer needed)
- [ ] Remove CLA.md (not needed for MIT)
- [ ] Update README.md, GUIDE.md, CHANGELOG.md
- [ ] Update CI workflows if they reference old names
- [ ] 213 tests pass, clippy clean, deny clean, fmt clean
