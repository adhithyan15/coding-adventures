# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-11

### Added

- Initial pure-ISO C++17 header-only port of the Rust `pbkdf2` crate
  (RFC 8018 § 5.2), in namespace `ca`.
- Generic `pbkdf2(hash, digest_size, block_size, ...)` core plus
  `pbkdf2_hmac_sha1` / `_sha256` / `_sha512` and `_hex` convenience wrappers,
  returning `std::vector<std::uint8_t>` / `std::string`.
- HMAC PRF via the sibling header-only `hmac` package over `sha1` / `sha256` /
  `sha512` (cross-package `deps=`).
- Throws `std::invalid_argument` on empty password (overridable), zero
  iterations, and zero / oversized (> 2^20 byte) key length.
- Tests via the shared `iso-harness` (GCC, Clang, MSVC) using the published
  RFC 6070 (HMAC-SHA1) and RFC 7914 (HMAC-SHA256) vectors, matching the Rust
  crate's own tests.
