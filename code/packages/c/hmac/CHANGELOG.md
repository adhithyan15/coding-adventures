# Changelog

All notable changes to `hmac` (C) are documented in this file.

## [0.1.0] - 2026-07-11

### Added

- Initial pure-ISO C17 port of the Rust `hmac` crate (RFC 2104): hash-agnostic
  `hmac_compute` (one-shot hash function pointer + block/digest sizes) and a
  constant-time `hmac_verify`.
- Tests (via the shared `iso-harness`) checking HMAC-SHA256 against the RFC 4231
  vectors — including a key longer than the block — plus constant-time verify;
  compiled and run under GCC, Clang, and MSVC with strict ISO-conformance flags.
