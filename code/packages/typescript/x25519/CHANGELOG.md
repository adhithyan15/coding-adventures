# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-12

### Added

- Initial implementation of X25519 (RFC 7748)
- Field arithmetic over GF(2^255-19) using native BigInt
- Montgomery ladder scalar multiplication
- Scalar clamping per RFC 7748
- `x25519()` — generic scalar multiplication
- `x25519Base()` — base point multiplication
- `generateKeypair()` — public key generation
- Full test suite with all RFC 7748 test vectors
- Iterated test (1 and 1000 iterations)
- Diffie-Hellman shared secret verification
