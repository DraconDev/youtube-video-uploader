# Testing Plan — video-uploader

## Overview

- **107 tests passing** (74 lib + 20 integration + 13 wiremock)
- **Grade: A-** — clippy clean aside from 12 auto-fixable style warnings
- **cargo audit: 0 known vulnerabilities**

---

## Current Coverage

### By Layer

| Layer | Files | What it covers |
|-------|-------|----------------|
| Unit tests | `src/**/*.rs` (inside `#[cfg(test)]` mods) | Core logic, validation, crypto, IP checks, claim name gen |
| Integration tests | `tests/integration.rs` | CredentialStore persistence, platform iterator, roundtrip |
| Wiremock tests | `tests/wiremock.rs` | HTTP mocking for YouTube/PeerTube/Odysee upload flows |

### By Platform

| Platform | Upload mocked | Auth mocked | Chunk/resume | Error handling |
|----------|--------------|-------------|--------------|----------------|
| YouTube | ✅ | ✅ | ✅ (308 + retry) | ✅ (4xx, 5xx) |
| PeerTube | ✅ | ✅ | — | ✅ (4xx, 403, 500) |
| Odysee | ✅ RPC | — | — | ✅ (daemon down, RPC error) |

---

## Gap Analysis

### Missing Tests

#### High Priority

1. **`net::is_private_ip` — missing edge cases**
   - IPv4-mapped: `::ffff:127.0.0.1` ✅ covered
   - IPv4-mapped public: `::ffff:8.8.8.8` ✅ covered
   - IPv6 unspecified `::` ✅ covered
   - **Gap**: IPv4-mapped loopback in CGNAT range (`::ffff:100.64.0.1`)
   - **Gap**: IPv4 in IPv6 with non-mapped prefix (e.g., `::192.168.1.1`)

2. **`config::derive_key_pbkdf2` — no direct unit test**
   - `test_credential_store_roundtrip` covers the happy path indirectly
   - **Gap**: No test for wrong passphrase → `UploadError::Encryption`
   - **Gap**: No test for corrupted ciphertext (too short, bad magic, invalid UTF-8 after decrypt)

3. **`validation::validate` — platform not validated**
   - **Gap**: What happens if platform = `"unknown"`? Gets `u64::MAX` size limit.
   - Should return `UploadError::Config` or warn

4. **YouTube URL validation SSRF — no test**
   - **Gap**: Test that `evilgoogleapis.com` is rejected
   - **Gap**: Test that `foo.googleapis.com` is accepted
   - **Gap**: Test that `googleapis.com` (apex) is accepted

#### Medium Priority

5. **`auth::device_code::poll_for_token` — token error response**
   - `test_token_error_response_deserialization` exists but no test for error *handling*
   - **Gap**: Test that a `slow_down` error returns `UploadError::Auth("Device code polling rate limited")`

6. **CredentialStore V1 → V2 migration**
   - `test_credential_store_roundtrip` uses V2 format directly
   - **Gap**: No test for loading a V1-format file (requires `FORMAT_VERSION_V1` and `derive_key_legacy`)
   - **Gap**: No test for V1 file with wrong passphrase

7. **`UploaderRegistry` concurrency**
   - `test_registry_upload_to_all_concurrent` exists
   - **Gap**: No test for partial failure — what happens if 2/3 platforms fail?

8. **PeerTube upload URL construction**
   - **Gap**: No test for trailing slash handling on instance URL
   - `test_upload_url_trailing_slash` exists but doesn't mock the HTTP call

9. **`Visibility` serialization edge cases**
   - **Gap**: `Visibility::Unlisted` roundtrip through JSON and TOML
   - `test_visibility_serde_public_roundtrip` and `test_visibility_serde_private_roundtrip` exist

10. **`VideoUpload` file path handling**
    - **Gap**: Unicode filename with non-ASCII characters
    - **Gap**: Relative vs absolute paths
    - `test_video_upload_builder_pattern` and `test_video_upload_minimal` exist

#### Low Priority

