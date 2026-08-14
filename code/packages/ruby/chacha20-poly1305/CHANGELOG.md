# Changelog

All notable changes to this package will be documented in this file.

## [Unreleased]

### Added

- Consume the versioned SE04 cross-language fixture for byte-identical
  HChaCha20, raw XChaCha20, AEAD, and authentication-failure conformance.

## [0.2.0] - 2026-08-14

### Added

- HChaCha20 subkey derivation from the pinned SE04 construction
- Raw XChaCha20 with 24-byte nonces and caller-selected counters
- XChaCha20-Poly1305 authenticated encryption and decryption
- Exact draft HChaCha20 and Appendix A.3.1 vector coverage
- Negative, invalid-length, empty-message, and multi-block coverage

## [0.1.0] - 2026-04-12

### Added

- ChaCha20 stream cipher (256-bit key, 96-bit nonce, 32-bit counter)
- Poly1305 one-time MAC using Ruby's native big integer arithmetic
- AEAD authenticated encryption/decryption (RFC 8439 Section 2.8)
- Full RFC 8439 test vector coverage (Sections 2.4.2, 2.5.2, 2.8.2)
- Constant-time tag comparison for side-channel resistance
- Literate programming style with extensive inline documentation
