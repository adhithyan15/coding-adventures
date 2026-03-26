# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-03-23

### Added

- Initial implementation of `ImmutableList` -- a persistent vector using a 32-way trie with structural sharing.
- Core operations: `new()`, `from_slice()`, `push()`, `get()`, `set()`, `pop()`, `len()`, `is_empty()`.
- `iter()` for ordered iteration over elements.
- `to_vec()` for collecting elements into a plain `Vec<String>`.
- `PartialEq`, `Eq`, `Display`, `Debug`, `Default`, and `Clone` trait implementations.
- Tail buffer optimization: ~97% of pushes are O(1) tail appends.
- Structural sharing via `Arc`: mutations create O(log32 n) new nodes, sharing everything else.
- Comprehensive test suite covering correctness, structural sharing, boundary cases (32, 33, 1024, 1025, 32768, 100K elements), and performance smoke tests.
