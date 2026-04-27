# Changelog

All notable changes to this package will be documented in this file.

## [0.01] - 2026-04-20

### Added

- Initial pure-Perl port of Argon2id (RFC 9106) — hybrid variant and
  the RFC-recommended default for password hashing.
- `argon2id(...)` — raw binary tag.
- `argon2id_hex(...)` — lowercase hex tag.
- Validation of RFC 9106 parameter bounds (salt ≥ 8 bytes, tag ≥ 4 bytes,
  parallelism in [1, 2²⁴-1], memory ≥ 8·parallelism, 32-bit length caps).
- Test suite with the RFC 9106 §5.3 gold-standard vector and
  parameter-edge coverage.
