# Changelog

All notable changes to this package will be documented in this file.

## [0.01] - 2026-04-20

### Added

- Initial pure-Perl port of Argon2i (RFC 9106).
- `argon2i(...)` — raw binary tag.
- `argon2i_hex(...)` — lowercase hex tag.
- Validation of RFC 9106 parameter bounds (salt ≥ 8 bytes, tag ≥ 4 bytes,
  parallelism in [1, 2²⁴-1], memory ≥ 8·parallelism, 32-bit length caps).
- Test suite with the RFC 9106 §5.2 gold-standard vector and
  parameter-edge coverage.
