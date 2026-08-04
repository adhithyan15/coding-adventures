# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-11

### Added

- Pure ISO C17 port of the Rust `avl-tree` crate (DT08): a self-balancing AVL
  tree with cached subtree height and size for O(log n) order statistics.
- Persistent API mirroring the Rust crate: `avl_insert` / `avl_delete` return a
  new tree (deep copy) and leave the input untouched. `avl_empty` / `avl_free`
  manage ownership; updates return `NULL` only on allocation failure.
- Queries: `avl_search`, `avl_contains`, `avl_min_value`, `avl_max_value`,
  `avl_predecessor`, `avl_successor`, `avl_kth_smallest`, `avl_rank`,
  `avl_to_sorted_array`, `avl_size`, `avl_height`, `avl_balance_factor`,
  `avl_is_valid_bst`, `avl_is_valid_avl`.
- Same rotate / rebalance / extract-min algorithms as the crate; the deep-clone
  path is bounded by tree height (balanced ⇒ O(log n) recursion depth).
- Tests replicate the crate's unit tests and add delete, neighbour queries,
  persistence, and a 0..99 insert/delete stress under GCC and Clang via
  `iso-harness`.
