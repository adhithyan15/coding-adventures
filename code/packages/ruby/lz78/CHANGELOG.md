# Changelog

## [0.1.0] - 2026-04-11

### Added

- Initial LZ78 implementation (CMP01 spec).
- `encode`, `decode`, `compress`, `decompress` public API.
- Embedded `TrieNode` with integer-keyed children for byte sequences.
- End-of-stream flush token handling with original_length truncation.
- 28 tests, 65 assertions.
