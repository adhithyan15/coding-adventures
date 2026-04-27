# Changelog

All notable changes to this package are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] — 2026-04-20

### Added

- Initial pure-Haskell Argon2d implementation following RFC 9106.
- `argon2d` / `argon2dHex` high-level API returning the raw tag or a
  lowercase-hex string.
- `argon2Version` constant for the only approved v1.3 (0x13) version.
- G-mixer with the full BLAKE2 quarter-round plus Argon2's
  `2 * trunc32(a) * trunc32(b)` cross-term, using native `Word64`
  wrap-on-overflow.
- H' variable-length hash (RFC §3.3) with correct chain length —
  `r = ceil(t/32) - 2` 64-byte blocks then a final `t - 32r`-byte
  block.
- Data-dependent `fillSegment`: reference-block indices derived from
  the previous block's first 64 bits.
- `index_alpha` biasing (RFC §3.4.1.1) toward more recently computed
  blocks.
- Input validation for password/salt/key/AD length (32-bit cap),
  parallelism in `[1, 2^24-1]`, `memory_cost >= 8*parallelism`,
  `time_cost >= 1`, `tag_length >= 4`, and version exactly `0x13`.
- Hspec test suite covering the RFC 9106 §5.1 canonical vector,
  validation rejections, determinism, input-binding, tag-length
  variants (4 / 16 / 65 / 128 bytes) and multi-pass correctness.
