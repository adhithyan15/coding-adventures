# Changelog

All notable changes to this package are documented in this file.

## [0.1.0] — 2026-04-18

### Added
- Initial from-scratch BLAKE2b (RFC 7693) implementation in pure Go.
- `Sum`, `SumHex`: one-shot hash returning raw bytes or lowercase hex.
- `Hasher`: streaming hasher with `Update`, `Digest`, `HexDigest`, and
  `Copy` methods. The digest is non-destructive.
- `New(digestSize, key, salt, personal)` constructor with validation
  for digest size (1..64), key length (0..64), and 16-byte
  salt/personal lengths.
- Known-answer test suite cross-validated against Python's
  `hashlib.blake2b` (the authoritative reference implementation),
  covering:
  - Block-boundary message lengths: 0, 1, 63, 64, 65, 127, 128, 129,
    255, 256, 257, 1024, 4096, 9999.
  - Truncated digest sizes (1, 16, 20, 32, 48, 64).
  - Keyed mode across key lengths 1, 16, 32, 64.
  - Salt plus personalization.
  - Streaming: single-chunk, byte-by-byte, block-boundary, exact-block-
    then-more (the canonical BLAKE2 off-by-one), idempotent digest,
    update-after-digest, independent copy.
  - Invalid parameter rejection.
- No external dependencies; all KATs baked in as literals.

### Notes
- Sequential mode only. Tree hashing, BLAKE2s, BLAKE2bp, BLAKE2sp,
  BLAKE2Xb (XOF), and BLAKE3 are out of scope per the HF06 spec.
