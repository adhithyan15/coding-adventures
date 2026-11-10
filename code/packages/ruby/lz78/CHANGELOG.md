# Changelog

## [Unreleased]

### Security

- Windows fix: decode clamps the header-declared length to the bytes produced before byteslice (0xFFFFFFFF overflowed a 32-bit C long and raised RangeError); test simulates the 32-bit limit.
- Hardening (CWE-789): deserialise_tokens clamps the untrusted token count to the data present; decode stops once original_length is reached (quadratic expansion before truncation) and on a dangling dict_index (previously NoMethodError).
- Added test/test_hostile_headers.rb: hostile headers checked for bounded time and sane results, plus round trips over every byte string of length <= 2 and a seeded random sample.

## [0.1.0] - 2026-04-11

### Added

- Initial LZ78 implementation (CMP01 spec).
- `encode`, `decode`, `compress`, `decompress` public API.
- Embedded `TrieNode` with integer-keyed children for byte sequences.
- End-of-stream flush token handling with original_length truncation.
- 28 tests, 65 assertions.
