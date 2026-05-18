# Testing Plan — video-uploader

## Overview

- **175+ tests** passing (85 lib + 22 CLI + 19 wiremock + 7 file_io + 6 http_integration + 21 integration + 11 proptest + 4 fuzz)
- **Grade: A** — clippy clean, fmt clean, zero warnings
- **cargo deny**: advisories ok, bans ok, licenses ok, sources ok
- **cargo audit**: 0 known vulnerabilities

## Current Coverage

### By Layer

| Layer | Files | What it covers |
|-------|-------|----------------|
| Unit tests | `src/**/*.rs` (inside `#[cfg(test)]` mods) | Core logic, validation, crypto, IP checks, profiles, meta TOML |
| Integration tests | `tests/integration.rs` | CredentialStore persistence, roundtrip, error variants |
| Wiremock tests | `tests/wiremock.rs` | HTTP mocking for YouTube upload, auth, and refresh flows |
| Proptest | `tests/proptest.rs` | Property-based: `is_private_ip`, credential roundtrip |
| Fuzz tests | `fuzz/tests/` | Fuzz: credential decrypt, input validation |
| CLI tests | `video-uploader-cli/tests/cli.rs` | CLI arg parsing, help flags, passphrase handling |

### By Feature

| Feature | Test Type | Coverage |
|---------|-----------|----------|
| Encryption/decryption | Integration | V1→V2 migration, roundtrip, wrong passphrase, corrupted files |
| Credential store | Unit + Integration | Multi-workspace, default workspace, rename, remove |
| Upload validation | Unit | File size, extension, empty title, missing file |
| Resumable upload | Wiremock | Initiate → chunk upload → complete flow |
| Token refresh | Wiremock | Refresh flow with mock token endpoint |
| Device code flow | Wiremock | Start + poll flow |
| IP check (SSRF) | Unit + Proptest | Private IPs, public IPs, edge cases |
| Profiles | Unit | Load, list, resolve, apply_profile, VideoMeta |
| Channel info | Unit + CLI | fetch_channel_info, channel subcommand help |
| Batch upload | CLI | Dry run, multi-row with workspaces, CSV validation |
| Visibility default | Unit | Private by default in all paths |
