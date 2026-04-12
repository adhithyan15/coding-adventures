# Changelog — coding_adventures_pbkdf2 (Elixir)

## 0.1.0 — 2026-04-11

### Added
- Initial implementation of PBKDF2 (RFC 8018) for Elixir.
- `pbkdf2_hmac_sha1/4`, `pbkdf2_hmac_sha256/4`, `pbkdf2_hmac_sha512/4` — return binary.
- `pbkdf2_hmac_sha1_hex/4`, `pbkdf2_hmac_sha256_hex/4`, `pbkdf2_hmac_sha512_hex/4` — return lowercase hex string.
- Validates empty password, non-positive iterations, and non-positive key_length.
- Full RFC 6070 test vectors (HMAC-SHA1) and RFC 7914 Appendix B vector (HMAC-SHA256).
- Uses `:crypto.exor/2` for XOR accumulation and `<<i::big-unsigned-integer-size(32)>>` for INT_32_BE encoding.
