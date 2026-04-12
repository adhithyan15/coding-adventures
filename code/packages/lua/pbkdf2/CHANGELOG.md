# Changelog — coding-adventures-pbkdf2 (Lua)

## 0.1.0 — 2026-04-11

### Added
- Initial implementation of PBKDF2 (RFC 8018) for Lua 5.4+.
- `pbkdf2_hmac_sha1`, `pbkdf2_hmac_sha256`, `pbkdf2_hmac_sha512` — return raw byte strings.
- `pbkdf2_hmac_sha1_hex`, `pbkdf2_hmac_sha256_hex`, `pbkdf2_hmac_sha512_hex` — return lowercase hex strings.
- Validates empty password, non-positive/non-integer iterations, and key_length.
- Full RFC 6070 test vectors (HMAC-SHA1) and RFC 7914 Appendix B vector (HMAC-SHA256).
- Uses `string.pack(">I4", i)` for INT_32_BE block index encoding.
- Handles HMAC byte-table output via `to_str()` converter (HMAC returns tables, PBKDF2 works with strings).
