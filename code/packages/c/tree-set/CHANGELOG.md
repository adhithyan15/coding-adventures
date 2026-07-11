# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-11

### Added

- Pure ISO C17 port of the Rust `tree-set` crate: an ordered set built on the
  sibling `avl-tree` backend (the crate's default), depended on via
  `# build-tool: deps=c/avl-tree`.
- Persistent API mirroring the crate: `tset_insert` / `tset_remove` and the
  algebra operations return a new set; `tset_empty` / `tset_free` manage
  ownership; allocating operations return `NULL` only on allocation failure.
- Queries delegate to the backend: `tset_size`, `tset_is_empty`,
  `tset_contains`, `tset_min_value`, `tset_max_value`, `tset_predecessor`,
  `tset_successor`, `tset_kth_smallest`, `tset_rank`, `tset_to_sorted_array`,
  `tset_range`.
- Set algebra (`tset_union`, `tset_intersection`, `tset_difference`,
  `tset_symmetric_difference`) and relations (`tset_is_subset`,
  `tset_is_superset`, `tset_is_disjoint`, `tset_equals`), computed by the
  crate's linear merge over sorted sequences; result-size arithmetic is
  overflow-guarded and per-set arrays use `calloc`'s checked multiply.
- Tests replicate the crate's unit tests and add persistence, range boundary
  cases, and relation predicates under GCC and Clang via `iso-harness`.
