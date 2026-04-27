# Changelog — coding-adventures-aes (Lua)

All notable changes to this package are documented here.

## [0.1.0] — 2026-04-11

### Added

- Initial implementation of `coding_adventures.aes` (SE02).
- `aes_encrypt_block(block, key)` — AES-128/192/256 block encryption (FIPS 197).
- `aes_decrypt_block(block, key)` — AES block decryption.
- `expand_key(key)` — key schedule, returns 11/13/15 round keys.
- `SBOX` and `INV_SBOX` — exported 256-entry lookup tables.
- GF(2^8) arithmetic inline (xtime, gf_mul) with polynomial 0x11B.
- FIPS 197 Appendix B and C test vectors (AES-128, AES-192, AES-256).
- SP 800-38A AES-256 test vector.
