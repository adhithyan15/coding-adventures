# Changelog

All notable changes to this project will be documented in this file.

## [0.1.0] - 2026-04-13

### Added
- Initial implementation of Ed25519 digital signatures (RFC 8032)
- Custom multi-precision integer arithmetic using [UInt64] limb arrays
- Key generation from 32-byte seeds via SHA-512
- Deterministic signing (Schnorr-like with EdDSA nonce derivation)
- Signature verification via the equation S*B == R + k*A
- Extended coordinates (X, Y, Z, T) for efficient point arithmetic
- Unified point addition formula (complete, no special cases)
- Point encoding/decoding with y-coordinate compression
- Field square root using Atkin algorithm (p = 5 mod 8)
- Tests against RFC 8032 Section 7.1 test vectors (vectors 1-3)
- Verification failure tests (tampered message, wrong key, bad signature)
- Round-trip sign/verify tests for various message lengths
