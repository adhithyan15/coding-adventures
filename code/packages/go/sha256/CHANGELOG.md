# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-05

### Added
- Initial implementation of SHA-256 (FIPS 180-4) from scratch
- One-shot API: `Sum256(data)` returning `[32]byte` digest
- Hex convenience: `HexString(data)` returning 64-char lowercase hex string
- Streaming API: `Digest` struct with `Write()`, `Sum256()`, `HexDigest()`, `Copy()`
- Non-destructive `Sum256()` that does not consume internal state
- Deep `Copy()` for branching hash computations
- Full FIPS 180-4 test vectors (empty, "abc", 56-byte, million-a)
- Block boundary tests (55, 56, 63, 64, 119, 120, 127, 128 bytes)
- Avalanche effect test
- All 256 single-byte uniqueness test
- Streaming equivalence tests with various chunk sizes
- Operation-based capability cage pattern
