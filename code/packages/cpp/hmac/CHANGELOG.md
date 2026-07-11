# Changelog

All notable changes to `hmac` (C++) are documented in this file.

## [0.1.0] - 2026-07-11

### Added

- Initial pure-ISO C++17 header-only port of the Rust `hmac` crate (RFC 2104):
  the `ca::hmac` function template (hash-agnostic) plus constant-time
  `ca::hmac_verify`.
- Tests (via the shared `iso-harness`) checking HMAC-SHA256 against the RFC 4231
  vectors (including a key longer than the block) plus constant-time verify;
  compiled and run under GCC, Clang, and MSVC with strict ISO-conformance flags.
