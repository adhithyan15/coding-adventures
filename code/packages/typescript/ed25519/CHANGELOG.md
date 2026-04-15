# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-13

### Added
- Initial implementation of Ed25519 digital signatures (RFC 8032)
- `generateKeypair(seed)` — derive public/secret key from 32-byte seed
- `sign(message, secretKey)` — deterministic Ed25519 signing
- `verify(message, signature, publicKey)` — signature verification
- Full field arithmetic over GF(2^255-19) using native BigInt
- Extended coordinates point representation for efficient curve operations
- Point compression/decompression (32-byte encoded points)
- Tested against all RFC 8032 Section 7.1 test vectors
- Literate programming style with detailed mathematical explanations
