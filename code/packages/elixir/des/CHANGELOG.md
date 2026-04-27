# Changelog — coding_adventures_des (Elixir)

All notable changes to this package are documented here.

## [0.1.0] — 2026-04-11

### Added

- Initial implementation of `CodingAdventures.Des` (SE01).
- `expand_key/1` — derives 16 DES round subkeys (each 6 bytes) from an 8-byte key via PC-1, left rotations, and PC-2.
- `des_encrypt_block/2` — encrypt a single 8-byte block; validates key and block length.
- `des_decrypt_block/2` — decrypt a single 8-byte block (reversed subkeys).
- `des_ecb_encrypt/2` — ECB-mode encryption with PKCS#7 padding.
- `des_ecb_decrypt/2` — ECB-mode decryption with PKCS#7 unpadding.
- `tdea_encrypt_block/4` — Triple DES EDE encrypt: E_K1(D_K2(E_K3(P))).
- `tdea_decrypt_block/4` — Triple DES EDE decrypt: D_K3(E_K2(D_K1(C))).
- Full FIPS 46-3 / SP 800-20 test vectors (plaintext-variable and key-variable tables).
- NIST SP 800-67 TDEA test vector verified.
- TDEA backward-compatibility: K1=K2=K3 reduces to single DES.
- Literate-programming inline comments explaining Feistel networks, S-boxes, and key schedule.
