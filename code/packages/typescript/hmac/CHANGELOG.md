# Changelog — @coding-adventures/hmac

## [0.1.0] — 2026-04-06

### Added

- `hmac(hashFn, blockSize, key, message)` — generic HMAC over any hash function
- `hmacMD5(key, message)` / `hmacMD5Hex` — HMAC-MD5 (RFC 2202), 16-byte tag
- `hmacSHA1(key, message)` / `hmacSHA1Hex` — HMAC-SHA1 (RFC 2202), 20-byte tag
- `hmacSHA256(key, message)` / `hmacSHA256Hex` — HMAC-SHA256 (RFC 4231), 32-byte tag
- `hmacSHA512(key, message)` / `hmacSHA512Hex` — HMAC-SHA512 (RFC 4231), 64-byte tag
- `toHex(bytes)` utility exported for convenience
- Full test suite: RFC 4231 TC1–TC3, TC6, TC7 for SHA-256/SHA-512; RFC 2202 TC1, TC2, TC6 for MD5/SHA-1
- Key normalisation: long keys are pre-hashed; all keys zero-padded to block_size
- Literate source explaining ipad/opad constants, length extension attacks, and HMAC construction
