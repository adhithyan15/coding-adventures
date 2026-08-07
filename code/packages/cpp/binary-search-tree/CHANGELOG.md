# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-11

### Added

- Initial pure-ISO C++17 header-only port of the Rust `binary-search-tree`
  crate (DT07), in namespace `ca::bst`.
- `BST<T>` templated over any less-than-comparable, copyable element type.
- Persistent value semantics: `const` `insert` / `erase` return new trees.
- `empty`, `from_sorted_array` (balanced build).
- Queries: `find`, `contains`, `min_value`, `max_value`, `predecessor`,
  `successor`, `kth_smallest` (1-based order statistic), `rank`,
  `to_sorted_array`, `size`, `height`, `is_valid`, `root`.
- Per-node cached subtree size for `O(h)` rank / select.
- Tests via the shared `iso-harness` (GCC, Clang, MSVC), mirroring the Rust
  crate's own vectors, plus a `BST<std::string>` generic-use case.
