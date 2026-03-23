# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-03-23

### Added

- Full UUID v1/v3/v4/v5/v7 implementation from scratch (RFC 4122 + RFC 9562)
- `Ca::Uuid::UUID` class: stores 16 bytes in binary encoding; derives `version`,
  `variant`, `nil?`, `max?`, `to_s`, `inspect`, `<=>` from the raw bytes
- `Ca::Uuid::UUIDError` exception class
- `UUID.parse(s)`: accepts standard, uppercase, compact, braced, URN formats
- `UUID.valid?(s)`: non-raising validation
- Namespace constants: `Ca::Uuid::NAMESPACE_DNS`, `NAMESPACE_URL`, `NAMESPACE_OID`,
  `NAMESPACE_X500` (RFC 4122 Appendix C)
- `Ca::Uuid::NIL` and `Ca::Uuid::MAX` sentinels
- `Ca::Uuid.v4()`: 122 bits of `SecureRandom.random_bytes`
- `Ca::Uuid.v5(namespace, name)`: SHA-1 name-based via Ca::Sha1;
  RFC vector: v5(NAMESPACE_DNS, "python.org") = "886313e1-3b8a-5372-9b90-0c9aee199e5d"
- `Ca::Uuid.v3(namespace, name)`: MD5 name-based via Ca::Md5;
  RFC vector: v3(NAMESPACE_DNS, "python.org") = "6fa459ea-ee8a-3ca4-894e-db77e160355e"
- `Ca::Uuid.v1()`: 60-bit Gregorian timestamp via `Process.clock_gettime` +
  random clock sequence + random node (multicast bit set)
- `Ca::Uuid.v7()`: 48-bit Unix millisecond timestamp (sortable) + 74 random bits
- Gem named `ca_uuid`, module `Ca::Uuid` (ca_ prefix for consistency)
- 90 tests, 96.67% line coverage
- Knuth-style literate programming comments throughout
