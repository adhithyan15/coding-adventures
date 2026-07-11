# Changelog

All notable changes to `blake2b` (C++) are documented in this file.

## [0.1.0] - 2026-07-11

### Added

- Initial pure-ISO C++17 header-only port of the Rust `blake2b` crate (RFC 7693):
  `ca::blake2b_hasher` (digest size, optional key/salt/personal;
  `update`/`digest`/`hex_digest`) plus one-shot `ca::blake2b` / `ca::blake2b_hex`.
- Tests (via the shared `iso-harness`) pinned to the RFC 7693 vectors plus
  streaming, keyed, and parameter-validation checks — under GCC, Clang, and MSVC
  with strict ISO-conformance flags.
