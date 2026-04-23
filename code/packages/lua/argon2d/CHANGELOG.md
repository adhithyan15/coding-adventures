# Changelog — coding-adventures-argon2d

## 0.1.0 — 2026-04-20

### Added
- Initial pure-Lua implementation of Argon2d (RFC 9106).
- `argon2d.argon2d(password, salt, t, m, p, T[, opts])` and
  `argon2d.argon2d_hex(...)`.
- Options: `key`, `associated_data`, `version`.
- Implements Argon2 v1.3 (0x13) only.
- RFC 9106 §5.1 gold-standard vector plus 18 parameter-edge tests
  (19 total).
