# Changelog

All notable changes to this package will be documented in this file.

## Unreleased

### Fixed

- Replaced an index loop in the channel recovery-order test with
  `iter().enumerate().take(100)` to satisfy clippy 1.100
  `needless_range_loop`; behavior unchanged.

## [0.1.0] - 2026-03-22

### Added
- `Message` struct with immutable design, binary wire format, factory methods
- `Channel` struct with append-only log, binary persistence, crash recovery
- `Actor` with mailbox, behavior closures, state management
- `ActorSystem` runtime with lifecycle management, message delivery, dead letters
- Comprehensive test suite with 58 test cases targeting 95%+ coverage
