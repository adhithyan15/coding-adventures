# Changelog — @coding-adventures/pbkdf2

## 0.1.0 — 2026-04-11

### Added
- Initial implementation of PBKDF2 (RFC 8018) for TypeScript.
- `pbkdf2HmacSHA1`, `pbkdf2HmacSHA256`, `pbkdf2HmacSHA512` — return `Uint8Array`.
- `pbkdf2HmacSHA1Hex`, `pbkdf2HmacSHA256Hex`, `pbkdf2HmacSHA512Hex` — return `string`.
- Validates empty password, non-positive/non-integer iterations, and non-positive/non-integer keyLength.
- Full RFC 6070 test vectors (HMAC-SHA1) and RFC 7914 Appendix B vector (HMAC-SHA256).
- Literate source with algorithm walkthrough and security guidance.
