# Changelog

## [Unreleased]

### Added

- Add required_capabilities.json (pure computation).

### Security

- CWE-789/CWE-400: `deserialise_tokens` no longer preallocates from the untrusted token-count header; the count is clamped to `(len - 8) / 4`. `decode` stops on a dangling `dict_index` instead of panicking, and stops as soon as the declared `original_length` is reached (a hostile chain of tokens previously expanded quadratically before truncation).
- Added `tests/hostile_headers.rs` (allocation guard, hostile headers, exhaustive <=2-byte and seeded random round trips).

## [0.1.0] - 2026-04-11

### Added

- Initial LZ78 implementation (CMP01 spec). 21 tests (16 unit + 5 doctests).
- Arena-based trie for efficient byte-indexed child lookup during encoding.
- `encode`, `decode`, `compress`, `decompress` public API.
