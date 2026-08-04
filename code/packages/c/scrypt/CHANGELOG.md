# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-11

### Added

- Initial pure-ISO C17 port of the Rust `scrypt` crate (RFC 7914): the
  sequential memory-hard PBKDF.
- `scrypt(...)` deriving into a caller-provided buffer with a `ScryptStatus`
  return; full parameter validation matching the Rust error variants.
- Inline Salsa20/8 core, BlockMix (even-then-odd interleaving), and ROMix
  (V-table fill + data-dependent mixing).
- PBKDF2-HMAC-SHA256 expand/extract via the sibling `pbkdf2` package
  (cross-package `deps=` onto `pbkdf2` → `hmac` → `sha1`/`sha256`/`sha512`).
- Overflow-guarded `p*r` / `p*128*r` parameter checks and a checked
  `calloc(N, 128*r)` V-table allocation.
- Tests via the shared `iso-harness` (GCC, Clang, MSVC) using the published
  RFC 7914 §12 vectors (N=16, 1024, 16384), matching the Rust crate's tests.
