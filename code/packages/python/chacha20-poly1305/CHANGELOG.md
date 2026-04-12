# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-12

### Added

- ChaCha20 stream cipher (256-bit key, 96-bit nonce, 32-bit counter)
- Poly1305 one-time MAC (16-byte authentication tag)
- AEAD authenticated encryption/decryption (RFC 8439 Section 2.8)
- Full RFC 8439 test vector coverage (Sections 2.4.2, 2.5.2, 2.8.2)
- Constant-time tag comparison for side-channel resistance
- Literate programming style with extensive inline documentation
