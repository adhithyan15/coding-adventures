# Changelog — coding_adventures_des (Rust)

## [0.1.0] — 2026-04-11

### Added

- Initial implementation of DES block cipher (FIPS 46-3) in Rust.
- `expand_key` — derives 16 × 6-byte round subkeys from an 8-byte DES key using
  PC-1, PC-2, and left-rotation schedule.
- `encrypt_block` / `decrypt_block` — single 8-byte block encryption and
  decryption via the 16-round Feistel network.
- `ecb_encrypt` / `ecb_decrypt` — ECB mode with PKCS#7 padding for
  variable-length data (educational use only; ECB is insecure for real data).
- `tdea_encrypt_block` / `tdea_decrypt_block` — Triple DES (3TDEA) in
  Encrypt-Decrypt-Encrypt (EDE) ordering: C = E_K1(D_K2(E_K3(P))).
- All FIPS 46-3 / NIST SP 800-20 known-answer test vectors pass.
- NIST SP 800-67 TDEA test vector passes.
- Backward-compatibility property: K1=K2=K3 reduces 3DES to single DES.