11. **`progress.rs` listener callbacks**
    - `StderrProgressListener` is exercised via integration tests but no isolated test
    - **Gap**: Test that `on_progress` fires correct byte counts
    - **Gap**: Test that `on_complete` fires with correct URL

12. **Odysee `visibility_to_bid`**
    - `test_visibility_to_bid` exists ✅

13. **`validate_daemon_url` for Odysee**
    - `test_validate_daemon_url` exists ✅

14. **`upload::VideoUpload::file_size` async edge cases**
    - `test_video_upload_file_size_async` exists ✅

15. **PKCE pair generation**
    - `test_generate_pkce_pair_produces_valid_pair` exists ✅

16. **`max_concurrency` getter**
    - `test_registry_builder_custom_max_concurrency` exists ✅

---

## Proposed Test Additions

### 1. SSRF URL Validation Tests (High)
**File**: `src/platforms/youtube.rs` — add to `#[cfg(test)]` mod

```rust
#[test]
fn test_upload_url_rejects_evil_googleapis_com() {
    let url = "https://evilgoogleapis.com/upload.googleapis.com/upload";
    assert!(validate_upload_url(url).is_err());
}

#[test]
fn test_upload_url_accepts_subdomain_googleapis_com() {
    let url = "https://foo.googleapis.com/upload.googleapis.com/upload";
    assert!(validate_upload_url(url).is_ok());
}

#[test]
fn test_upload_url_accepts_apex_googleapis_com() {
    let url = "https://googleapis.com/upload.googleapis.com/upload";
    assert!(validate_upload_url(url).is_ok());
}
```

### 2. Network Edge Case Tests (High)
**File**: `src/net.rs` — add to `#[cfg(test)]` mod

```rust
#[test]
fn test_ipv4_mapped_in_cgnat_range() {
    // ::ffff:100.64.0.1 — CGNAT IP as IPv4-mapped
    assert!(!is_private_ip("::ffff:100.64.0.1")); // NOT private (not a private CGNAT IP)
}

#[test]
fn test_ipv4_mapped_loopback() {
    assert!(is_private_ip("::ffff:127.0.0.1"));
    assert!(!is_private_ip("::ffff:127.0.0.2")); // 127.0.0.2 is also loopback so should be private
}
```

### 3. CredentialStore Error Paths (High)
**File**: `tests/integration.rs`

```rust
#[test]
fn test_credential_store_wrong_passphrase_returns_error() {
    // Already exists at line 276, verify it covers:
    // - V2 format with wrong passphrase
    // - V1 format with wrong passphrase
}

#[test]
fn test_credential_store_corrupted_file() {
    let temp_file = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(temp_file.path(), b"corrupted data too short");
    let result = CredentialStore::load_from_path("pass", temp_file.path());
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), UploadError::Encryption(_)));
}
```

### 4. Token Error Handling (Medium)
**File**: `tests/wiremock.rs` — add new test

```rust
#[tokio::test]
async fn test_poll_for_token_handles_slow_down_error() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/token"))
        .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({
            "error": "slow_down",
            "error_description": "Polling too fast"
        })))
        .mount(&mock_server)
        .await;

    // Patch GOOGLE_TOKEN_URL or use a test-specific helper
    let result = poll_for_token("device", "client", "secret", "verifier", 300, 5).await;
    assert!(result.is_err());
}
```

### 5. V1 Migration Test (Medium)
**File**: `tests/integration.rs`

```rust
#[test]
fn test_credential_store_v1_to_v2_migration() {
    // Write a V1-format encrypted file using derive_key_legacy
    // Load it with CredentialStore::load_from_path
    // Verify it migrates and re-saves as V2
}
```

### 6. Unknown Platform Validation (High)
**File**: `src/validation.rs` — add to `validate` function or tests

```rust
#[tokio::test]
async fn test_validate_unknown_platform_rejected() {
    let file = tempfile::NamedTempFile::new().unwrap();
    let path = file.path().with_extension("mp4");
    std::fs::write(&path, vec![0u8; 1024]).unwrap();
    let video = VideoUpload::new(&path, "Title");
    let result = validate(&video, "unknown_platform").await;
    assert!(result.is_err());
}
```

