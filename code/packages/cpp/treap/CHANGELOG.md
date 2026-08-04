# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-11

### Added

- Initial pure-ISO C++17 header-only port of the Rust `treap` crate (DT10), in
  namespace `ca::treap`: a randomized balanced BST maintaining BST order on keys
  and max-heap order on priorities.
- `Treap<K>` templated over any less-than-comparable, copyable key type.
- Persistent value semantics: `const` `insert` / `erase` / `split` and static
  `merge` return new treaps.
- Optional explicit priority per insert (`std::optional<double>`), else a
  deterministic xorshift PRNG (function-local `static`; the Rust crate uses a
  global `AtomicU32` — identical arithmetic, single-threaded here).
- Queries: `find`, `contains`, `min_key`, `max_key`, `predecessor`, `successor`,
  `kth_smallest` (1-based), `to_sorted_array`, `size`, `height`, `is_valid`,
  `root`.
- Per-node cached subtree size for `O(h)` order statistics.
- Tests via the shared `iso-harness` (GCC, Clang, MSVC), mirroring the Rust
  crate's own vectors, plus PRNG-priority, empty-treap, and `Treap<std::string>`
  generic cases.
