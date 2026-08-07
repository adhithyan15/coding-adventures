# Changelog

All notable changes to `hkdf` (C++) are documented in this file.

## [0.1.0] - 2026-07-11

### Added

- Initial pure-ISO C++17 header-only port of the Rust `hkdf` crate (RFC 5869):
  the `ca::hkdf_extract` / `ca::hkdf_expand` / `ca::hkdf` function templates
  (hash-agnostic), built on the sibling header-only `hmac`.
- Tests (via the shared `iso-harness`) checking HKDF-SHA256 against the RFC 5869
  vectors plus throwing error paths — compiled and run under GCC, Clang, and
  MSVC with strict ISO-conformance flags.
