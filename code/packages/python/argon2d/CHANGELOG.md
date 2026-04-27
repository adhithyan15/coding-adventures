# Changelog

All notable changes to this package are documented in this file.

## [0.1.0] — 2026-04-19

### Added

- Initial pure-Python Argon2d (RFC 9106) implementation.  Depends on
  `coding-adventures-blake2b` for the outer BLAKE2b calls; the
  compression function's modified round is inlined.
- `argon2d` and `argon2d_hex` one-shot functions.
- Parameter validation (`salt` ≥ 8 B, `T` ≥ 4, `m` ≥ 8·p, `t` ≥ 1,
  `p` ∈ [1, 2²⁴−1], version = 0x13).
- Test suite mirroring the RFC 9106 §5.1 canonical vector plus
  parameter-edge cases (single-lane / multi-lane, multiple passes,
  tag-length variants across the H'-fold boundary).

### Notes

- Pure Python is comfortably two orders of magnitude slower than
  native-backed libraries; this port exists for educational and
  cross-language-verification purposes.
- For password hashing, prefer the Argon2id variant
  (`coding-adventures-argon2id`).
