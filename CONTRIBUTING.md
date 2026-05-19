# Contributing to youtube-uploader

Thank you for your interest in contributing! Here are the guidelines:

## Getting Started

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/my-feature`)
3. Make your changes
4. Ensure all tests pass (`cargo test --workspace --features test-utils`)
5. Ensure clippy is clean (`cargo clippy --workspace --features test-utils -- -D warnings`)
6. Submit a pull request

## Code Standards

- **Zero clippy warnings** — CI enforces `-D warnings`
- **All tests must pass** — 213+ tests across the workspace
- **No inline TODOs** — Track all TODOs in `GUIDE.md` roadmap section
- **Default visibility = Private** — Never change this without explicit intent
- **Pretty-print via `output.rs`** — All user-facing output goes through the output module

## License

By contributing, you agree that your contributions will be licensed under the [MIT License](./LICENSE).
