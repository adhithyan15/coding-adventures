# Changelog

All notable changes to this project will be documented in this file.

## [0.1.0] - 2026-04-12

### Added

- Initial implementation of X25519 (RFC 7748) elliptic curve Diffie-Hellman
- `x25519/2` — scalar multiplication over Curve25519
- `x25519_base/1` — multiply by the standard base point (u=9)
- `generate_keypair/1` — derive public key from private key
- Montgomery ladder with projective coordinates
- Field arithmetic over GF(2^255-19) using native Elixir big integers
- Modular exponentiation via square-and-multiply for field inversion
- Full test suite with all RFC 7748 test vectors including 1000-iteration test
- 96%+ test coverage
