# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-12

### Added

- Initial implementation of X25519 (RFC 7748)
- Custom multi-precision arithmetic using UInt64 limbs
- Field arithmetic over GF(2^255-19) with fast reduction
- Montgomery ladder scalar multiplication
- Scalar clamping per RFC 7748
- `x25519(scalar:u:)` -- generic scalar multiplication
- `x25519Base(scalar:)` -- base point multiplication
- `generateKeypair(privateKey:)` -- public key generation
- Full test suite with all RFC 7748 test vectors
- Iterated test (1 and 1000 iterations)
- Diffie-Hellman shared secret verification
