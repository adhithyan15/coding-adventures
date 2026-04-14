# Changelog

## 0.1.0 (2026-04-13)

### Added
- Initial implementation of Ed25519 digital signatures (RFC 8032)
- Key generation from 32-byte seed via SHA-512 hashing and scalar clamping
- Deterministic signing using SHA-512-based nonce derivation
- Signature verification with full point decoding and malleability checks
- Extended twisted Edwards coordinate point arithmetic (addition, doubling, scalar multiplication)
- Field arithmetic over GF(2^255-19) using native arbitrary-precision integers
- Square root computation for point decoding (p = 5 mod 8 method)
- Hex encoding/decoding utilities
- RFC 8032 Section 7.1 test vectors (verified against libsodium)
- Verification edge case tests (wrong message, wrong key, tampered signature, invalid lengths)
- Round-trip sign/verify tests with various message lengths
