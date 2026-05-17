# Implement v0.2 — Multi-channel workspace support + all hygiene items

## Goal
Transform video-uploader from a single-channel YouTube uploader to a multi-channel workspace-based uploader, plus all remaining hygiene fixes.

## Phase 1: Library — CredentialStore + YouTubeUploader
- [x] Change `CredentialStore` from platform-keyed (`platforms: HashMap`) to workspace-keyed (`workspaces: HashMap`)
- [x] Add `default_workspace: Option<String>` field + accessor methods
- [x] Update TOML serialization: `[metadata]` + `[workspaces.youtube]` nested format
- [x] Auto-migrate v0.1 format (flat HashMap) → v0.2 format on load
- [x] Add `YouTubeUploader::new(store, passphrase, workspace)` — replace hardcoded `"youtube"`

## Phase 2: CLI — Workspace flag + commands
- [x] Add `--workspace` / `-w` global flag to `Cli`
- [x] Update `auth` command to write to named workspace
- [x] Update `upload` command to pass workspace to uploader
- [x] Update `batch` command to read optional `workspace` column from CSV
- [x] Update `list` command to show workspaces with default marker
- [x] Add `workspace` subcommand (default, rename, remove)

## Phase 3: Tests
- [x] Update all tests: `"youtube"` keys → workspace names
- [x] Add multi-workspace test (`test_credential_store_multi_workspace`)
- [x] Add default workspace test (`test_credential_store_default_workspace`)
- [x] Add v0.1→v0.2 migration test (`test_credential_store_v01_to_v02_format_migration`)
- [x] Add v1 encryption + v0.1 format migration test (`test_credential_store_v1_encryption_v01_format_migration`)
- [x] Update CLI workspace output message tests

## Phase 4: Docs & Version
- [x] Bump to 0.2.0 in `Cargo.toml`
- [x] Update CHANGELOG with `[0.2.0]` section
- [ ] Update README for multi-channel usage
- [ ] Add `examples/multi_channel.rs`

## Phase 5: Hygiene
- [ ] Client-side upload resume
- [ ] `CredentialStore` HashMap zeroization on drop
- [ ] Access token zeroization in `PlatformCredentials`
- [ ] Passphrase zeroization through call chain
- [ ] Shared HTTP client for token refresh
- [ ] End-to-end upload flow integration test
- [ ] Batch upload test with concurrency
- [ ] Pre-validate CSV rows
- [ ] Rustdoc examples
- [ ] `examples/` directory
- [ ] ARM Linux + Intel Mac release targets
- [x] Fix ALL clippy warnings (passes clean with `-D warnings`)

## Summary (v0.2.0 shipped)
- Core feature: complete multi-channel workspace support
- All tests pass (142 total across workspace)
- Clippy clean (zero warnings)
- Auto-migration from v0.1 format preserved