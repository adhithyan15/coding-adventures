# Changelog — coding_adventures_argon2d

## 0.1.0 — 2026-04-20

### Added
- Initial implementation of Argon2d (RFC 9106) for Ruby.
- `Argon2d.argon2d` / `Argon2d.argon2d_hex` — binary tag or hex string.
- Keyword options: `key`, `associated_data`, `version`.
- Implements Argon2 v1.3 (0x13) only.
- RFC 9106 §5.1 gold-standard vector plus 18 parameter-edge tests (19 total).
