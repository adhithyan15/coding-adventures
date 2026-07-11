# Changelog

All notable changes to `blake2b` (C) are documented in this file.

## [0.1.0] - 2026-07-11

### Added

- Initial pure-ISO C17 port of the Rust `blake2b` crate (RFC 7693): streaming
  `blake2b_init`/`update`/`final` (configurable digest size, optional key, salt,
  personalization) plus one-shot `blake2b` and `blake2b_hex`. Fixed-buffer,
  allocation-free.
- Tests (via the shared `iso-harness`) pinned to the published RFC 7693 vectors
  (BLAKE2b-512 empty/"abc", BLAKE2b-256 "abc") plus streaming, keyed-determinism,
  and parameter-validation checks — under GCC, Clang, and MSVC with strict ISO
  flags.
