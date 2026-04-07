# Changelog — coding_adventures_hmac (Ruby)

## [0.1.0] — 2026-04-06

### Added

- `hmac(hash_fn, block_size, key, message)` — generic HMAC over any hash Proc
- `hmac_md5(key, message)` / `hmac_md5_hex` — HMAC-MD5 (RFC 2202), 16-byte tag
- `hmac_sha1(key, message)` / `hmac_sha1_hex` — HMAC-SHA1 (RFC 2202), 20-byte tag
- `hmac_sha256(key, message)` / `hmac_sha256_hex` — HMAC-SHA256 (RFC 4231), 32-byte tag
- `hmac_sha512(key, message)` / `hmac_sha512_hex` — HMAC-SHA512 (RFC 4231), 64-byte tag
- Full test suite: RFC 4231 TC1–TC3, TC6, TC7 for SHA-256/SHA-512; RFC 2202 TC1, TC2, TC6 for MD5/SHA-1
- Key normalisation: long keys pre-hashed; all keys zero-padded to block_size with `ljust`
- Literate source with inline comments explaining ipad/opad, length extension attacks, and the algorithm
