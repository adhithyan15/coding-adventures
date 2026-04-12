# Changelog — coding-adventures-scrypt

All notable changes to this package are documented here.

## [0.1.0] — 2026-04-11

### Added

- Initial implementation of the scrypt memory-hard key derivation function
  (RFC 7914).
- `scrypt(password, salt, n, r, p, dk_len) -> bytes` — public API
- `scrypt_hex(password, salt, n, r, p, dk_len) -> str` — hex-string variant
- Internal `_salsa20_8(data) -> bytes` — 64-byte Salsa20/8 permutation
  (RFC 7914 § 3), implemented with 4 double-rounds (column + row quarter-rounds)
- Internal `_block_mix(blocks, r) -> list` — BlockMix function (RFC 7914 § 4),
  applies Salsa20/8 iteratively and reorders even/odd outputs
- Internal `_ro_mix(b_bytes, n, r) -> bytes` — ROMix function (RFC 7914 § 5),
  fills an N-entry V table then does N pseudorandom lookups
- Internal `_pbkdf2_sha256(password, salt, iterations, key_length) -> bytes` —
  PBKDF2-SHA256 that allows empty passwords (needed for RFC vector 1)
- Internal `_hmac_sha256_raw(key, message) -> bytes` — HMAC-SHA256 with no
  empty-key restriction, calling `sha256` directly
- RFC 7914 test vectors 1 and 2 pass
- Full parameter validation with clear error messages
- Literate programming comments throughout (Knuth style)
- `py.typed` marker for mypy compatibility
- `required_capabilities.json` (empty — no special capabilities needed)
