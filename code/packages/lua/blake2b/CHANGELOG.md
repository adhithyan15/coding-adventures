# Changelog

All notable changes to this package are documented in this file.

## [0.1.0] — 2026-04-19

### Added
- Initial from-scratch BLAKE2b (RFC 7693) implementation in pure Lua.
  No external dependencies, Lua 5.3+ required for 64-bit integers.
- `blake2b.digest(msg, opts)` and `blake2b.hex(msg, opts)` one-shot
  APIs (also aliased as `blake2b.blake2b` and `blake2b.blake2b_hex`).
- `blake2b.Hasher` streaming value with `update`, `digest`, `hex_digest`,
  and `copy`.  Digest is non-destructive.
- Options: `digest_size` (1..64, default 64), `key` (0..64 bytes),
  `salt` (exactly 0 or 16 bytes), `personal` (exactly 0 or 16 bytes).
- Native Lua 64-bit integer arithmetic with wrap-on-overflow addition
  and logical `>>` right shift — no 64-bit emulation, no C extensions.
- Test suite mirrors the Python / Go / TypeScript / Rust / Ruby /
  Elixir / Swift KAT tables: block boundaries (0, 1, 63, 64, 65, 127,
  128, 129, 255, 256, 257, 1024, 4096, 9999), variable digest sizes,
  keyed mode across 1/16/32/64-byte keys, salt+personal, streaming
  including the canonical exact-block-then-more off-by-one, idempotent
  digest, update-after-digest, independent copy, and invalid parameter
  rejection.

### Notes
- Sequential mode only. Tree hashing, BLAKE2s, BLAKE2bp, BLAKE2sp,
  BLAKE2Xb, and BLAKE3 are out of scope per the HF06 spec.
