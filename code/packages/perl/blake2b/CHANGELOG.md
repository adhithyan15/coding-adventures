# Changelog

All notable changes to this package are documented in this file.

## [0.01] — 2026-04-19

### Added
- Initial from-scratch BLAKE2b (RFC 7693) implementation in pure Perl.
  No XS, no C extensions, no external CPAN crypto dependencies.
- `CodingAdventures::Blake2b::blake2b($data, %opts)` and
  `blake2b_hex($data, %opts)` one-shot functions.
- `CodingAdventures::Blake2b->new(%opts)` streaming hasher with
  `update`, `digest`, `hex_digest`, and `copy`.  `digest` is non-
  destructive; `update` is callable after `digest`.
- Options: `digest_size` (1..64, default 64), `key` (0..64 bytes),
  `salt` (exactly 0 or 16 bytes), `personal` (exactly 0 or 16 bytes).
- All 64-bit addition is wrapped in `use integer` and masked to stay
  within `[0, 2^64)`, sidestepping Perl's NV promotion for large
  unsigned sums.  Bitwise operators work on 64-bit values natively.
- Test suite mirrors the Python / Go / TypeScript / Rust / Ruby /
  Elixir / Swift / Lua KAT tables: block boundaries (0, 1, 63, 64,
  65, 127, 128, 129, 255, 256, 257, 1024, 4096, 9999), variable
  digest sizes, keyed mode across 1/16/32/64-byte keys, salt+personal,
  streaming including the canonical exact-block-then-more off-by-one,
  idempotent digest, update-after-digest, independent copy, and
  parameter validation.

### Notes
- Sequential mode only. Tree hashing, BLAKE2s, BLAKE2bp, BLAKE2sp,
  BLAKE2Xb, and BLAKE3 are out of scope per the HF06 spec.
- Requires a 64-bit Perl 5.26+ (`ivsize == 8`).
