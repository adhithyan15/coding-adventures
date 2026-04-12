# Changelog

## [0.1.0] - 2026-04-12

### Added
- `aes_encrypt_block` / `aes_decrypt_block` — AES-128/192/256 block cipher (FIPS 197)
- `expand_key` — AES key schedule for all three key sizes
- `SBOX` / `INV_SBOX` — S-box and inverse, built from GF(2^8) inverse + affine transform
- Comprehensive test suite: FIPS 197 Appendix B, C.1, C.2, C.3 test vectors; round-trips; error handling
