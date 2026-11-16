# Changelog

All notable changes to `irc-framing` (Go) will be documented in this file.

## [Unreleased]

### Fixed

- SEC-011 (CWE-400/770): `Feed` buffered input without limit when no LF arrived. The unterminated tail is now capped at 511 bytes (510 content plus a CR; RFC 1459 and RFC 2812 section 2.3: 512 including CRLF). Past the cap the partial line is dropped and input is skipped up to the next LF (fail closed, same silent discard as a complete overlong line; no API change, so no version bump). `Reset()` also clears the discard state. Ported from the Rust fix (PR #19359).

## [0.1.0] - 2026-04-12

### Added

- Initial Go implementation ported from the Python reference implementation.
- `Framer` struct with internal byte buffer.
- `NewFramer() *Framer` — creates a Framer with pre-allocated 1024-byte buffer.
- `Feed(data []byte)` — appends raw bytes to the internal buffer.
- `Frames() [][]byte` — drains all complete CRLF-terminated lines.
  - Strips trailing `\r` before `\n` (handles both CRLF and LF).
  - Silently discards lines longer than 510 bytes (RFC 1459 maximum content size).
- `Reset()` — clears the buffer for connection reuse.
- `BufferSize() int` — returns the number of buffered bytes.
- 16 unit tests: empty framer, partial messages, CRLF vs LF, multiple messages,
  overlong line discard, exact max-length lines, reset semantics, double-drain.
- 100% statement coverage.
