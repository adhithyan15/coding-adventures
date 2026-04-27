# Changelog — @coding-adventures/aes

## [0.1.0] — 2026-04-11

### Added

- Initial implementation of AES block cipher (FIPS 197) supporting AES-128, AES-192, and AES-256.
- `SBOX` and `INV_SBOX`: 256-entry lookup tables computed at module load time from GF(2^8) inverses (polynomial 0x11B) and the AES affine transformation.
- `expandKey(key)`: key schedule for 16-, 24-, and 32-byte keys. Produces 11, 13, or 15 round keys respectively.
- `aesEncryptBlock(block, key)`: 16-byte encryption through AddRoundKey + Nr-1 full rounds (SubBytes, ShiftRows, MixColumns, AddRoundKey) + final round (no MixColumns).
- `aesDecryptBlock(block, key)`: 16-byte decryption using inverse operations (InvShiftRows, InvSubBytes, InvMixColumns).
- `toHex(bytes)` / `fromHex(hex)`: utility conversions.
- Depends on `@coding-adventures/gf256`'s `createField(0x11B)` for GF(2^8) arithmetic.
- Comprehensive test suite: FIPS 197 Appendix B and C.1/C.2/C.3 known-answer tests, S-box properties (bijection, no fixed points, spot-check values), key schedule validation, round-trip tests for all key sizes, and avalanche effect test.
- Coverage >80%.
