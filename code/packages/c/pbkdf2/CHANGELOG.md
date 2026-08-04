# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-11

### Added

- Initial pure-ISO C17 port of the Rust `pbkdf2` crate (RFC 8018 § 5.2).
- Generic `pbkdf2(hash, h_len, block_size, ...)` core deriving into a
  caller-provided buffer, plus `pbkdf2_hmac_sha1` / `_sha256` / `_sha512`
  convenience wrappers.
- HMAC PRF via the sibling `hmac` package over the `sha1` / `sha256` / `sha512`
  packages (cross-package `deps=`).
- `Pbkdf2Status` result codes; empty-password guard (overridable), zero
  iteration / key-length rejection, and a 2^20-byte key-length cap to bound
  memory and the 32-bit block counter. Salt||counter concatenation guarded
  against size_t overflow.
- Tests via the shared `iso-harness` (GCC, Clang, MSVC) using the published
  RFC 6070 (HMAC-SHA1) and RFC 7914 (HMAC-SHA256) vectors, matching the Rust
  crate's own tests.
