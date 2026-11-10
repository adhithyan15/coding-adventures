# Changelog

All notable changes to `lz77` (C++) are documented in this file.

## [Unreleased]

### Security

- Memory safety (CWE-125/CWE-191): decode computed out.size() - offset without validating the offset, so a token with offset 0 or beyond the output wrapped the index and read outside the vector (undefined behaviour). Decoding now stops at the first such token and returns what was produced so far. The header token count was already clamped to the data present.
- Added hostile-header tests (header-claimed counts/lengths of 2^32-1, counts beyond the data) and round trips over every byte string of length <= 2 plus a seeded random sample to tests/*_test.cpp. Verified clean under -fsanitize=address,undefined (g++) as well as the iso-harness g++/clang++ runs.

## [0.1.0] - 2026-07-11

### Added

- Initial pure-ISO C++17 header-only port of the Rust `lz77` crate:
  `ca::lz77::encode`/`decode`/`serialise`/`deserialise`/`compress`/`decompress`
  over a `token` struct, returning `std::vector`s.
- Tests (via the shared `iso-harness`) covering token structure and
  compress/decompress round-trips — compiled and run under GCC, Clang, and MSVC
  with strict ISO-conformance flags.
