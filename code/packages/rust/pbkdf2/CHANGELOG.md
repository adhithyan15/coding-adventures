# Changelog — coding_adventures_pbkdf2

## 0.1.0 — 2026-04-11

### Added
- Initial implementation of PBKDF2 (RFC 8018) for Rust.
- `pbkdf2_hmac_sha1`, `pbkdf2_hmac_sha256`, `pbkdf2_hmac_sha512` — return `Result<Vec<u8>, Pbkdf2Error>`.
- `pbkdf2_hmac_sha1_hex`, `pbkdf2_hmac_sha256_hex`, `pbkdf2_hmac_sha512_hex` — return `Result<String, Pbkdf2Error>`.
- `Pbkdf2Error` enum with `EmptyPassword`, `InvalidIterations`, `InvalidKeyLength` variants.
- Full RFC 6070 test vectors (HMAC-SHA1) and RFC 7914 Appendix B vector (HMAC-SHA256).
- Literate source with algorithm walkthrough, block diagram, and security guidance.
