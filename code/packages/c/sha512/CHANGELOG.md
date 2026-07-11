# Changelog

All notable changes to `sha512` (C) are documented in this file.

## [0.1.0] - 2026-07-11

### Added

- Initial pure-ISO C17 port of the Rust `sha512` crate (FIPS 180-4): streaming
  `sha512_init`/`update`/`final` plus one-shot `sha512` and `sha512_hex`.
  Fixed-buffer, allocation-free, 128-bit length field.
- Tests (via the shared `iso-harness`) pinned to the published FIPS test vectors
  (empty, "abc", a 112-byte padding-boundary message) plus a streaming check —
  compiled and run under GCC, Clang, and MSVC with strict ISO-conformance flags.
