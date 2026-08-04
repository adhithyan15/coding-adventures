# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-11

### Added

- Pure ISO C++17, header-only port of the Rust `aes` crate, in namespace
  `ca::aes`: the AES block cipher (FIPS 197) for 128-, 192-, and 256-bit keys.
- `encrypt_block` / `decrypt_block` (→ `std::optional<block_t>`), `sbox` /
  `inv_sbox`.
- The S-box is built from GF(2^8) inverses via the sibling header-only `gf256`
  package (`ca::gf256::Field(0x11B)`) plus the AES affine transform, through a
  thread-safe function-local static. Header-only dependency
  (`# build-tool: deps=cpp/gf256`).
- Tests pinned to the FIPS 197 known-answer vectors (AES-128/192/256, Appendices
  B and C) plus S-box bijection/inverse checks, under GCC and Clang via
  `iso-harness`.
