# Changelog

All notable changes to `lzw` (C) are documented in this file.

## [0.1.0] - 2026-07-11

### Added

- Initial pure-ISO C17 port of the Rust `lzw` crate: `lzw_compress` /
  `lzw_decompress` with variable-width codes (9→16 bits), CLEAR/STOP handling,
  dictionary-full reset, LSB-first bit packing, and a 4-byte length header.
  Open-addressed encoder dictionary; prefix-chain decoder with the KwKwK case.
- Tests (via the shared `iso-harness`) covering compress/decompress round-trips
  over long runs, repeating patterns, and the full byte alphabet, plus header
  and malformed-input checks — under GCC, Clang, and MSVC with strict ISO flags.
