# Changelog

All notable changes to this package will be documented in this file.

## [Unreleased]

### Fixed

- `bytes property > returns a copy (mutation-safe)` was flaky at roughly 1 run
  in 256. It set the returned copy's first byte to `0xFF` and then asserted the
  original's first byte was not `0xFF` — which conflates "the original was not
  mutated" with "the original does not happen to start with `0xFF`". `v4()`
  fills byte 0 with random bits (the version and variant nibbles live in bytes
  6 and 8), so an id genuinely starting with `0xFF` failed the test on correct
  code. Measured over 200,000 generations: 788 hits, 1 in 253.8. The test now
  snapshots the whole array before mutating and compares against it, which
  tests the actual claim and is independent of the random bytes. It compares
  all 16 bytes rather than one, so any collateral change to the original is
  caught. Over 200,000 generations the old assertion failed 771 times and the
  new one zero times; against a getter mutated to return the live array, a
  memoised copy, or a subarray view, the new form fails deterministically where
  the old one only did so by chance.
  No production code changed; `get bytes()` was always correct.

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
