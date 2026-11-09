# Changelog

## [Unreleased]

### Security

- Windows fix: decode clamps the header-declared length to the decoded size before Array#[] (0xFFFFFFFF overflowed a 32-bit C long and raised RangeError); test simulates the 32-bit limit.
- decode stops once original_length is reached and skips invalid back-references (previously appended nil). block_count was already capped.
- Added test/test_hostile_headers.rb: hostile headers checked for bounded time and sane results, plus round trips over every byte string of length <= 2 and a seeded random sample.

## [0.1.0] - 2026-04-11

### Added

- Initial implementation of LZSS encoding and decoding (CMP02 spec).
- `Literal` and `Match` structs as the two token types.
- `encode` / `decode` token-level API.
- `compress` / `decompress` one-shot CMP02 wire-format API.
- Security: `block_count` capped against payload size to prevent DoS.
