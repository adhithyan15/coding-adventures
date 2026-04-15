# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-12

### Added
- ChaCha20 stream cipher (256-bit key, 96-bit nonce, 32-bit counter)
- Poly1305 one-time MAC (16-byte tag) with multi-limb big integer arithmetic
- AEAD combined authenticated encryption per RFC 8439 Section 2.8
- Constant-time tag comparison to prevent timing attacks
- All RFC 8439 test vectors passing
