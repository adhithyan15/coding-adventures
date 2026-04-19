# Changelog

All notable changes to this package are documented in this file.

## [0.1.0] — 2026-04-18

### Added
- Initial from-scratch BLAKE2b (RFC 7693) implementation in Elixir. No
  external runtime dependencies.
- `CodingAdventures.Blake2b.blake2b/2` and `.blake2b_hex/2` one-shot
  APIs with keyword-option configuration.
- `CodingAdventures.Blake2b.Hasher` struct and `new/1`, `update/2`,
  `digest/1`, `hex_digest/1`, `copy/1` streaming pipeline. Digest is
  non-destructive.
- Keyword options: `:digest_size` (1..64), `:key` (0..64 bytes),
  `:salt` (16 bytes or empty), `:personal` (16 bytes or empty).
- 64-bit arithmetic masked via `&&& 0xFFFFFFFFFFFFFFFF` throughout to
  keep arbitrary-precision integers from leaking past a single word.
- Test suite mirrors the Python / Go / TypeScript / Rust / Ruby KAT
  tables. Covers block boundaries (0, 1, 63, 64, 65, 127, 128, 129, 255,
  256, 257, 1024, 4096, 9999), variable digest sizes, keyed mode across
  1/16/32/64-byte keys, salt+personal, streaming across block boundaries
  including the canonical exact-block-then-more off-by-one, idempotent
  digest, update-after-digest, independent copy, and invalid parameter
  rejection.

### Notes
- Sequential mode only. Tree hashing, BLAKE2s, BLAKE2bp, BLAKE2sp,
  BLAKE2Xb, and BLAKE3 are out of scope per the HF06 spec.
