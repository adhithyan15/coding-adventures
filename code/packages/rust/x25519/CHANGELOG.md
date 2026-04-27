# Changelog

## 0.1.0 — 2026-04-12

### Added
- Initial implementation of X25519 (RFC 7748)
- Field arithmetic over GF(2^255 - 19) using radix-2^51 limbs
- Montgomery ladder with constant-time conditional swap
- Optimized squaring with doubled cross-terms
- Fermat inversion via addition chain
- `x25519()` — core scalar multiplication
- `x25519_base()` — multiplication by base point u=9
- `generate_keypair()` — public key derivation
- Full test suite with RFC 7748 test vectors
- Iterated test (1 and 1000 iterations)
