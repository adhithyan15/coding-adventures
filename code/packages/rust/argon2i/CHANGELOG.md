# Changelog — coding_adventures_argon2i

## 0.1.0 — 2026-04-20

### Added
- Initial implementation of Argon2i (RFC 9106) for Rust.
- `argon2i` / `argon2i_hex` — `Result<Vec<u8>|String, Argon2Error>`.
- `Options` struct with `key`, `associated_data`, `version` fields.
- Implements Argon2 v1.3 (0x13) only.
- RFC 9106 §5.2 gold-standard vector plus 15 parameter-edge tests (16 total).
