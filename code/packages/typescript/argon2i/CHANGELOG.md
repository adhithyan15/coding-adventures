# Changelog

## 0.1.0 — 2026-04-19

- Initial release: pure-TypeScript argon2i (RFC 9106).
- One-shot API: `argon2i(...)` returns raw bytes, `argon2iHex(...)` returns lowercase hex.
- Supports optional secret key (`K`) and associated data (`X`).
- Implements RFC 9106 v1.3 only (the live version).
- Tests: RFC 9106 §5 canonical vector plus parameter-edge suite.
