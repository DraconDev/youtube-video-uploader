# video-uploader: Next Phase Strategic Plan

## Objective

Advance the project from a working MVP to production-ready quality by addressing security hardening, reliability gaps, API polish, and platform extensibility. All work must preserve the existing 76-test suite and maintain zero-warning builds.

---

## Current State Assessment

### What Works Well
- Trait-based architecture (`PlatformUploader`) is clean and extensible
- AES-GCM encrypted credential store with `0o600` file permissions
- YouTube: true resumable chunked upload (8 MiB) with 308 resume support
- api.video: streaming upload for small files, 128 MiB progressive chunks for large files
- Retry logic with exponential backoff (3 retries, 2^n delays) on both platforms
- Progress reporting via `ProgressListener` trait
- Auto OAuth2 token refresh before upload
- 76 tests: 54 unit + 16 integration + 6 wiremock HTTP tests

### Known Gaps (from code review)
1. **No request timeouts on OAuth2 flows** — `device_code.rs` and `refresh_token.rs` use default `reqwest::Client` with no timeout
2. **`VideoUpload::file_size()` does blocking I/O** — called from async context in uploaders without `tokio::task::spawn_blocking`
3. **Credential store uses SHA-256 for key derivation** — sufficient but not ideal; no salt means same passphrase always produces same key
4. **No cancellation support** — in-progress uploads cannot be cancelled via `CancellationToken`
5. **YouTube `initiate_resumable` not covered by retry** — if the initial POST fails with 503, the upload fails permanently
6. **`upload_to_all` panics on join handle** — uses `.unwrap_or_else` which is fine but could propagate better
7. **No `Content-Type` detection for video files** — all uploads use `application/octet-stream`
8. **`PlatformCredentials` is a grab-bag struct** — all fields are `Option` regardless of platform needs
9. **CLI `--platforms` is comma-separated string** — not validated against configured platforms before upload starts
10. **No structured logging configuration** — CLI just calls `tracing_subscriber::fmt::init()` with no filter setup
11. **YouTube metadata `category_id` defaults silently to "22"** (People & Blogs) without telling the user
12. **`CredentialStore::save()` does not fsync** — data could be lost on crash
13. **No rate limiting or concurrency control** in `UploaderRegistry::upload_to_all`

---

## Implementation Plan

### Phase 1: Reliability & Async Hygiene

- [ ] Add `tokio::time::timeout` wrappers to all OAuth2 HTTP calls (device code start, token poll, refresh)
- [ ] Wrap `VideoUpload::file_size()` and `std::fs::metadata` calls in `tokio::task::spawn_blocking` to avoid blocking the async runtime
- [ ] Add retry logic around `YouTubeUploader::initiate_resumable` (currently only `upload_chunks` retries)
- [ ] Add `tokio::sync::CancellationToken` support to `PlatformUploader::upload` signature (or as a struct field) for cooperative cancellation

### Phase 2: Security Hardening

- [ ] Add a random salt to credential encryption: generate 16 random bytes, prepend to ciphertext, use in HKDF-SHA256 key derivation instead of raw SHA-256
- [ ] Add `zeroize` dependency and clear passphrase/key material from memory after use
- [ ] Call `file.sync_all()` after writing credentials in `CredentialStore::save`

### Phase 3: API Polish

- [ ] Replace silent `category_id` default with explicit enum `YouTubeCategory` or at minimum log the chosen category
- [ ] Add `VideoUpload::mime_type()` method using `mime_guess` or file extension mapping
- [ ] Pass detected MIME type through to YouTube `X-Upload-Content-Type` and api.video instead of hardcoded `application/octet-stream`
- [ ] Add `UploaderRegistry::upload_to_all_with_limit(max_concurrent: usize)` for concurrency control
- [ ] Replace `PlatformCredentials` enum-per-platform approach or add builder methods to ensure correct fields are populated

### Phase 4: Observability

- [ ] Configure `tracing_subscriber` with `EnvFilter` in CLI so `RUST_LOG=video_uploader=debug` works
- [ ] Add `tracing::instrument` spans to upload methods with platform/title fields
- [ ] Add structured progress: `ProgressListener` could include platform name in callbacks

### Phase 5: Testing & Coverage

- [ ] Add wiremock test for `refresh_access_token` failure → retry success path
- [ ] Add wiremock test for `initiate_resumable` 5xx retry
- [ ] Add wiremock test for api.video chunked upload (file > 128 MiB)
- [ ] Add integration test for `UploaderRegistry::upload_to_all` with mixed success/failure
- [ ] Add test for `validate` rejecting oversized files
- [ ] Add test for `validate` rejecting unsupported extensions

### Phase 6: Documentation

- [ ] Add rustdoc examples to `UploaderRegistry` methods
- [ ] Document the `PlatformUploader` trait with a complete example implementation
- [ ] Add CHANGELOG.md tracking breaking vs non-breaking changes
- [ ] Add CONTRIBUTING.md with platform addition checklist

---

## Verification Criteria

- [ ] `cargo test` passes with ≥ 90 tests
- [ ] `cargo clippy` produces zero warnings
- [ ] `cargo doc` builds without warnings
- [ ] New wiremock tests cover retry paths for `initiate_resumable`
- [ ] File validation tests cover all error variants
- [ ] No `unwrap()` or `expect()` added in production code (tests exempt)

---

## Potential Risks and Mitigations

1. **Credential format change breaks existing users**
   - Mitigation: Keep backward-compatible loader that detects old format (no salt) and re-encrypts on first save

2. **`spawn_blocking` for file_size adds complexity**
   - Mitigation: Benchmark first; if file metadata is truly fast, document why spawn_blocking is used

3. **MIME type detection adds dependency**
   - Mitigation: Use lightweight `mime_guess` crate (already commonly used with reqwest); fallback to octet-stream

4. **CancellationToken in trait is a breaking change**
   - Mitigation: Add as an optional field on uploader structs first, then consider trait change for 0.2

---

## Alternative Approaches

1. **For credential encryption**: Use `argon2` instead of HKDF for key derivation. Trade-off: slower, more secure, adds dependency.
2. **For progress reporting**: Use `tokio::sync::watch` channel instead of trait callbacks. Trade-off: more idiomatic for Rust async, but breaks existing `ProgressListener` implementations.
3. **For platform credentials**: Use a sealed trait per platform instead of `PlatformCredentials` struct. Trade-off: more type-safe, but significantly more boilerplate.

---

## Recommended Next Immediate Steps (Priority Order)

1. Fix async-blocking `file_size()` calls with `spawn_blocking`
2. Add request timeouts to OAuth2 flows
3. Add salt to credential encryption
4. Add `sync_all()` to credential save
5. Add `EnvFilter` to CLI tracing
