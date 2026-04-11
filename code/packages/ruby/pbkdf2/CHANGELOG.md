# Changelog — coding_adventures_pbkdf2

## 0.1.0 — 2026-04-11

### Added
- Initial implementation of PBKDF2 (RFC 8018) for Ruby.
- `pbkdf2_hmac_sha1`, `pbkdf2_hmac_sha256`, `pbkdf2_hmac_sha512` — returns binary String.
- `pbkdf2_hmac_sha1_hex`, `pbkdf2_hmac_sha256_hex`, `pbkdf2_hmac_sha512_hex` — returns lowercase hex String.
- Validates empty password, non-positive iterations, and non-positive key_length.
- Full RFC 6070 test vectors (HMAC-SHA1) and RFC 7914 Appendix B vector (HMAC-SHA256).
- Literate source with algorithm walkthrough, INT_32_BE explanation, and security guidance.
