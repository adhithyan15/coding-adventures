# Changelog

All notable changes to `lzw` (C++) are documented in this file.

## [Unreleased]

### Security

- decompress stops expanding codes once the declared original_length is reached (each code can emit ~4 KiB, so a small stream could previously expand far beyond it before the final resize). Valid streams are unchanged.
- Added hostile-header tests (header-claimed counts/lengths of 2^32-1, counts beyond the data) and round trips over every byte string of length <= 2 plus a seeded random sample to tests/*_test.cpp. Verified clean under -fsanitize=address,undefined (g++) as well as the iso-harness g++/clang++ runs.

## [0.1.0] - 2026-07-11

### Added

- Initial pure-ISO C++17 header-only port of the Rust `lzw` crate:
  `ca::lzw::compress` / `ca::lzw::decompress` (variable-width codes, CLEAR/STOP,
  dictionary-full reset, LSB-first packing, 4-byte length header).
- Tests (via the shared `iso-harness`) covering round-trips over long runs,
  repeating patterns, and the full byte alphabet, plus header and malformed-input
  checks — under GCC, Clang, and MSVC with strict ISO-conformance flags.
