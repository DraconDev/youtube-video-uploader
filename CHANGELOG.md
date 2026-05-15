# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

### Changed
- Updated CLI description to reflect actual supported platforms (YouTube, Odysee)
- Improved README library usage example to use `UploaderRegistry`

### Fixed
- `ProgressListener::on_error` is now called on upload failure in both YouTube and Odysee uploaders
- Removed spurious `#[allow(dead_code)]` attributes from `OdyseeUploader` methods

## [0.1.0] - 2024-01-01

### Added
- `video-uploader` library with `PlatformUploader` trait for multi-platform uploads
- YouTube uploader with OAuth2 device code flow and resumable uploads
- Odysee/LBRY uploader via lbrynet daemon JSON-RPC API
- `UploaderRegistry` for concurrent multi-platform dispatch with configurable concurrency
- AES-GCM encrypted credential storage (V2 format with PBKDF2 key derivation)
- V1 → V2 credential migration on load
- `ProgressListener` trait with `StderrProgressListener` implementation
- `VideoUpload` builder API with visibility, tags, description, category support
- Platform-specific file size validation (YouTube: 128 GiB, Odysee: 2 GiB)
- `video-uploader` CLI with `auth`, `upload`, `batch`, and `list` commands
- CSV batch upload support
- Full async/await runtime using Tokio
- Comprehensive test suite with wiremock for HTTP mocking
- CI pipeline (tests, clippy, fmt, docs, audit)

[unreleased]: https://github.com/dracon/video-uploader/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/dracon/video-uploader/releases/tag/v0.1.0