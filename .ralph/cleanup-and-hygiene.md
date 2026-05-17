## Goal
Implement remaining items from `todo.md` — security hardening, API quality, upload resilience, testing, CLI polish, and documentation.

## Current State
- 166 tests pass, clippy clean with `-D warnings`
- v0.2 multi-channel workspace feature is complete
- All terminology cleanup done (platform → workspace)

## Phase 1: Security hardening
- [x] Access token zeroization in `PlatformCredentials` — all sensitive fields use `Zeroizing<String>` with serde support
- [x] Passphrase zeroization — `get_passphrase()` returns `Zeroizing<String>`, auto-zeroed on drop

## Phase 2: API quality
- [x] Token refresh: pass `&reqwest::Client` from `YouTubeUploader` instead of creating new client per call
- [x] Add rustdoc with examples to public types

## Phase 3: Upload resilience
- [x] Client-side upload resume — `UploadState` with save/load/delete, `YouTubeUploader::resume()`, `extract_resume_state()`

## Phase 4: Testing
- [x] End-to-end wiremock test: token refresh → chunk upload → result
- [x] Batch upload test with concurrency (3-entry CSV + workspace markers)
- [x] CLI `workspace` subcommand tests

## Phase 5: CLI polish
- [x] Pre-validate CSV rows in `batch`
- [x] `VideoUpload` field encapsulation — fields `pub(crate)`, getters, builder methods renamed to `with_*`

## Phase 6: Documentation
- [x] Update README for v0.2
- [x] Add `examples/` directory

## Phase 7: Release targets
- [x] Add `aarch64-unknown-linux-gnu` and `x86_64-apple-darwin` to release workflow

## Acceptance Criteria
- All 166+ tests continue to pass ✅
- Clippy clean with `-D warnings` ✅
- No new warnings in `cargo doc` ✅
- README reflects v0.2 features ✅
