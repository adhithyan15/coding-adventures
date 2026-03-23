# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-03-23

### Added

- Full UUID v1/v3/v4/v5/v7 implementation from scratch (RFC 4122 + RFC 9562)
- `UUID` class with `bytes`, `int`, `version`, `variant`, `isNil`, `isMax`
  properties; `toString()`, `equals()`, `compareTo()` methods
- `UUIDError` class extending Error
- `parse()`: accepts standard, uppercase, compact, braced, URN formats
- `isValid()`: non-throwing string validation
- Namespace constants: `NAMESPACE_DNS`, `NAMESPACE_URL`, `NAMESPACE_OID`,
  `NAMESPACE_X500` (RFC 4122 Appendix C)
- `NIL` and `MAX` sentinels
- `v4()`: 122 bits from `crypto.getRandomValues()` (Web Crypto API)
- `v5(namespace, name)`: SHA-1 name-based via @ca/sha1;
  RFC vector: v5(NAMESPACE_DNS, "python.org") = "886313e1-3b8a-5372-9b90-0c9aee199e5d"
- `v3(namespace, name)`: MD5 name-based via @ca/md5;
  RFC vector: v3(NAMESPACE_DNS, "python.org") = "6fa459ea-ee8a-3ca4-894e-db77e160355e"
- `v1()`: 60-bit 100-ns Gregorian timestamp (BigInt arithmetic) +
  14-bit random clock sequence + 48-bit random node
- `v7()`: 48-bit Unix millisecond timestamp (sortable) + 74 random bits
- Package renamed `@ca/uuid` for consistent ca_ naming across all languages
- 95 tests, 94.7% statement coverage, 91.22% branch coverage
- Knuth-style literate programming comments throughout
