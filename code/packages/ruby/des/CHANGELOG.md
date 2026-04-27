# Changelog — coding_adventures_des

## [0.1.0] — 2026-04-11

### Added

- Initial implementation of DES block cipher (FIPS 46-3).
- `Des.expand_key(key)`: 16 round subkeys from PC-1/PC-2 key schedule.
- `Des.des_encrypt_block(block, key)`: IP → 16 Feistel rounds → FP.
- `Des.des_decrypt_block(block, key)`: reversed subkey order.
- `Des.des_ecb_encrypt(plain, key)`: ECB mode with PKCS#7 padding.
- `Des.des_ecb_decrypt(cipher, key)`: ECB decryption with unpadding.
- `Des.tdea_encrypt_block(block, k1, k2, k3)`: Triple DES EDE encrypt.
- `Des.tdea_decrypt_block(block, k1, k2, k3)`: Triple DES EDE decrypt.
- Comprehensive test suite with FIPS/SP 800-20 vectors, round-trip tests, ECB edge cases, and 3DES backward compatibility.
