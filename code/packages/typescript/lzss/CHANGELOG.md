# Changelog

## [Unreleased]

### Security

- decode stops once originalLength is reached and skips invalid back-references (previously pushed undefined bytes). block_count was already capped.
- Added tests/hostile_headers.test.ts (wired into the npm test scripts): hostile headers checked for bounded time, plus round trips over every byte string of length <= 2 and a seeded random sample.

## [0.1.0] - 2026-04-11

### Added

- Initial implementation of LZSS encoding and decoding (CMP02 spec).
- `Literal` and `Match` interfaces as the two token types.
- `encode` / `decode` token-level API.
- `compress` / `decompress` one-shot CMP02 wire-format API.
- Security: `block_count` capped against payload size to prevent DoS.
