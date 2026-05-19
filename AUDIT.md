# Audit — youtube-uploader v0.6.0

**Date**: 2026-05-19

## ✅ All Clear

| Check | Result |
|-------|--------|
| `cargo clippy -- -D warnings` | Clean |
| `cargo test --workspace --features test-utils` | 213 passed, 0 failed |
| `cargo deny check` | advisories ok, bans ok, licenses ok, sources ok |
| `cargo fmt --check` | Clean |
| `cargo doc --workspace --no-deps` | Clean (no warnings) |
| Inline TODOs/HACK/FIXME | Zero in source |
| Dead code warnings | Zero |
| Old name (`video-uploader`) references in source/docs | Zero (only CHANGELOG history) |
| AGPL/CLA/Commercial-License references in active docs | Zero |
| Stale files (spec.md, TESTING_PLAN.md, etc.) | All removed |

## ⚠️ Known Minor Items (not blocking)

| # | Item | Status |
|---|------|--------|
| 1 | Missing rustdoc on some `pub fn` in `upload.rs` (builders/getters) | Low priority — they have `///` on the struct, individual methods are self-explanatory |
| 2 | `main.rs` at 914 lines | Could split handlers into submodules; not urgent |
| 3 | `cargo audit` binary won't run on NixOS | NixOS dynamic linker issue; `cargo deny check advisories` covers the same ground |
| 4 | `output.rs` has 8 `#[allow(dead_code)]` functions | Intentional — utility functions for future use (version_banner, quota_info, etc.) |

## File Inventory

### Source (library)
| File | Lines | Role |
|------|-------|------|
| `youtube-uploader/src/config.rs` | 873 | Encrypted CredentialStore with workspaces |
| `youtube-uploader/src/youtube.rs` | 748 | YouTubeUploader (resumable upload, token refresh) |
| `youtube-uploader/src/upload.rs` | 641 | VideoUpload, UploadResult, Visibility, License |
| `youtube-uploader/src/profile.rs` | 580 | UploadProfile, VideoMeta (TOML presets + per-video) |
| `youtube-uploader/src/auth/device_code.rs` | 311 | OAuth2 device code flow |
| `youtube-uploader/src/resume.rs` | 230 | UploadState (crash recovery) |
| `youtube-uploader/src/net.rs` | 224 | HTTP client, retry, SSRF protection |
| `youtube-uploader/src/validation.rs` | 85 | File validation |
| `youtube-uploader/src/error.rs` | 160 | UploadError enum |
| `youtube-uploader/src/progress.rs` | 85 | ProgressListener trait |

### Source (CLI)
| File | Lines | Role |
|------|-------|------|
| `youtube-uploader-cli/src/main.rs` | 914 | CLI entry point |
| `youtube-uploader-cli/src/output.rs` | 598 | Pretty-print output module |

### Tests
| File | Lines | Count | What |
|------|-------|-------|------|
| `youtube-uploader/tests/wiremock.rs` | 984 | 21 | HTTP mocking for upload/auth/refresh |
| `youtube-uploader/tests/integration.rs` | 528 | 21 | CredentialStore, VideoUpload, errors |
| `youtube-uploader-cli/tests/cli.rs` | 476 | 24 | CLI arg parsing, subcommands |
| `youtube-uploader/tests/http_integration.rs` | 279 | 6 | Real HTTP edge cases |
| `youtube-uploader/tests/proptest.rs` | 192 | 11 | Property-based tests |
| `youtube-uploader/tests/file_io.rs` | ~120 | 7 | File persistence |

### Docs
| File | Purpose |
|------|---------|
| `README.md` | Quick start + library example |
| `GUIDE.md` | Full guide (setup, CLI ref, profiles, automation, architecture, security) |
| `CHANGELOG.md` | Version history |
| `CONTRIBUTING.md` | Contribution guidelines |
| `LICENSE` | MIT |

### Config
| File | Purpose |
|------|---------|
| `Cargo.toml` | Workspace definition |
| `deny.toml` | cargo-deny license/advisory config |
| `rustfmt.toml` | Formatting config |
| `.env` | OAuth2 client credentials (gitignored) |

## Security

| Layer | Detail |
|-------|--------|
| Encryption | AES-256-GCM |
| Key derivation | PBKDF2, 100K iterations |
| Memory | `Zeroizing<String>` for all secrets, zeroize on drop |
| SSRF | Upload URLs validated to `*.googleapis.com` / `*.google.com` |
| Default visibility | **Private** — uploads never accidentally public |
| License | MIT — no copyleft restrictions |

## Test Coverage Summary

| Suite | Count |
|-------|-------|
| Unit (lib) | 108 |
| Integration | 21 |
| CLI | 24 |
| Wiremock | 21 |
| Proptest | 11 |
| File I/O | 7 |
| HTTP integration | 6 |
| Output | 11 |
| Fuzz | 4 |
| **Total** | **213** |
