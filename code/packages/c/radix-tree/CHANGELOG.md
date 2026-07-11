# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-11

### Added

- Pure ISO C17 port of the Rust `radix-tree` crate: a compressed trie (Patricia
  trie) for string keys with a `long` value.
- API: `radix_new` / `radix_free`, `radix_insert`, `radix_search` /
  `radix_contains`, `radix_delete`, `radix_starts_with`,
  `radix_longest_prefix_match`, `radix_keys` / `radix_words_with_prefix` (sorted
  visitor callbacks), `radix_len`, `radix_is_empty`, `radix_node_count`.
- Edge splitting on insert (OOM-safe: new pieces allocated before the existing
  edge is mutated) and prune/merge path compression on delete.
- Edges kept sorted by first byte so key enumeration is ordered; byte-oriented
  and consistent for any input.
- Tests mirroring the crate's suite (split cases, prune/merge node counts,
  prefix queries, empty-string keys, sorted enumeration) under GCC and Clang via
  `iso-harness`.
