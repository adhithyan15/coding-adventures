# Changelog

All notable changes to `sha1` (C++) are documented in this file.

## [0.1.0] - 2026-07-11

### Added

- Initial pure-ISO C++17 header-only port of the Rust `sha1` crate (FIPS 180-4):
  `ca::sha1_hasher` (`update`/`digest`/`hex_digest`) plus one-shot `ca::sha1`
  (returns `std::array<uint8_t,20>`) and `ca::sha1_hex`.
- Tests (via the shared `iso-harness`) pinned to the published FIPS test vectors
  plus streaming and padding-boundary checks — compiled and run under GCC,
  Clang, and MSVC with strict ISO-conformance flags.
