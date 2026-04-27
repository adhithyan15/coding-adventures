# Changelog

All notable changes to this package are documented in this file.

## [0.1.0] — 2026-04-18

### Added
- Initial from-scratch BLAKE2b (RFC 7693) implementation in Rust. No
  external runtime or test dependencies.
- `blake2b(data, opts)` and `blake2b_hex(data, opts)` one-shot APIs.
- `Blake2bHasher` streaming type with `update`, `digest`, `hex_digest`,
  and `copy`. Digest is non-destructive.
- `Blake2bOptions` builder with chainable setters for `digest_size`
  (1..=64), `key` (0..=64 bytes), `salt` (16 bytes or empty), and
  `personal` (16 bytes or empty).
- `Blake2bError` enum with specific variants for each validation
  failure, displaying human-readable messages.
- Native `u64` arithmetic via `wrapping_add` and `rotate_right`; no
  `unsafe` (`#![forbid(unsafe_code)]`).
- Test suite mirrors the Python / Go / TypeScript KAT tables. Covers
  block boundaries (0, 1, 63, 64, 65, 127, 128, 129, 255, 256, 257,
  1024, 4096, 9999), variable digest sizes, keyed mode across
  1/16/32/64-byte keys, salt+personal, streaming across block boundaries
  including the canonical exact-block-then-more off-by-one, idempotent
  digest, update-after-digest, independent copy, and invalid parameter
  rejection.

### Notes
- Sequential mode only. Tree hashing, BLAKE2s, BLAKE2bp, BLAKE2sp,
  BLAKE2Xb, and BLAKE3 are out of scope per the HF06 spec.
