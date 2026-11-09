# Changelog

## [Unreleased]

### Security

- CWE-789/CWE-400: deserialiseTokens clamps the untrusted token count to (len-8)/4 before make() and the loop. Decode stops on a dangling DictIndex instead of panicking and stops once originalLength is reached (previously quadratic expansion before truncation).
- Added hostile_headers_test.go: hostile headers checked for bounded time and cumulative allocation, plus round trips over every byte string of length <= 2 and a seeded random sample.

## [0.1.0] - 2026-04-11

### Added

- Initial implementation of LZ78 encoding and decoding (CMP01 spec).
- `Encode(data []byte, maxDictSize int) []Token` — trie-based encoder.
- `Decode(tokens []Token, originalLength int) []byte` — parent-chain decoder.
- `Compress` / `Decompress` — one-shot API with CMP01 wire format.
- 95.4% test coverage, 27 tests.
