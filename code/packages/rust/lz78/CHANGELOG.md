# Changelog

## [0.1.0] - 2026-04-11

### Added

- Initial LZ78 implementation (CMP01 spec). 21 tests (16 unit + 5 doctests).
- Arena-based trie for efficient byte-indexed child lookup during encoding.
- `encode`, `decode`, `compress`, `decompress` public API.
