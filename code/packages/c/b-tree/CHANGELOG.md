# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-11

### Added

- Pure ISO C17 port of the Rust `b-tree` crate: a full CLRS B-tree of minimum
  degree `t`, specialised to `long → long` (a sorted integer map).
- Insert with proactive top-down splitting; delete with pre-fill (rotate from a
  sibling / merge) and root shrinking.
- API: `btree_new` / `btree_free`, `btree_insert`, `btree_delete`,
  `btree_search` / `btree_contains`, `btree_min_key` / `btree_max_key`,
  `btree_inorder` / `btree_range_query` (visitor callbacks), `btree_len`,
  `btree_is_empty`, `btree_height`, `btree_is_valid`.
- Fixed-capacity nodes (`2t-1` keys, `2t` children) so no in-node reallocation is
  needed; `t` clamped so `2t` cannot overflow `size_t`; delete allocates nothing.
- Torture tests (1000–2000 out-of-order inserts at degrees 2/3/7, sorted-order
  and `is_valid` checks, deletion of half the keys) under GCC and Clang via
  `iso-harness`.
