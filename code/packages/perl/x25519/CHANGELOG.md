# Changelog

All notable changes to this project will be documented in this file.

## [0.01] - 2026-04-12

### Added

- Initial implementation of X25519 (RFC 7748) elliptic curve Diffie-Hellman
- `x25519()` — scalar multiplication over Curve25519
- `x25519_base()` — multiply by the standard base point (u=9)
- `generate_keypair()` — derive public key from private key
- Montgomery ladder with projective coordinates
- Field arithmetic over GF(2^255-19) using core Math::BigInt
- Full test suite with all RFC 7748 test vectors including 1000-iteration test
- POD documentation
