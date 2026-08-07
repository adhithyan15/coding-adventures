# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-11

### Added

- Pure ISO C++17, header-only port of the Rust `b-plus-tree` crate, in namespace
  `ca`: a fully generic `ca::b_plus_tree<K, V>` with all values in leaves and a
  leaf linked list for range scans.
- Insert with leaf/internal splitting (bottom-up); delete with borrow/merge
  rebalancing and root shrinking; `std::vector` nodes with `std::unique_ptr`
  children and a non-owning raw `next` leaf-chain pointer.
- `insert`, `remove`, `search` (→ `const V*`), `contains`, `min_key` / `max_key`
  (→ `std::optional<K>`), `full_scan` / `range_scan` (→ vectors of pairs), `len`,
  `empty`, `height`, `is_valid`.
- Torture tests (1000–2000 out-of-order inserts at degrees 2/3/6, sorted
  full-scan and `is_valid` checks, deletion of half the keys) plus a
  `std::string`-value genericity check, under GCC and Clang via `iso-harness`.
