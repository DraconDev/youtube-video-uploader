# Remove api.video Support

## Objective

Remove api.video platform from video-uploader to simplify the codebase and focus on decentralized alternatives.

## Implementation Plan

- [ ] Remove ApiVideoUploader from platforms/mod.rs exports
- [ ] Delete video-uploader/src/platforms/apivideo.rs
- [ ] Remove ApiVideoUploader from lib.rs exports
- [ ] Remove api.video from registry.rs get_uploader()
- [ ] Remove Apivideo variant from CLI PlatformArg enum
- [ ] Remove api.video auth handler from Commands::Auth
- [ ] Remove api.video upload arm from Commands::Upload
- [ ] Remove api.video from README platform table
- [ ] Remove api.video setup instructions from README
- [ ] Remove api.video wiremock tests
- [ ] Remove api.video from integration tests
- [ ] Run tests to verify everything works

## Verification Criteria

- `cargo build` succeeds
- `cargo test` passes with no failures
- `cargo clippy` shows no warnings
- README only lists YouTube, PeerTube, Odysee as supported platforms