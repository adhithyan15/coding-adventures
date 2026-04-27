# Changelog — coding-adventures-argon2id

## 0.1.0 — 2026-04-21

### Added
- Initial pure-Lua implementation of Argon2id (RFC 9106).
- `argon2id.argon2id(password, salt, t, m, p, T[, opts])` and
  `argon2id.argon2id_hex(...)`.
- Options: `key`, `associated_data`, `version`.
- Implements Argon2 v1.3 (0x13) only.
- RFC 9106 §5.3 gold-standard vector plus 18 parameter-edge tests
  (19 total).
