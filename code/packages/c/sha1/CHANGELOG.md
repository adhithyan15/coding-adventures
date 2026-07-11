# Changelog

All notable changes to `sha1` (C) are documented in this file.

## [0.1.0] - 2026-07-11

### Added

- Initial pure-ISO C17 port of the Rust `sha1` crate (FIPS 180-4): streaming
  `sha1_init`/`update`/`final` plus one-shot `sha1` and `sha1_hex`. Fixed-buffer,
  allocation-free.
- Tests (via the shared `iso-harness`) pinned to the published FIPS test vectors
  (empty, "abc", the 56-byte padding-boundary message) plus a streaming-equals-
  one-shot check — compiled and run under GCC, Clang, and MSVC with strict
  ISO-conformance flags.
