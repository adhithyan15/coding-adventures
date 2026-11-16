# Changelog

All notable changes to this package will be documented in this file.

## [Unreleased]

### Added

- Draft Warrant document `code/specs/hash-functions.warrant` and contract
  `code/specs/DT17a-hash-functions-warrant.md` (WX-P1a) for the eight byte-string
  hashes. Documentation only: no code, test, BUILD or dependency changed, and no
  evidence has been generated (the generator has no negative control for a
  bytes-to-integer operation; see DT17a section 5).

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
