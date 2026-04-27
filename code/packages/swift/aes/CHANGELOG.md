# Changelog

## [0.1.0] - 2026-04-12

### Added
- `aesEncryptBlock` / `aesDecryptBlock` — AES-128/192/256 block cipher (FIPS 197)
- `expandKey` — AES key schedule for all three key sizes
- `sbox` / `invSbox` — S-box and inverse, built from GF(2^8) inverse + affine transform
- Comprehensive test suite: FIPS 197 Appendix B, C.1, C.2, C.3 test vectors; round-trips; error handling
