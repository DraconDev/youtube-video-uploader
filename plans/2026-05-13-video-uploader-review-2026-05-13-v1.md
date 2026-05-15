# video-uploader: Comprehensive Review & Next Phase Plan

**Date:** 2026-05-13
**Status:** All previous tasks completed. Project is healthy: 71 tests pass, clippy clean.

---

## Current Project Health

| Metric | Value |
|--------|-------|
| Total tests | 71 (49 unit + 16 integration + 6 wiremock) |
| Compiler warnings | 0 |
| Clippy warnings | 0 |
| Security | PBKDF2+salt encryption, 0o600 perms, sync_all, 30s timeouts |

---

## Architecture Assessment

### Strengths

1. **Clean trait-based abstraction** — `PlatformUploader` trait with `platform_name()`, `upload()`, `supports_resumable()` is well-designed
2. **UploaderRegistry with Semaphore** — Concurrency-limited concurrent uploads, builder pattern for configuration
3. **Async file_size()** — `tokio::fs::metadata` doesn't block the async runtime
4. **MIME type detection** — `mime_guess` used for `X-Upload-Content-Type` and multipart Content-Type
5. **PBKDF2 + salt encryption** — Credential store uses 100k iterations, random 16-byte salt, backward-compatible v1 format
6. **Retry with exponential backoff** — Both platforms retry on 5xx/429 with 2^n delays
7. **YouTube category warning** — `tracing::warn!` fires when defaulting to "22"
8. **Validation at registry layer** — Individual uploaders stay testable with fake paths
9. **CLI feature-complete** — auth, upload, list, batch commands with CSV manifest, tilde expansion, dry-run, EnvFilter

### Issues Found

| # | Issue | Severity | Location |
|---|-------|----------|----------|
| 1 | `reqwest::Client::builder().build()?` can fail in `device_code.rs:42` and `refresh_token.rs:21` — propagates as `UploadError::Http` which is misleading | Low | `device_code.rs:40-42`, `refresh_token.rs:19-21` |
| 2 | `UploaderRegistry::builder()` returns default builder but `max_concurrency` defaults to 0 (via `Default`) — `Semaphore::new(0)` would deadlock | **High** | `registry.rs:11-12` |
| 3 | `upload.rs:4` defines `SUPPORTED_EXTENSIONS` but `validation.rs:9` defines `VALID_EXTENSIONS` — two separate lists that can drift | Medium | `upload.rs:4`, `validation.rs:9` |
| 4 | `youtube.rs:with_base_url` ignores the `base_url` parameter — dead code for testing | Medium | `youtube.rs:48-62` |
| 5 | `apivideo.rs:upload_simple` — `mime_str()` failure creates a broken Part (falls back to `Part::bytes(Vec::new())`) | Medium | `apivideo.rs:107-109` |
| 6 | CLI `Upload` handler doesn't use `UploaderRegistry` — manual per-platform dispatch instead of the recommended registry | Low | `main.rs:262-296` |
| 7 | `temp_file_with_ext` in validation tests leaks temp files — `NamedTempFile` dropped without deletion | Low | `validation.rs:105-115` |
| 8 | No `rustfmt.toml` or CI configuration | Low | Project root |

---

## Recommended Next Phase: Priority Actions

### High Priority

- [ ] Fix `UploaderRegistryBuilder::default()` — `max_concurrency` defaults to 0 which creates a `Semaphore::new(0)` causing deadlock. Change to `fn default() -> Self { Self { max_concurrency: 4 } }`
- [ ] Deduplicate extension list — remove `SUPPORTED_EXTENSIONS` from `upload.rs`, keep `VALID_EXTENSIONS` in `validation.rs` as the single source of truth
- [ ] Fix `ApiVideoUploader::upload_simple` mime_str error handling — use `.mime_str()` properly or fall back to the mime type string directly
- [ ] Fix `YouTubeUploader::with_base_url` to actually use the `base_url` parameter for wiremock testing, or remove it

### Medium Priority

- [ ] Refactor CLI Upload handler to use `UploaderRegistry::upload_to_all()` instead of manual per-platform dispatch
- [ ] Add `rustfmt.toml` with project formatting standards
- [ ] Add GitHub Actions CI: test + clippy + fmt check on PR
- [ ] Clean up leaked temp files in validation tests

### Low Priority / Future

- [ ] Add `zeroize` for sensitive in-memory credential fields
- [ ] Add `ffprobe` integration for format validation
- [ ] Add cancel tokens for in-progress uploads
- [ ] Persist YouTube resumable upload URLs for crash recovery
