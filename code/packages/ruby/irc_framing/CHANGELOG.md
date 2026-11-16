# Changelog

## [Unreleased]

### Fixed
- SEC-011 (CWE-400/770): `Framer#feed` buffered input without limit when no LF arrived. The unterminated tail is now capped at 511 bytes (510 content plus a CR; RFC 1459 and RFC 2812 section 2.3: 512 including CRLF). Past the cap the partial line is dropped and input is skipped up to the next LF (fail closed, same silent discard as a complete overlong line; no API change, so no version bump). `reset` also clears the discard state. Ported from the Rust fix (PR #19359).

## [0.1.0] - 2026-04-12

### Added
- Initial implementation: `Framer` with `feed`, `frames`, `reset`, `buffer_size`
