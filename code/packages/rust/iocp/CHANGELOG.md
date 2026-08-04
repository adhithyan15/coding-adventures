# Changelog

All notable changes to this package will be documented in this file.

## Unreleased

- Document the opaque-pointer Windows contracts and keep their safe wrappers
  clean under the Rust 1.97 Clippy raw-pointer lint.

## [0.1.0] - 2026-04-17

### Added

- `CompletionPacket` and `CompletionPort`
- Windows wrappers for completion-port creation, association, posting, and dequeue
- non-Windows unsupported fallback
