# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-11

### Added

- Initial pure-ISO C17 port of the Rust `treap` crate (DT10): a randomized
  balanced BST maintaining BST order on keys and max-heap order on priorities.
- Persistent (deep-copy) `treap_insert` / `treap_delete` / `treap_split` /
  `treap_merge` returning new treaps.
- `treap_empty`, `treap_free`; rotate-on-insert and priority-merge-on-delete.
- Optional explicit priority per insert, else a deterministic xorshift PRNG
  (plain `static` counter; the Rust crate uses a global `AtomicU32` — identical
  arithmetic, single-threaded here).
- Queries: `treap_search`, `treap_contains`, `treap_min_key`, `treap_max_key`,
  `treap_predecessor`, `treap_successor`, `treap_kth_smallest` (1-based),
  `treap_to_sorted_array`, `treap_size`, `treap_height`, `treap_is_valid`.
- Per-node cached subtree size for `O(h)` order statistics.
- Tests via the shared `iso-harness` (GCC, Clang, MSVC), mirroring the Rust
  crate's own vectors, plus PRNG-priority and empty-treap cases.
