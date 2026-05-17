# TODO

## ✅ Done (2026-05-17)

- [x] Replace `atty` with `std::io::IsTerminal`
- [x] HTTP client builder panics on failure
- [x] Cache `file_size()` — computed once in `upload()`
- [x] Add `#[non_exhaustive]` to `Visibility`, `UploadResult`, `UploadError`
- [x] Add `UploadResult::new()` constructor
- [x] Replace `UploadError::Other(String)` with typed `UploadError::NoAttempts`
- [x] CI: `Swatinem/rust-cache@v2`, `cargo audit --deny warnings`
- [x] Release: smoke-test binary before packing
- [x] Update CHANGELOG with `[0.1.2]` section
- [x] Remove all inline TODOs — consolidated to this file
- [x] Full v0.2 multi-channel workspace support
- [x] Rename `UploadResult.platform` → `workspace`, `platform_id` → `video_id`
- [x] Fix `upload_chunks()` double `stat()` — `total_size` threaded through
- [x] Fix `tracing::instrument` fields: `platform` → `workspace`
- [x] Rename integration tests: `platforms` → `workspaces`
- [x] Remove 4 duplicate tests between `config.rs` and `integration.rs`
- [x] Fix CLI batch CSV header: `platforms` → `workspace`
- [x] Remove stale `plans/` directory
- [x] Simplify `CredentialStore::zeroize()`
- [x] Make `initiate_resumable()` public
- [x] All clippy warnings fixed (`-D warnings` passes clean)
- [x] Version bumped to 0.2.0, CHANGELOG updated
- [x] CredentialStore Zeroize + Drop implemented
- [x] PlatformCredentials Zeroize + Drop implemented
- [x] Passphrase wrapped in `Zeroizing<String>` in `YouTubeUploader`
- [x] Token refresh reuses HTTP client — `refresh_access_token()` accepts `&reqwest::Client`
- [x] Added `refresh_access_token_standalone()` for device code flow
- [x] Added rustdoc with examples to all public types
- [x] CLI `workspace` subcommand tests
- [x] CSV pre-validation in `batch`
- [x] CLI batch CSV `workspace` column
- [x] All sensitive fields in `PlatformCredentials` wrapped with `Zeroizing<String>` (access_token, refresh_token, client_id, client_secret, api_key)
- [x] Added `PlatformCredentials::new()` constructor
- [x] Re-exported `Zeroizing` from library crate
- [x] CLI passphrase zeroization — `get_passphrase()` returns `Zeroizing<String>`
- [x] Upload resume support — `UploadState` with save/load/delete, `YouTubeUploader::resume()`, `extract_resume_state()`
- [x] `VideoUpload` field encapsulation — fields `pub(crate)`, getter methods, builder methods renamed to `with_*`
- [x] README updated for v0.2
- [x] `examples/` directory: `basic_upload.rs`, `batch_upload.rs`, `custom_progress.rs`, `multi_channel.rs`
- [x] Release targets: `aarch64-unknown-linux-gnu` (via cross), `x86_64-apple-darwin` (macOS-13 runner)
- [x] E2E wiremock upload flow test
- [x] Batch upload test with workspace markers

---

## No remaining items

All items from the original cleanup review have been implemented. The project is in good shape with:
- **166 tests**, 0 failures, clippy clean
- **v0.2.0** with multi-channel workspace support
- Zeroization of all secrets (credentials, passphrases, tokens)
- Upload resume capability
- Encapsulated public API with builder pattern
- Comprehensive rustdoc and examples
- Multi-platform release workflow
