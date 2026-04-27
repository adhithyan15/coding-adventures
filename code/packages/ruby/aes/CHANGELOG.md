# Changelog — coding_adventures_aes

## [0.1.0] — 2026-04-11

### Added

- Initial implementation of AES block cipher (FIPS 197) supporting AES-128, AES-192, and AES-256.
- `Aes::SBOX` and `Aes::INV_SBOX`: 256-entry tables computed at load time from GF(2^8) inverses (polynomial 0x11B) and the AES affine transformation.
- `Aes.expand_key(key)`: key schedule for 16-, 24-, and 32-byte keys (11, 13, or 15 round keys).
- `Aes.aes_encrypt_block(block, key)`: 16-byte encryption.
- `Aes.aes_decrypt_block(block, key)`: 16-byte decryption using InvSubBytes, InvShiftRows, InvMixColumns.
- Inline GF(2^8) arithmetic using Russian peasant multiplication (polynomial 0x11B) — no external gem dependency.
- Comprehensive test suite: FIPS 197 Appendix B, C.1, C.2, C.3 vectors, S-box properties, key schedule, round-trip for all key sizes, avalanche effect, and error handling.
