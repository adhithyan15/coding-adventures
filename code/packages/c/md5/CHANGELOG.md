# Changelog

All notable changes to `md5` (C) are documented in this file.

## [0.1.0] - 2026-07-11

### Added

- Initial pure-ISO C17 port of the Rust `md5` crate (RFC 1321): streaming
  `md5_init`/`update`/`final` plus one-shot `md5` and `md5_hex`. Fixed-buffer,
  allocation-free, little-endian.
- Tests (via the shared `iso-harness`) pinned to the RFC 1321 test suite plus a
  padding-boundary message and a streaming-equals-one-shot check — compiled and
  run under GCC, Clang, and MSVC with strict ISO-conformance flags.
