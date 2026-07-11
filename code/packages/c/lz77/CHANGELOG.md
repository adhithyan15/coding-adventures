# Changelog

All notable changes to `lz77` (C) are documented in this file.

## [0.1.0] - 2026-07-11

### Added

- Initial pure-ISO C17 port of the Rust `lz77` crate: `lz77_encode`, `decode`,
  `serialise`, `deserialise`, `compress`, `decompress` over `(offset, length,
  next_char)` tokens. malloc-owned outputs with overflow-guarded growth.
- Tests (via the shared `iso-harness`) covering token structure (literals + a
  backreference) and compress/decompress round-trips over assorted inputs —
  compiled and run under GCC, Clang, and MSVC with strict ISO-conformance flags.
