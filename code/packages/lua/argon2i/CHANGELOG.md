# Changelog — coding-adventures-argon2i

## 0.1.0 — 2026-04-20

### Added
- Initial pure-Lua implementation of Argon2i (RFC 9106).
- `argon2i.argon2i(password, salt, t, m, p, T[, opts])` and
  `argon2i.argon2i_hex(...)`.
- Options: `key`, `associated_data`, `version`.
- Implements Argon2 v1.3 (0x13) only.
- RFC 9106 §5.2 gold-standard vector plus 18 parameter-edge tests
  (19 total).
