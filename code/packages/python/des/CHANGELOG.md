# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-12

### Added
- `expand_key` — DES key schedule: 64-bit key → 16 × 48-bit round subkeys via PC-1, left rotations, and PC-2
- `des_encrypt_block` — single 64-bit block DES encryption (IP → 16 Feistel rounds → FP)
- `des_decrypt_block` — single 64-bit block DES decryption (reversed subkey order)
- `des_ecb_encrypt` / `des_ecb_decrypt` — ECB mode with PKCS#7 padding for variable-length data
- `tdea_encrypt_block` / `tdea_decrypt_block` — Triple DES EDE (Encrypt-Decrypt-Encrypt) for 3-key 3DES
- All permutation tables (IP, FP, PC-1, PC-2, E, P) and 8 S-boxes hardcoded as constants
- PKCS#7 padding and unpadding with full validation
- Comprehensive test suite: NIST FIPS 81, SP 800-20, SP 800-67 test vectors; round-trip tests; error handling
