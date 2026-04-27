# Changelog — uuid (Kotlin)

## [0.1.0] — 2026-04-25

### Added
- Initial implementation of UUID v1/v3/v4/v5/v7 per RFC 4122 and RFC 9562.
- `UUID` is a Kotlin `data class` with two `Long` fields (`msb`, `lsb`) in
  big-endian byte order, providing value semantics (equals, hashCode, copy,
  destructuring) for free.
- `fromString(text)`, `fromBytes(ByteArray)` — factory methods; throw
  `UUIDException` on invalid input.
- `isValid(text)` — nullable-safe static validator.
- `toBytes()` — returns the 16-byte big-endian representation.
- `toString()` — canonical `8-4-4-4-12` lowercase hyphenated string.
- `version`, `variant`, `isNil`, `isMax` — Kotlin `val` properties.
- `compareTo` — unsigned byte-order comparison; temporal for v7.
- `v4()`, `v7()`, `v1()`, `v5(namespace, name)`, `v3(namespace, name)` —
  factory functions in the companion object.
- `NIL`, `MAX`, `NAMESPACE_DNS`, `NAMESPACE_URL`, `NAMESPACE_OID`,
  `NAMESPACE_X500` — standard constants.
- `UUIDException` — nullable-cause constructor for ergonomic use.
- Literate source with bit-layout diagrams and algorithm explanations.
- 49 unit tests covering all versions, all parse formats, RFC test vectors,
  uniqueness, ordering, and Kotlin-specific `data class` behaviour.
