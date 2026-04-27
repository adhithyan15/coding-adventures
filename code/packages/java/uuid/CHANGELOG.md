# Changelog — uuid (Java)

## [0.1.0] — 2026-04-25

### Added
- Initial implementation of UUID v1/v3/v4/v5/v7 per RFC 4122 and RFC 9562.
- Stored as two `long` fields (`msb`, `lsb`) in big-endian byte order, matching
  the JDK's `java.util.UUID` layout.
- `fromString(text)` — parses standard, compact, braced, and URN UUID formats
  (case-insensitive). Throws `UUIDException` for invalid input.
- `fromBytes(byte[])` — constructs from 16 raw bytes in network byte order.
- `isValid(text)` — static validator returning boolean.
- `toBytes()` — returns the 16-byte big-endian representation.
- `toString()` — returns the canonical `8-4-4-4-12` lowercase hyphenated form.
- `version()` — the version nibble (bits 48–51).
- `variant()` — the variant field as a string: `"rfc4122"`, `"microsoft"`,
  `"ncs"`, or `"reserved"`.
- `isNil()`, `isMax()` — true for the all-zero and all-one UUIDs.
- `compareTo(UUID)` — unsigned lexicographic ordering by bytes; corresponds to
  temporal ordering for v7 UUIDs.
- `v4()` — random UUID using `SecureRandom`.
- `v7()` — time-ordered random UUID (RFC 9562); 48-bit millisecond timestamp
  in high bits for database index locality.
- `v1()` — time-based UUID; 60-bit Gregorian timestamp + random node ID.
- `v5(namespace, name)` — name-based SHA-1 UUID. RFC test vector:
  `v5(NAMESPACE_DNS, "python.org")` → `886313e1-3b8a-5372-9b90-0c9aee199e5d`.
- `v3(namespace, name)` — name-based MD5 UUID. RFC test vector:
  `v3(NAMESPACE_DNS, "python.org")` → `6fa459ea-ee8a-3ca4-894e-db77e160355e`.
- `NIL`, `MAX`, `NAMESPACE_DNS`, `NAMESPACE_URL`, `NAMESPACE_OID`,
  `NAMESPACE_X500` — standard constants per RFC 4122.
- `UUIDException` extends `IllegalArgumentException` for parse errors.
- Static-initialisation order fix: `UUID_PATTERN` declared before the
  `NAMESPACE_*` constants (which call `fromString()` at class-init time).
- Literate source with bit-layout diagrams, algorithm explanations, and
  historical context.
- 47 unit tests covering all 5 UUID versions, all parse formats, properties,
  RFC test vectors, uniqueness, ordering, and edge cases.
