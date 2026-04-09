# Changelog

## 0.1.0 — 2026-04-06

Initial release.

- Implemented `hmac/4` — generic HMAC taking any hash function and block size
- Implemented `hmac_md5/2` and `hmac_md5_hex/2` — HMAC-MD5 (16-byte tag)
- Implemented `hmac_sha1/2` and `hmac_sha1_hex/2` — HMAC-SHA1 (20-byte tag)
- Implemented `hmac_sha256/2` and `hmac_sha256_hex/2` — HMAC-SHA256 (32-byte tag)
- Implemented `hmac_sha512/2` and `hmac_sha512_hex/2` — HMAC-SHA512 (64-byte tag)
- 45 tests covering RFC 4231 (SHA-256/SHA-512) and RFC 2202 (MD5/SHA-1) vectors
- 100% line coverage
- Depends on `coding_adventures_md5`, `coding_adventures_sha1`,
  `coding_adventures_sha256`, `coding_adventures_sha512`
