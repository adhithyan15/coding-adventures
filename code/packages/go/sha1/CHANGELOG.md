# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-03-22

### Added

- Full SHA-1 implementation from scratch (FIPS 180-4) with literate-programming commentary
- One-shot API: `Sum1(data []byte) [20]byte` and `HexString(data []byte) string`
- Streaming API: `Digest` struct with `Write()`, `Sum1()`, `HexDigest()` methods; `New()` constructor
- Named `Sum1` (not `Sum`) to avoid clashing with stdlib `crypto/sha1.Sum`
- 30+ tests covering FIPS 180-4 vectors, block boundaries, edge cases, and streaming
- Knuth-style explanations for every function, constant, and algorithm step
