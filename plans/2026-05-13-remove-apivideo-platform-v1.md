# Remove api.video Support

## Objective

Remove api.video platform from the video-uploader codebase since it's a paid centralized service that doesn't align with the project's goal of competing with YouTube through free/decentralized alternatives.

## Implementation Plan

- [ ] Delete `video-uploader/src/platforms/apivideo.rs`
- [ ] Remove `pub mod apivideo;` from `platforms/mod.rs`
- [ ] Remove `apivideo::ApiVideoUploader` from `lib.rs` exports
- [ ] Remove `ApiVideoUploader` import from `registry.rs`
- [ ] Remove `"api.video"` arm from `registry.rs::get_uploader()`
- [ ] Remove `Apivideo` variant from CLI `PlatformArg` enum
- [ ] Remove `PlatformArg::Apivideo` display impl
- [ ] Remove api.video auth handler from `Commands::Auth`
- [ ] Remove api.video upload arm from `Commands::Upload`
- [ ] Remove api.video from `README.md` platform table
- [ ] Remove api.video setup instructions from README
- [ ] Remove api.video wiremock tests
- [ ] Remove api.video from integration tests
- [ ] Run `cargo build` to verify compilation
- [ ] Run `cargo test` to verify all tests pass
- [ ] Run `cargo clippy` to verify no warnings

## Verification Criteria

- `cargo build` completes without errors
- All tests pass (expected: fewer tests, but no failures)
- `cargo clippy` reports no warnings
- README only documents YouTube, PeerTube, and Odysee

## Supported Platforms After Removal

| Platform | Auth Method | Notes |
|----------|-------------|-------|
| YouTube | OAuth2 | Free, large reach |
| PeerTube | API token | Federated, decentralized |
| Odysee | LBRY SDK | Crypto/blockchain |

## Files to Modify

1. `video-uploader/src/platforms/apivideo.rs` (delete)
2. `video-uploader/src/platforms/mod.rs`
3. `video-uploader/src/lib.rs`
4. `video-uploader/src/registry.rs`
5. `video-uploader-cli/src/main.rs`
6. `README.md`
7. `video-uploader/tests/wiremock.rs`
8. `video-uploader/tests/integration.rs`