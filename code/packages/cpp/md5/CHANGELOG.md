# Changelog

All notable changes to `md5` (C++) are documented in this file.

## [0.1.0] - 2026-07-11

### Added

- Initial pure-ISO C++17 header-only port of the Rust `md5` crate (RFC 1321):
  `ca::md5_hasher` (`update`/`digest`/`hex_digest`) plus one-shot `ca::md5`
  (returns `std::array<uint8_t,16>`) and `ca::md5_hex`.
- Tests (via the shared `iso-harness`) pinned to the RFC 1321 test suite plus
  padding-boundary and streaming checks — compiled and run under GCC, Clang, and
  MSVC with strict ISO-conformance flags.
