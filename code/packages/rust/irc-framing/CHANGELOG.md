# Changelog — irc-framing

## [Unreleased]

### Fixed

- `required_capabilities.json` rewritten to the canonical schema-v1 shape (version, package, capabilities objects, justification) so the hardened capability analyzer accepts it.

## [0.1.0] — 2026-04-12

### Added
- `Framer` struct with internal `Vec<u8>` buffer
- `feed(&mut self, data: &[u8])`, `frames(&mut self) -> Vec<Vec<u8>>`, `reset()`, `buffer_size()`
- RFC 1459 overlong line discard (>510 bytes)
- Both CRLF and bare LF support
- Comprehensive unit tests