### 7. Registry Partial Failure (Medium)
**File**: `src/registry.rs` — add to `#[cfg(test)]` mod

```rust
#[tokio::test]
async fn test_registry_partial_platform_failure() {
    // Set up registry with YouTube + PeerTube creds
    // Mock YouTube to fail, PeerTube to succeed
    // Verify error is captured and returned properly
}
```

---

## Clippy Fixes (Auto-fixable)

```bash
# Fix all 12 clippy warnings in one command
cargo clippy --fix --all-targets --all-features --allow-dirty
```

Or manually:
```bash
# Remove dead code
sed -i '268d' video-uploader/tests/integration.rs  # hash_password_for_filename

# Fix expect(&format!(...)) → unwrap_or_else
# Lines 407, 411 in integration.rs

# Remove .into() on &str
# Line 492 in integration.rs

# Remove & from &mock_server.uri() calls
# Lines 192, 214, 235, 258, 316, 363, 390, 446 in wiremock.rs
```

---

## Testing Infrastructure Improvements

### 1. Add `cargo test --workspace` alias
Add to `Cargo.toml` workspace metadata or a `Justfile`:
```
just test          # run all tests
just clippy        # run clippy
just audit         # run cargo audit
just test-workspace # test both crates
```

### 2. Property-based tests with `proptest`
Add to `Cargo.toml` dev-dependencies:
```toml
[dev-dependencies]
proptest = "1.5"
```
Use for:
- `generate_claim_name` — random Unicode titles, verify invariants
- `is_private_ip` — random IP strings, compare against Python `ipaddress` module reference
- PKCE pair generation — verify `challenge == base64url(SHA256(verifier))` for 1000 random pairs

### 3. Fuzzing with `cargo-fuzz`
```bash
cargo install cargo-fuzz
cargo fuzz add validate_input
```
Targets:
- `validate_input` — random file data + random extensions
- `credential_decrypt` — random ciphertext → fuzz decryption error paths

### 4. Contract tests between library and CLI
Add an integration test that:
1. Spins up the CLI via `std::process::Command`
2. Runs `video-uploader auth --platform youtube --client-id test`
3. Verifies credentials file is created at the expected path

---

## CI Pipeline

```yaml
# .github/workflows/test.yml
name: Tests

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: clippy, rustfmt
      - run: cargo test --workspace --all-features
      - run: cargo clippy --workspace --all-features -- -D warnings
      - run: cargo audit
      - run: cargo test --doc

  miri:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@miri
      - run: cargo miri test
```

---

## Priority Order

| Priority | Tasks |
|----------|-------|
| **P0** (Security) | SSRF URL validation tests (#1 above), unknown platform validation (#6), wrong-passphrase decryption (#3) |
| **P1** (Correctness) | V1 migration test (#5), token error handling (#4), CGNAT IPv4-mapped edge case (#2) |
| **P2** (Coverage) | Clippy fixes, partial failure test (#7), progress listener tests |
| **P3** (Future) | Property-based tests, fuzzing, CLI contract tests, `cargo-fuzz` |

---

## Files to Modify

| File | Changes |
|------|---------|
| `src/platforms/youtube.rs` | Add SSRF validation tests |
| `src/net.rs` | Add IPv4-mapped CGNAT test |
| `src/validation.rs` | Add unknown platform validation test |
| `tests/integration.rs` | Add corrupted file test, V1 migration test |
| `tests/wiremock.rs` | Add token error handling test, fix clippy |
| `video-uploader/Cargo.toml` | Add `proptest` dev-dependency (P2) |
| `.github/workflows/test.yml` | Add CI pipeline (P3) |

---

## Verification Command

After all P0 and P1 tests are added, verify:

```bash
cargo test --workspace --all-features && \
cargo clippy --workspace --all-features -- -D warnings && \
cargo audit
```

Expected: **0 failures, 0 warnings (except deprecated legacys), 0 vulnerabilities**.