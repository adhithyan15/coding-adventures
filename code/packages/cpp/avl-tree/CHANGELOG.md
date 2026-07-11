# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-11

### Added

- Pure ISO C++17, header-only port of the Rust `avl-tree` crate (DT08), in
  namespace `ca::avl`: a templated self-balancing AVL tree with cached subtree
  height and size for O(log n) order statistics.
- Persistent API via value semantics: `insert` / `erase` are `const` and return
  a new tree; `AVLTree<T>` deep-copies on copy, so the receiver is untouched
  (`erase` replaces the keyword `delete`).
- Queries return `std::optional<T>` where a lookup may miss: `find`, `contains`,
  `min_value`, `max_value`, `predecessor`, `successor`, `kth_smallest`, `rank`,
  `to_sorted_array`, `size`, `height`, `root`, `balance_factor`, `is_valid_bst`,
  `is_valid_avl`.
- Built on `std::unique_ptr` children (mirroring Rust's `Option<Box<Node>>`);
  the same rotate / rebalance / extract-min algorithms. `T` requires only
  less-than comparability and copyability.
- Tests replicate the crate's unit tests and add delete, neighbour queries,
  value-semantics independence, and a 0..99 insert/delete stress under GCC and
  Clang via `iso-harness`.
