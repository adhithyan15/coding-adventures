# Changelog

## 0.1.0 — 2026-04-06

Initial release.

- Implemented `hmac()` — generic HMAC taking any hash function and block size
- Implemented `hmac_md5()` / `hmac_md5_hex()` — HMAC-MD5 (16-byte tag)
- Implemented `hmac_sha1()` / `hmac_sha1_hex()` — HMAC-SHA1 (20-byte tag)
- Implemented `hmac_sha256()` / `hmac_sha256_hex()` — HMAC-SHA256 (32-byte tag)
- Implemented `hmac_sha512()` / `hmac_sha512_hex()` — HMAC-SHA512 (64-byte tag)
- 46 tests covering RFC 4231 (SHA-256/SHA-512) and RFC 2202 (MD5/SHA-1) vectors
- 100% line coverage
- Depends on `coding-adventures-md5`, `coding-adventures-sha1`,
  `coding-adventures-sha256`, `coding-adventures-sha512`
