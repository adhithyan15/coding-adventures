# Changelog

All notable changes to this package will be documented in this file.

## [0.01] - 2026-04-20

### Added

- Initial pure-Perl port of Argon2d (RFC 9106).
- `argon2d(...)` — raw binary tag.
- `argon2d_hex(...)` — lowercase hex tag.
- Validation of RFC 9106 parameter bounds (salt ≥ 8 bytes, tag ≥ 4 bytes,
  parallelism in [1, 2²⁴-1], memory ≥ 8·parallelism, 32-bit length caps on
  password/salt/key/associated_data/memory/tag).
- Test suite with the RFC 9106 §5.1 gold-standard vector plus
  parameter-edge coverage for tag length, key binding, associated-data
  binding, multi-lane, and multi-pass cases.
