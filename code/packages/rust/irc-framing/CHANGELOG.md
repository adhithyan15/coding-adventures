# Changelog — irc-framing

## [0.1.0] — 2026-04-12

### Added
- `Framer` struct with internal `Vec<u8>` buffer
- `feed(&mut self, data: &[u8])`, `frames(&mut self) -> Vec<Vec<u8>>`, `reset()`, `buffer_size()`
- RFC 1459 overlong line discard (>510 bytes)
- Both CRLF and bare LF support
- Comprehensive unit tests
