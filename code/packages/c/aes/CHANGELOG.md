# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-11

### Added

- Pure ISO C17 port of the Rust `aes` crate: the AES block cipher (FIPS 197) for
  128-, 192-, and 256-bit keys.
- API: `aes_encrypt_block` / `aes_decrypt_block`, `aes_expand_key` (key
  schedule), `aes_sbox` / `aes_inv_sbox`.
- The S-box is built from GF(2^8) inverses via the sibling `gf256` package
  (`# build-tool: deps=c/gf256`) plus the AES affine transform, exactly as the
  crate uses `gf256::Field`.
- Fixed-size stack storage for round keys and state (the cipher allocates
  nothing); single-threaded lazy S-box build.
- Tests pinned to the FIPS 197 known-answer vectors (AES-128/192/256, Appendices
  B and C) plus S-box bijection/inverse checks, under GCC and Clang via
  `iso-harness`.
