# Changelog

## 0.1.0 — 2026-04-12

### Added
- Initial implementation of AES modes of operation in Go
- ECB (Electronic Codebook) mode with PKCS#7 padding
- CBC (Cipher Block Chaining) mode with IV and PKCS#7 padding
- CTR (Counter Mode) with 12-byte nonce and 4-byte counter
- GCM (Galois/Counter Mode) with GF(2^128) GHASH authentication
- Comprehensive tests with NIST SP 800-38A and NIST GCM specification vectors
