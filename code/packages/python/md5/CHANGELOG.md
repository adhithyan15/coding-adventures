# Changelog

All notable changes to this package will be documented in this file.

## [0.1.1] - 2026-03-23

### Changed

- Renamed Python module from `md5` to `ca_md5` to avoid future name conflicts
  and for consistency with the `ca_` prefix applied to all packages in this repo.
  Import: `from ca_md5 import md5, md5_hex, MD5`

## [0.1.0] - 2026-03-22

### Added

- Full MD5 implementation from scratch (RFC 1321) with literate-programming commentary
- T-table: 64 sine-derived constants computed at module load time
- CRITICAL: little-endian throughout — block parsing (`<16I`) and output (`<4I`)
- One-shot API: `md5(data: bytes) -> bytes` and `md5_hex(data: bytes) -> str`
- Streaming API: `MD5` class with `update()`, `digest()`, `hexdigest()`, `copy()` methods
- 42 tests covering RFC 1321 vectors, little-endian verification, block boundaries, edge cases, streaming
- 100% test coverage
- Knuth-style explanations for every function, the T-table derivation, and the little-endian gotcha
