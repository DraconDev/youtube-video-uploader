# Full Audit — video-uploader v0.4.4

**Date**: 2026-05-18  
**Status**: 175 tests pass, clippy clean, 0 inline TODOs

---

## ✅ Strengths

| Area | Status |
|------|--------|
| Clippy (`-D warnings`) | Clean — zero warnings |
| Test coverage | 175 tests across 9 test suites |
| Inline TODOs | Zero — all tracked in `todo.md` |
| Security | AES-256-GCM encryption, PBKDF2 100K, zeroize on drop, passphrase file support |
| API encapsulation | `pub(crate)` fields, getter/builder pattern |
| `#[non_exhaustive]` | On `Visibility`, `UploadResult`, `UploadError`, `License` |
| Automation pipeline | Profile TOML → meta TOML → CLI → JSON output — fully working |
| Channel info | Auto-fetched on auth, stored, displayed |
| Default visibility | Private — safety-first |
| CI | Tests, clippy, fmt, docs, audit |

---

## 🔴 Must Fix

### 1. `cargo deny` license check FAILS
Several RustCrypto crates (`aead`, `aes`, `aes-gcm`) don't declare SPDX expressions in their `Cargo.toml`, causing `cargo deny check licenses` to fail. These are all Apache-2.0 / MIT — legitimate.

**Fix**: Add `allow` list in `deny.toml`:
```toml
[licenses]
allow = ["MIT", "Apache-2.0", "BSD-2-Clause", "BSD-3-Clause", "CC0-1.0", "ISC", "Zlib", "Unicode-3.0"]
private = { ignore = true }
```

### 2. Missing rustdoc on `CredentialStore` and `PlatformCredentials` public API
18 public functions on `CredentialStore` + `PlatformCredentials` have no `///` doc comments. This is the most-used API surface.

**Fix**: Add `///` doc comments to all public fns in `config.rs`.

### 3. Missing rustdoc on `UploadError`, `net`, `validation` public items
`UploadError::is_retryable()`, `build_http_client()`, `build_http_client_with_timeout()`, `retry()`, `is_private_ip()`, `validate()` — no doc comments.

**Fix**: Add `///` doc comments.

---

## 🟡 Should Fix

### 4. `TESTING_PLAN.md` is stale
Says "118 tests" — we now have 175. Also says "Grade: A" with outdated breakdown.

**Fix**: Update test counts and breakdown.

### 5. `spec.md` references "Odysee, Rumble, PeerTube" in scope history
The spec mentions excluded platforms with rationale. This is fine historically, but the "In Scope" section says "YouTube only" which is correct. Minor: spec doesn't mention profiles, meta TOML, or channel info.

**Fix**: Update `spec.md` to document v0.3+ features (profiles, meta TOML, `--output json`, channel info).

### 6. Dead code in `output.rs`
`bullet()`, `numbered()`, `spacer()` are `#[allow(dead_code)]`. These are utility functions for future use.

**Fix**: Either remove them or use them somewhere (e.g., `spacer()` in headers).

### 7. Examples use `.with_description()` — inconsistent with builder rename
All builders were renamed from `.description()` to `.with_description()`. The `basic_upload.rs` example correctly uses `with_description`. ✅ No issue here — already updated.

### 8. `fuzz/` crate may be stale
The fuzz crate exists but isn't in the workspace and may not compile. Should verify.

**Fix**: Test `cd fuzz && cargo test` or remove if unused.

### 9. `--visibility` has no explicit `default_value` in clap
Changed from `default_value = "private"` to `Option<VisibilityArg>` so meta/profile defaults can apply. This is correct behavior, but `--help` no longer shows `[default: private]`. Users may be confused.

**Fix**: Add `help = "..."` text mentioning the default comes from profile/built-in.

---

## 🟢 Nice to Have

### 10. `cargo audit` not installed in CI security job
The CI workflow runs `cargo install cargo-audit` each time — slow. Pin a version or use a cached action.

### 11. Release workflow doesn't test ARM binary
ARM64 binary is built via `cross` but smoke test is skipped. Add QEMU-based smoke test.

### 12. No `deny.toml` configuration file
Project lacks a `deny.toml` for `cargo-deny` configuration. Should add one with license allowlist and advisory settings.

### 13. Auth code flow not tested in CI
The auth code fallback path isn't covered by any automated test. Only device code flow is tested via wiremock.

### 14. `ProgressListener` is `Send + Sync` but `StderrProgressListener` uses `AtomicU64`
The `AtomicU64` in `StderrProgressListener` is never read (`last_uploaded` is stored but never used). It was intended for delta-based speed calculation but the current implementation uses absolute speed from `start`.

**Fix**: Remove `last_uploaded` field — it's dead code.

---

## File Size Health Check

| File | Lines | Assessment |
|------|-------|------------|
| `main.rs` | 916 | ⚠️ Large — consider splitting CLI into submodules |
| `wiremock.rs` | 905 | OK — integration test file |
| `config.rs` | 782 | OK — mostly tests |
| `youtube.rs` | 716 | OK — core uploader |
| `integration.rs` | 525 | OK |
| `profile.rs` | 492 | OK |

`main.rs` at 916 lines is the biggest concern. A future refactor could extract:
- `auth_cmd()` handler
- `upload_cmd()` handler  
- `batch_cmd()` handler
- `workspace_cmd()` handler
- `channel_cmd()` handler

---

## Summary

| Category | Count | Items |
|----------|-------|-------|
| 🔴 Must Fix | 3 | `deny.toml`, rustdoc on config/net/error/validation |
| 🟡 Should Fix | 6 | TESTING_PLAN, spec.md, dead code, fuzz crate, help text, stale AtomicU64 |
| 🟢 Nice to Have | 5 | CI audit caching, ARM smoke test, auth code tests, deny.toml, main.rs split |
| ✅ Already Good | — | Clippy, tests, no inline TODOs, security, API design |
