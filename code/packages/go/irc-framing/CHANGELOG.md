# Changelog

All notable changes to `irc-framing` (Go) will be documented in this file.

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
