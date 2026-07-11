# Changelog

All notable changes to `lzw` (C++) are documented in this file.

## [0.1.0] - 2026-07-11

### Added

- Initial pure-ISO C++17 header-only port of the Rust `lzw` crate:
  `ca::lzw::compress` / `ca::lzw::decompress` (variable-width codes, CLEAR/STOP,
  dictionary-full reset, LSB-first packing, 4-byte length header).
- Tests (via the shared `iso-harness`) covering round-trips over long runs,
  repeating patterns, and the full byte alphabet, plus header and malformed-input
  checks — under GCC, Clang, and MSVC with strict ISO-conformance flags.
