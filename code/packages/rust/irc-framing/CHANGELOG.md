# Changelog — irc-framing

## [Unreleased]

### Fixed

- SEC-011 (CWE-400/770): `Framer::feed` buffered input without limit when no LF arrived. The unterminated tail is now capped at 511 bytes (510 content plus a CR; RFC 1459 and RFC 2812 section 2.3: 512 including CRLF). Past the cap the partial line is dropped and input is skipped up to the next LF (fail closed, same silent discard as a complete overlong line; no API change, so no version bump). `reset()` also clears the discard state.
- `required_capabilities.json` rewritten to the canonical schema-v1 shape (version, package, capabilities objects, justification) so the hardened capability analyzer accepts it.

## [0.1.0] — 2026-04-12

### Added
- `Framer` struct with internal `Vec<u8>` buffer
- `feed(&mut self, data: &[u8])`, `frames(&mut self) -> Vec<Vec<u8>>`, `reset()`, `buffer_size()`
- RFC 1459 overlong line discard (>510 bytes)
- Both CRLF and bare LF support
- Comprehensive unit tests
