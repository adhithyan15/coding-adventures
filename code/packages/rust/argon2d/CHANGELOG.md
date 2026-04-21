# Changelog — coding_adventures_argon2d

## 0.1.0 — 2026-04-20

### Added
- Initial implementation of Argon2d (RFC 9106) for Rust.
- `argon2d` — returns `Result<Vec<u8>, Argon2Error>`.
- `argon2d_hex` — returns `Result<String, Argon2Error>` (lowercase hex).
- `Options` struct with `key`, `associated_data`, `version` fields.
- Implements Argon2 v1.3 (0x13) only (the live RFC version).
- RFC 9106 §5.1 gold-standard vector plus 15 parameter-edge tests (16 total).
