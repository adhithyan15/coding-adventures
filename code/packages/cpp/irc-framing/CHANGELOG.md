# Changelog

All notable changes to this package will be documented in this file.

## [Unreleased]

### Fixed

- SEC-011 (CWE-400/770): `Framer::feed` buffered input without limit when no LF arrived. The unterminated tail is now capped at 511 bytes (510 content plus a CR; RFC 1459 and RFC 2812 section 2.3: 512 including CRLF). Past the cap the partial line is dropped and input is skipped up to the next LF (fail closed, same silent discard as a complete overlong line; no API change, so no version bump). `reset()` also clears the discard state. The header documentation previously described the unbounded buffer as by design; that paragraph is replaced. New public constant `Framer::MAX_PENDING_BYTES`. Ported from the Rust fix (PR #19359).

## [0.1.0] - 2026-07-12

### Added

- Initial pure-ISO C++17 header-only port of the Rust `irc-framing` crate, in
  namespace `ca::irc`: a stateful byte-stream-to-line-frame converter.
- `Framer` with `feed` (raw bytes or a `std::string`), `frames` (returns
  `std::vector<std::vector<unsigned char>>` of complete CRLF/LF-stripped lines),
  `reset`, and `buffer_size`.
- Overlong lines (content > 510 bytes, RFC 1459 §2.3) are silently discarded.
  Frame extraction scans with a cursor and drains the consumed prefix once (the
  Rust original drains after each line); the observable result is identical.
- Tests via the shared `iso-harness` (GCC, Clang, MSVC): CRLF and lone-LF
  framing, partial buffering across feeds, the CR/LF split boundary, the empty
  frame, the 510-byte overlong-line rule, reset, and multi-feed sequences.
