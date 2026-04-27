# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-13

### Added

- Initial implementation of Ed25519 (RFC 8032) digital signatures
- Field arithmetic in GF(2^255 - 19) using radix-2^51 limbs
- Extended twisted Edwards point operations (add, double, scalar multiply)
- Point encoding and decoding (32-byte compressed format)
- Key generation from 32-byte seed
- Deterministic signing (SHA-512-based nonce derivation)
- Signature verification
- Multi-precision scalar arithmetic mod L (group order)
- All four RFC 8032 Section 7.1 test vectors
- Field arithmetic tests (inverse, sqrt, sqrt(-1))
- Point operation tests (identity, doubling, order)
- Verification rejection tests (wrong message, tampered sig, S >= L)
