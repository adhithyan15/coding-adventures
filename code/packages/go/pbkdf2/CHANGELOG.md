# Changelog — go/pbkdf2

## 0.1.0 — 2026-04-11

### Added
- Initial implementation of PBKDF2 (RFC 8018) for Go.
- `PBKDF2HmacSHA1`, `PBKDF2HmacSHA256`, `PBKDF2HmacSHA512` — return `([]byte, error)`.
- `PBKDF2HmacSHA1Hex`, `PBKDF2HmacSHA256Hex`, `PBKDF2HmacSHA512Hex` — return `(string, error)`.
- Sentinel errors: `ErrEmptyPassword`, `ErrInvalidIterations`, `ErrInvalidKeyLength`.
- Full RFC 6070 test vectors (HMAC-SHA1) and RFC 7914 vector (HMAC-SHA256).
- Literate source with block diagram, iteration count guidance, and real-world usage examples.
