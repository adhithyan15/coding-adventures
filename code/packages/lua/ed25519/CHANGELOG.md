# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-13

### Added
- Initial implementation of Ed25519 digital signatures (RFC 8032)
- Key generation from 32-byte seed via SHA-512 and scalar clamping
- Deterministic signing using prefix-based nonce derivation
- Signature verification via S*B == R + k*A check
- Extended twisted Edwards coordinate point arithmetic
- Big integer arithmetic with 30-bit limbs for Lua 5.4
- Fast field reduction mod p = 2^255 - 19
- Scalar arithmetic mod group order L
- All four RFC 8032 Section 7.1 test vectors passing
- Hex encoding/decoding utilities
