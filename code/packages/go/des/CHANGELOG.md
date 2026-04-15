# Changelog — go/des

All notable changes to this package will be documented in this file.

## [0.1.0] — 2026-04-11

### Added

- Initial implementation of DES (Data Encryption Standard) and Triple DES (3TDEA)
  conforming to FIPS 46-3 and NIST SP 800-67.
- `ExpandKey` — derives 16 round subkeys (each 6 bytes / 48 bits) from an 8-byte key
  using the PC-1 / PC-2 key schedule with left-rotation amounts per FIPS 46-3.
- `EncryptBlock` / `DecryptBlock` — raw 8-byte block cipher using 16-round Feistel network
  with IP, E, 8 S-boxes, P, and FP permutations. Decryption uses reversed subkeys.
- `ECBEncrypt` / `ECBDecrypt` — ECB mode with PKCS#7 padding for arbitrary-length data.
- `TDEAEncryptBlock` / `TDEADecryptBlock` — Triple DES EDE (Encrypt-Decrypt-Encrypt)
  ordering `C = E_K1(D_K2(E_K3(P)))` per SP 800-67. When K1=K2=K3, reduces to single DES.
- Comprehensive test suite with NIST FIPS 81 / SP 800-20 known-answer test vectors,
  round-trip tests across all API functions, ECB mode edge cases, TDEA backward
  compatibility, and error handling for invalid inputs.
