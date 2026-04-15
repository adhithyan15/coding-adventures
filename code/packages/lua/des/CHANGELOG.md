# Changelog — coding-adventures-des (Lua)

All notable changes to this package are documented here.

## [0.1.0] — 2026-04-11

### Added

- Initial implementation of `coding_adventures.des` (SE01).
- `expand_key(key)` — derives 16 × 6-byte subkeys from an 8-byte DES key.
- `des_encrypt_block(block, key)` — single-block DES encryption.
- `des_decrypt_block(block, key)` — single-block DES decryption.
- `des_ecb_encrypt(plaintext, key)` — ECB mode with PKCS#7 padding.
- `des_ecb_decrypt(ciphertext, key)` — ECB mode decryption.
- `tdea_encrypt_block(block, k1, k2, k3)` — Triple DES EDE encrypt.
- `tdea_decrypt_block(block, k1, k2, k3)` — Triple DES EDE decrypt.
- FIPS 46-3 / SP 800-20 known-answer test vectors.
- NIST SP 800-67 TDEA test vector verified.
- Backward-compatibility: K1=K2=K3 reduces to single DES.
