# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-12

### Added

- Initial implementation of X25519 (RFC 7748)
- Field arithmetic over GF(2^255-19) using Ruby native integers
- Montgomery ladder scalar multiplication
- Scalar clamping per RFC 7748
- `x25519` -- generic scalar multiplication
- `x25519_base` -- base point multiplication
- `generate_keypair` -- public key generation
- Full test suite with all RFC 7748 test vectors
- Iterated test (1 and 1000 iterations)
- Diffie-Hellman shared secret verification
