# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-10

### Added

- DT17 hash-functions port in Rust
- FNV-1a 32-bit and 64-bit implementations with known-vector coverage
- DJB2 and polynomial rolling hash implementations
- MurmurHash3 32-bit implementation with seed support
- SipHash-2-4 implementation with 128-bit key support
- `HashFunction` trait plus concrete strategy structs for composition-style use
- `avalanche_score` and `distribution_test` analysis helpers
- Deterministic tests for the analysis helpers and known-vector tests for all algorithms
