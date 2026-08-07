# Changelog

All notable changes to `lz77` (C++) are documented in this file.

## [0.1.0] - 2026-07-11

### Added

- Initial pure-ISO C++17 header-only port of the Rust `lz77` crate:
  `ca::lz77::encode`/`decode`/`serialise`/`deserialise`/`compress`/`decompress`
  over a `token` struct, returning `std::vector`s.
- Tests (via the shared `iso-harness`) covering token structure and
  compress/decompress round-trips — compiled and run under GCC, Clang, and MSVC
  with strict ISO-conformance flags.
