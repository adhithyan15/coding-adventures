# Changelog

All notable changes to this package will be documented in this file.

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
