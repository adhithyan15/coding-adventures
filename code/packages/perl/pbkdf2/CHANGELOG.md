# Changelog — CodingAdventures::PBKDF2 (Perl)

## 0.1.0 — 2026-04-11

### Added
- Initial implementation of PBKDF2 (RFC 8018) for Perl 5.26+.
- `pbkdf2_hmac_sha1`, `pbkdf2_hmac_sha256`, `pbkdf2_hmac_sha512` — return raw binary string.
- `pbkdf2_hmac_sha1_hex`, `pbkdf2_hmac_sha256_hex`, `pbkdf2_hmac_sha512_hex` — return lowercase hex string.
- Validates empty password, non-positive iterations, and non-positive key_length.
- Full RFC 6070 test vectors (HMAC-SHA1) and RFC 7914 Appendix B vector (HMAC-SHA256).
- Uses `pack("N", $i)` for INT_32_BE encoding; `pack("C*", @$bytes)` to convert HMAC arrayref output to binary string.
- Uses Perl's built-in `^` operator for XOR on binary strings.
