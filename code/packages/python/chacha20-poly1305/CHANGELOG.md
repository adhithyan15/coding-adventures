# Changelog

All notable changes to this package will be documented in this file.

## [Unreleased]

### Added

- Consume the versioned SE04 cross-language fixture for byte-identical
  HChaCha20, raw XChaCha20, AEAD, and authentication-failure conformance.

## [0.2.0] - 2026-08-14

### Added

- HChaCha20 subkey derivation from SE04 and `draft-irtf-cfrg-xchacha-03`.
- Raw XChaCha20 and XChaCha20-Poly1305 authenticated encryption with 24-byte
  nonces, delegating to the existing RFC 8439 implementation.
- Gold-vector, authentication-failure, invalid-length, empty-message,
  multi-block, and raw stream-cipher conformance tests.

### Changed

- Documented that 24-byte nonces make random collisions negligible but must
  still be unique for each key.

## [0.1.0] - 2026-04-12

### Added

- ChaCha20 stream cipher (256-bit key, 96-bit nonce, 32-bit counter)
- Poly1305 one-time MAC (16-byte authentication tag)
- AEAD authenticated encryption/decryption (RFC 8439 Section 2.8)
- Full RFC 8439 test vector coverage (Sections 2.4.2, 2.5.2, 2.8.2)
- Constant-time tag comparison for side-channel resistance
- Literate programming style with extensive inline documentation
