# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-11

### Added

- Pure ISO C++17, header-only port of the Rust `tree-set` crate, in namespace
  `ca::tree_set`: `TreeSet<T, Backend>` generic over its ordered-tree backend,
  defaulting to `ca::avl::AVLTree<T>` and also working with
  `ca::rb::RBTree<T>` (both exercised in the tests). Depends on the sibling
  `avl-tree` and `red-black-tree` packages.
- Persistent API via value semantics: `insert` / `remove` and the algebra
  operations are `const` and return a new set.
- Queries delegate to the backend; lookups that may miss return
  `std::optional<T>`: `contains`, `size`, `is_empty`, `min_value`, `max_value`,
  `first`, `last`, `predecessor`, `successor`, `kth_smallest`, `rank`,
  `to_sorted_array`, `range`.
- Set algebra (`union_with` — `union` is a keyword — `intersection`,
  `difference`, `symmetric_difference`) and relations (`is_subset`,
  `is_superset`, `is_disjoint`, `equals`), computed by the crate's linear merge
  over sorted sequences.
- Tests replicate the crate's unit tests on both backends and add persistence,
  range boundary cases, and relation predicates under GCC and Clang via
  `iso-harness`.
