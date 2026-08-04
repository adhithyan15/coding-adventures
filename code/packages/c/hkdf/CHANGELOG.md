# Changelog

All notable changes to `hkdf` (C) are documented in this file.

## [0.1.0] - 2026-07-11

### Added

- Initial pure-ISO C17 port of the Rust `hkdf` crate (RFC 5869): hash-agnostic
  `hkdf_extract`, `hkdf_expand`, and `hkdf`, built on the sibling `hmac`
  primitive. Overflow-guarded; empty-salt and length-limit handling per RFC.
- Tests (via the shared `iso-harness`) checking HKDF-SHA256 against the RFC 5869
  vectors (extract PRK + full OKM, including empty salt/info) plus the error
  paths — compiled and run under GCC, Clang, and MSVC with strict ISO flags.
