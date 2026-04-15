# Changelog

## [0.1.0] - 2026-04-11

### Added

- Initial implementation of LZ78 encoding and decoding (CMP01 spec).
- `Encode(data []byte, maxDictSize int) []Token` — trie-based encoder.
- `Decode(tokens []Token, originalLength int) []byte` — parent-chain decoder.
- `Compress` / `Decompress` — one-shot API with CMP01 wire format.
- 95.4% test coverage, 27 tests.
