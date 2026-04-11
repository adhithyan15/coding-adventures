# Changelog — swift/pbkdf2

## 0.1.0 — 2026-04-11

### Added
- Initial implementation of PBKDF2 (RFC 8018) for Swift.
- `pbkdf2HmacSHA1`, `pbkdf2HmacSHA256`, `pbkdf2HmacSHA512` — return `Data` (throws `PBKDF2Error`).
- `pbkdf2HmacSHA1Hex`, `pbkdf2HmacSHA256Hex`, `pbkdf2HmacSHA512Hex` — return `String`.
- `PBKDF2Error` enum with `emptyPassword`, `invalidIterations`, `invalidKeyLength` cases.
- Full RFC 6070 test vectors (HMAC-SHA1) and RFC 7914 Appendix B vector (HMAC-SHA256).
- Literate source with algorithm walkthrough, INT_32_BE explanation, and security guidance.
