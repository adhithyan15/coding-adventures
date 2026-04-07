# Changelog — HMAC (Swift)

## [0.1.0] — 2026-04-06

### Added

- `hmac(hashFn:blockSize:key:message:) -> Data` — generic HMAC over any hash function
- `hmacMD5(key:message:) -> Data` — HMAC-MD5 (RFC 2202), 16-byte tag
- `hmacSHA1(key:message:) -> Data` — HMAC-SHA1 (RFC 2202), 20-byte tag
- `hmacSHA256(key:message:) -> Data` — HMAC-SHA256 (RFC 4231), 32-byte tag
- `hmacSHA512(key:message:) -> Data` — HMAC-SHA512 (RFC 4231), 64-byte tag
- `hmacMD5Hex`, `hmacSHA1Hex`, `hmacSHA256Hex`, `hmacSHA512Hex` — hex-string variants
- Full test suite using Swift Testing framework: RFC 4231 TC1–TC3, TC6, TC7 for SHA-256/SHA-512; RFC 2202 TC1, TC2, TC6 for MD5/SHA-1
- Key normalisation: long keys pre-hashed; all keys zero-padded to block_size
- Literate source with inline comments explaining ipad/opad, length extension attacks, and the two-layer construction
