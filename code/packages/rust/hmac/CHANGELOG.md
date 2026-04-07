# Changelog — coding_adventures_hmac

## [0.1.0] — 2026-04-06

### Added

- `hmac<F>(hash_fn, block_size, key, message) -> Vec<u8>` — generic HMAC over any hash function
- `hmac_md5(key, message) -> [u8; 16]` — HMAC-MD5 (RFC 2202)
- `hmac_sha1(key, message) -> [u8; 20]` — HMAC-SHA1 (RFC 2202)
- `hmac_sha256(key, message) -> [u8; 32]` — HMAC-SHA256 (RFC 4231)
- `hmac_sha512(key, message) -> [u8; 64]` — HMAC-SHA512 (RFC 4231)
- `hmac_md5_hex`, `hmac_sha1_hex`, `hmac_sha256_hex`, `hmac_sha512_hex` — hex-string variants
- Full test suite: RFC 4231 TC1–TC3, TC6, TC7 for SHA-256/SHA-512; RFC 2202 TC1, TC2, TC6 for MD5/SHA-1
- Key normalisation: keys longer than block_size are pre-hashed; all keys zero-padded to block_size
- Literate documentation explaining ipad/opad choice, length extension attacks, and Merkle-Damgård weakness
