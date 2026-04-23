# Changelog

## 0.1.0 — 2026-04-12

### Added
- Initial implementation of X25519 (RFC 7748)
- Field arithmetic over GF(2^255 - 19) using math/big
- Montgomery ladder with conditional swap
- `X25519()` — core scalar multiplication
- `X25519Base()` — multiplication by base point u=9
- `GenerateKeypair()` — public key derivation
- Full test suite with RFC 7748 test vectors
- Iterated test (1 and 1000 iterations)
