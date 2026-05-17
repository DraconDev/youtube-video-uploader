# Testing Plan — video-uploader

## Overview

- **118 tests passing** (62 lib + 22 integration + 15 CLI + 6 proptest + 7 file_io + 6 http_integration)
- **Grade: A** — clippy clean, fmt clean, zero warnings
- **cargo audit: 0 known vulnerabilities**

---

## Current Coverage

### By Layer

| Layer | Files | What it covers |
|-------|-------|----------------|
| Unit tests | `src/**/*.rs` (inside `#[cfg(test)]` mods) | Core logic, validation, crypto, IP checks |
| Integration tests | `tests/integration.rs` | CredentialStore persistence, roundtrip, error variants |
| Wiremock tests | `tests/wiremock.rs` | HTTP mocking for YouTube upload, auth, and refresh flows |
| Proptest | `tests/proptest.rs` | Property-based: `is_private_ip`, PKCE pair, credential roundtrip |
| File I/O tests | `tests/file_io.rs` | File reading, extension detection, concurrent access |
| HTTP tests | `tests/http_integration.rs` | Connection refused, redirects, 5xx retry, TCP echo |
| CLI tests | `tests/cli.rs` | CLI arg parsing, help flags, passphrase handling |

### By Platform

| Platform | Upload mocked | Auth mocked | Chunk/resume | Error handling |
|----------|--------------|-------------|--------------|----------------|
| YouTube | ✅ | ✅ | ✅ (308 + retry) | ✅ (4xx, 5xx) |

---

## Gap Analysis

### Missing Tests

#### High Priority

1. **YouTube URL validation SSRF** ✅ covered by `test_validate_upload_url_rejects_evil_googleapis`

#### Medium Priority

2. **`auth::device_code::poll_for_token` — token error response**
   - `test_token_error_response_deserialization` exists but no test for error *handling*

3. **`CredentialStore` V1 → V2 migration** ✅ covered by `test_credential_store_v1_to_v2_auto_migration`

4. **`Visibility` serialization edge cases**
   - ✅ All three variants covered (public, unlisted, private)

#### Low Priority

5. **`progress.rs` listener callbacks** — integration tests cover

6. **PKCE pair generation** ✅ covered by proptest + unit tests

---

## Testing Infrastructure

### Property-based tests (proptest) ✅

Already in place:
- `proptest_is_private_ip_matches_reference` — random string inputs
- `proptest_credential_store_roundtrip_random_data` — encryption roundtrip
- `proptest_pkce_pair_verifies_correctly` — 100 random pairs
- `proptest_pkce_verifier_length_is_valid` — length bounds

### Fuzzing (Future)

Targets:
- `validate_input` — random file data + random extensions
- `credential_decrypt` — random ciphertext → fuzz decryption error paths

---

## CI Pipeline

Existing: `.github/workflows/ci.yml` (test, clippy, fmt, docs, audit)
Release: `.github/workflows/release.yml` (cross-platform binaries on v* tags)

---

## Verification Command

```bash
cargo test --workspace && \
cargo clippy --workspace -- -D warnings && \
cargo fmt --all -- --check
```

Expected: **0 failures, 0 warnings, 0 formatting issues**.
