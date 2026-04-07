# Changelog — coding-adventures/go/hmac

## [0.1.0] — 2026-04-06

### Added

- `HMAC(hashFn, blockSize, key, message) []byte` — generic HMAC over any `HashFn`
- `HmacMD5(key, message []byte) []byte` — HMAC-MD5 (RFC 2202)
- `HmacSHA1(key, message []byte) []byte` — HMAC-SHA1 (RFC 2202)
- `HmacSHA256(key, message []byte) []byte` — HMAC-SHA256 (RFC 4231)
- `HmacSHA512(key, message []byte) []byte` — HMAC-SHA512 (RFC 4231)
- `HmacMD5Hex`, `HmacSHA1Hex`, `HmacSHA256Hex`, `HmacSHA512Hex` — hex-string variants
- Full test suite: RFC 4231 TC1–TC3, TC6, TC7 for SHA-256/SHA-512; RFC 2202 TC1, TC2, TC6 for MD5/SHA-1
- Key normalisation: long keys are pre-hashed; all keys zero-padded to block_size
- Literate package doc explaining ipad/opad, length extension attacks, and algorithm
