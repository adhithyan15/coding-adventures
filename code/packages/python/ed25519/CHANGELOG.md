# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-13

### Added
- Initial implementation of Ed25519 digital signatures (RFC 8032)
- Field arithmetic in GF(2^255-19): inversion, square root
- Extended twisted Edwards curve point operations: add, double, scalar multiply
- Point encoding (compression) and decoding (decompression)
- Key generation from 32-byte seed via SHA-512 and clamping
- Deterministic signing (no random nonce needed)
- Signature verification via the equation S*B == R + k*A
- All four RFC 8032 Section 7.1 test vectors pass
- Literate programming style with detailed mathematical explanations
