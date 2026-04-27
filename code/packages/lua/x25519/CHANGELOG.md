# Changelog

All notable changes to this project will be documented in this file.

## [0.1.0] - 2026-04-12

### Added

- Initial implementation of X25519 (RFC 7748) elliptic curve Diffie-Hellman
- `x25519(scalar, u_point)` — scalar multiplication over Curve25519
- `x25519_base(scalar)` — multiply by the standard base point (u=9)
- `generate_keypair(private_key)` — derive public key from private key
- Custom big integer library using 30-bit limbs for GF(2^255-19) arithmetic
- Fast modular reduction exploiting the special form of p = 2^255 - 19
- Montgomery ladder with projective coordinates
- Optimized field inversion via addition chain for a^(p-2)
- Full test suite with all RFC 7748 test vectors including 1000-iteration test
- Hex encoding/decoding utilities
