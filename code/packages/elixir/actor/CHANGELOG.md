# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-03-22

### Added
- `Message` struct with immutable design, binary wire format, factory functions
- `Channel` struct with append-only log, binary persistence, crash recovery
- `Actor` struct with mailbox, behavior functions, state management
- `ActorSystem` with lifecycle management, message delivery, dead letters
- Minimal JSON encoder/decoder (no external dependencies)
- Comprehensive test suite with 89 test cases at 86%+ coverage
