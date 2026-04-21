# Changelog — coding_adventures_argon2id

## 0.1.0 — 2026-04-20

### Added
- Initial implementation of Argon2id (RFC 9106) for Rust.
- `argon2id` / `argon2id_hex` — `Result<Vec<u8>|String, Argon2Error>`.
- `Options` struct with `key`, `associated_data`, `version` fields.
- Implements Argon2 v1.3 (0x13) only.
- RFC 9106 §5.3 gold-standard vector plus 15 parameter-edge tests (16 total).
