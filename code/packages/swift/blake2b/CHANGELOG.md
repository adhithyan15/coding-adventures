# Changelog

All notable changes to this package are documented in this file.

## [0.1.0] — 2026-04-18

### Added
- Initial from-scratch BLAKE2b (RFC 7693) implementation in Swift. No
  external dependencies, no `unsafe`.
- `Blake2b.hash(_:options:)` and `Blake2b.hashHex(_:options:)` one-shot
  APIs.
- `Blake2b.Hasher` streaming value type with `update`, `digest`,
  `hexDigest`, and `copy`. Digest is non-destructive. `Hasher` is a
  struct so `copy()` is a structural copy.
- `Blake2b.Options` struct with `digestSize`, `key`, `salt`, `personal`.
- `Blake2b.ValidationError` enum with specific cases per validation
  failure.
- Native `UInt64` with `&+` wrapping add and a small `UInt128Emulated`
  counter struct to match the RFC's reserved 128-bit byte-count field.
- Test suite mirrors the Python / Go / TypeScript / Rust / Ruby /
  Elixir KAT tables: block boundaries (0, 1, 63, 64, 65, 127, 128, 129,
  255, 256, 257, 1024, 4096, 9999), variable digest sizes, keyed mode
  across 1/16/32/64-byte keys, salt+personal, streaming including the
  canonical exact-block-then-more off-by-one, idempotent digest,
  update-after-digest, independent copy, and invalid parameter
  rejection.

### Notes
- Sequential mode only. Tree hashing, BLAKE2s, BLAKE2bp, BLAKE2sp,
  BLAKE2Xb, and BLAKE3 are out of scope per the HF06 spec.
