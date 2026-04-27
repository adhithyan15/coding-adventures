# Changelog

## 0.1.0 — 2026-04-12

### Added
- Initial implementation of AES modes of operation
- ECB (Electronic Codebook) mode --- encrypt/decrypt with PKCS#7 padding
- CBC (Cipher Block Chaining) mode --- IV-based chaining with PKCS#7 padding
- CTR (Counter Mode) --- stream cipher mode with 12-byte nonce + 4-byte counter
- GCM (Galois/Counter Mode) --- authenticated encryption with GF(2^128) GHASH
- PKCS#7 padding and unpadding utilities
- Comprehensive tests using NIST SP 800-38A and NIST GCM specification test vectors
