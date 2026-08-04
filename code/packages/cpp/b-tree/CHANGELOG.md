# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-11

### Added

- Pure ISO C++17, header-only port of the Rust `b-tree` crate, in namespace `ca`:
  a full CLRS B-tree of minimum degree `t`, fully generic over `ca::b_tree<K, V>`.
- Insert with proactive top-down splitting; delete with pre-fill (rotate from a
  sibling / merge) and root shrinking; `std::vector` nodes with `std::unique_ptr`
  children.
- `insert`, `remove`, `search` (→ `const V*`), `contains`, `min_key` / `max_key`
  (→ `std::optional<K>`), `inorder` / `range_query` (→ vectors of pairs), `len`,
  `empty`, `height`, `is_valid`.
- Torture tests (1000–2000 out-of-order inserts at degrees 2/3/7, sorted-order
  and `is_valid` checks, deletion of half the keys) plus a `std::string`-value
  genericity check, under GCC and Clang via `iso-harness`.
