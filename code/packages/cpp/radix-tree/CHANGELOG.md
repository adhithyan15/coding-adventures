# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-11

### Added

- Pure ISO C++17, header-only port of the Rust `radix-tree` crate, in namespace
  `ca`: a generic `ca::radix_tree<V>` compressed trie (Patricia trie) for string
  keys.
- `insert`, `search` (→ `const V*`), `contains`, `remove` (`delete` is a
  keyword), `starts_with`, `longest_prefix_match` (→ `std::optional<std::string>`),
  `keys` / `words_with_prefix` (→ sorted `std::vector<std::string>`), `len`,
  `empty`, `node_count`.
- Nodes use `std::map<unsigned char, edge>` (sorted by first byte) with
  `std::unique_ptr` children; edge splitting on insert and prune/merge path
  compression on delete.
- Tests mirroring the crate's suite plus a `std::string`-value genericity check,
  under GCC and Clang via `iso-harness`.
