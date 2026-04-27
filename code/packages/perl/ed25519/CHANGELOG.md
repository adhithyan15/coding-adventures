# Changelog

All notable changes to this package will be documented in this file.

## [0.01] - 2026-04-13

### Added
- Initial implementation of Ed25519 digital signatures (RFC 8032)
- Key generation from 32-byte seed via SHA-512 and scalar clamping
- Deterministic signing using prefix-based nonce derivation
- Signature verification via S*B == R + k*A check
- Extended twisted Edwards coordinate point arithmetic using Math::BigInt
- All RFC 8032 Section 7.1 test vectors passing (verified against libsodium)
- Hex encoding/decoding utilities
