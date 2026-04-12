# Changelog — coding_adventures_scrypt

All notable changes to this package are documented here.

## [0.1.0] — 2026-04-11

### Added

- Initial implementation of scrypt (RFC 7914).
- `scrypt(password, salt, n, r, p, dk_len)` — derive a raw byte key.
- `scrypt_hex(password, salt, n, r, p, dk_len)` — derive a lowercase hex key.
- `ScryptError` enum with variants: `InvalidN`, `NTooLarge`, `InvalidR`,
  `InvalidP`, `InvalidKeyLength`, `KeyLengthTooLarge`, `PRTooLarge`,
  `HmacError`.
- Internal `pbkdf2_sha256_internal` that allows empty passwords (bypasses the
  empty-key guard in `coding_adventures_pbkdf2` to support RFC 7914 vector 1).
- `salsa20_8` — Salsa20/8 core function with an RFC 7914 §8 test vector.
- `block_mix_general` — BlockMix for arbitrary `r`.
- `ro_mix` — ROMix with V-table memory hardness.
- `integerify` — little-endian u64 extraction from last ROMix block.
- Unit tests: parameter validation, Salsa20/8 vector, output length,
  determinism, sensitivity.
- Integration tests: RFC 7914 vectors 1 (fast), 2 and 3 (ignored/slow),
  scrypt_hex correctness, error cases.
- `BUILD` file: `cargo test -p coding_adventures_scrypt -- --nocapture`.
- `README.md` with algorithm description, usage examples, and design notes.
- `required_capabilities.json` (no external capabilities required).
