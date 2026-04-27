# Changelog

## [0.1.0] - 2026-04-12

### Added
- `desEncryptBlock` / `desDecryptBlock` — DES single-block encrypt/decrypt (FIPS 46-3)
- `expandKey` — 16-round key schedule (PC-1, PC-2, rotation schedule)
- `desECBEncrypt` / `desECBDecrypt` — ECB mode with PKCS#7 padding
- `tdeaEncryptBlock` / `tdeaDecryptBlock` — 3DES EDE per NIST SP 800-67
- FIPS/NIST known-answer test vectors, round-trip tests, avalanche effect tests
