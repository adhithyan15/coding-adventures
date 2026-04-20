# Changelog

## 0.1.0 — 2026-04-20

- Initial release: pure-Go argon2id (RFC 9106).
- One-shot API: `Sum(...)` returns raw bytes, `SumHex(...)` returns lowercase hex.
- Supports optional secret key (`K`) and associated data (`X`) via `Options`.
- Implements RFC 9106 v1.3 only (the live version).
- Tests: RFC 9106 §5 canonical vector plus parameter-edge suite.
