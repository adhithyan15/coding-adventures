# Changelog

All notable changes to this package are documented in this file.

## [0.1.0] — 2026-04-18

### Added
- Initial from-scratch BLAKE2b (RFC 7693) implementation in pure Python.
- `blake2b(data, digest_size=64, key=b"", salt=b"", personal=b"")`: one-shot
  hash returning raw bytes.
- `blake2b_hex(...)`: one-shot hash returning a lowercase hex string.
- `Blake2bHasher`: streaming hasher with `update`, `digest`, `hex_digest`,
  and `copy` methods.  The digest is non-destructive; additional updates
  remain valid after finalization.
- Argument validation for digest size (1..64), key length (0..64), and
  salt/personalization lengths (exactly 16 bytes when provided).
- Full test suite cross-validated against Python's `hashlib.blake2b`,
  covering message lengths at block boundaries (0, 1, 63, 64, 65, 127,
  128, 129, …), keyed mode with varied key lengths, salt and
  personalization, truncated digest sizes, streaming across block
  boundaries, and idempotence of `digest()`.
- Knuth-style literate commentary on the G quarter-round, compression
  function, parameter block, and the final-block-flag invariant that is
  the canonical BLAKE2 off-by-one source.

### Notes
- Sequential mode only.  Tree hashing, BLAKE2s, BLAKE2bp, BLAKE2sp,
  BLAKE2Xb (XOF), and BLAKE3 are out of scope per the HF06 spec.
- Empty input with no key returns the canonical 64-byte digest
  `786a02f7…`.  The compression function must always run at least once,
  even for the empty message.
