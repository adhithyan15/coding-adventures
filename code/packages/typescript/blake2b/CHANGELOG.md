# Changelog

All notable changes to this package are documented in this file.

## [0.1.0] — 2026-04-18

### Added
- Initial from-scratch BLAKE2b (RFC 7693) implementation in TypeScript.
- `blake2b(data, options)` and `blake2bHex(data, options)` one-shot APIs.
- `Blake2bHasher` streaming class with `update`, `digest`, `hexDigest`,
  and `copy`. Digest is non-destructive.
- `Blake2bOptions` accepts `digestSize` (1..64), `key` (0..64 bytes),
  `salt` (16 bytes or empty), and `personal` (16 bytes or empty).
- BigInt-based 64-bit arithmetic. The mix of readability and correctness
  trumps the ~3× speedup of a two-32-bit-word emulation for an
  educational implementation.
- Test suite mirrors the Python and Go packages' KATs, precomputed
  against Python's `hashlib.blake2b`. Covers block boundaries (0, 1,
  63, 64, 65, 127, 128, 129, 255, 256, 257, 1024, 4096, 9999),
  variable digest sizes, keyed mode across 1/16/32/64-byte keys,
  salt+personal, streaming across block boundaries including the
  canonical exact-block-then-more off-by-one, idempotent digest,
  update-after-digest, independent copy, and invalid parameter
  rejection.

### Notes
- Sequential mode only. Tree hashing, BLAKE2s, BLAKE2bp, BLAKE2sp,
  BLAKE2Xb, and BLAKE3 are out of scope per the HF06 spec.
