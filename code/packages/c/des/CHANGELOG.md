# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-11

### Added

- Pure ISO C17 port of the Rust `des` crate: the DES block cipher (FIPS 46) and
  Triple DES (NIST SP 800-67), on the standard bit-array representation.
- API: `des_expand_key`, `des_encrypt_block` / `des_decrypt_block`,
  `des_ecb_encrypt` / `des_ecb_decrypt` (ECB with PKCS#7 padding),
  `des_tdea_encrypt_block` / `des_tdea_decrypt_block` (Triple DES EDE).
- Fixed-size stack buffers (the block cipher allocates nothing; ECB mallocs its
  output with a size-overflow guard); PKCS#7 padding validated on decrypt.
- Tests pinned to the FIPS 46 worked example and NIST SP 800-20 known-answer
  vectors, plus round-trips, ECB, and Triple DES, under GCC and Clang via
  `iso-harness`.
