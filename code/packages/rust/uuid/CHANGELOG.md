# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-03-23

### Added

- Full UUID v1/v3/v4/v5/v7 implementation from scratch (RFC 4122 + RFC 9562)
- `UUID` struct ([u8; 16]) implementing `Copy`, `Clone`, `Eq`, `Ord`, `Hash`,
  `Display`, `Debug`, `FromStr`
- `UUIDError` type implementing `std::error::Error`
- `parse()`: accepts standard, compact, braced, URN formats
- `is_valid()`: non-panicking string validation
- Constants: `NIL`, `MAX`, `NAMESPACE_DNS`, `NAMESPACE_URL`, `NAMESPACE_OID`,
  `NAMESPACE_X500` (RFC 4122 Appendix C)
- `v4()`: 122 bits via `getrandom::getrandom()` (OS-backed CSPRNG)
- `v5(namespace, name)`: SHA-1 name-based via coding_adventures_sha1::sum1();
  RFC vector: v5(NAMESPACE_DNS, "python.org") = "886313e1-3b8a-5372-9b90-0c9aee199e5d"
- `v3(namespace, name)`: MD5 name-based via coding_adventures_md5::sum_md5();
  RFC vector: v3(NAMESPACE_DNS, "python.org") = "6fa459ea-ee8a-3ca4-894e-db77e160355e"
- `v1()`: 60-bit 100-ns Gregorian timestamp via `SystemTime::now()` +
  random clock sequence + random node (multicast bit set via `getrandom`)
- `v7()`: 48-bit Unix millisecond timestamp (sortable) + 74 random bits
- Crate named `coding_adventures_uuid` (ca_ prefix for consistency)
- 59 unit tests + 8 doc-tests = 67 total, 100% function coverage
- Knuth-style literate programming doc-comments throughout
