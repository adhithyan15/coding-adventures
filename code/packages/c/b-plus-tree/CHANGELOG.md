# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-11

### Added

- Pure ISO C17 port of the Rust `b-plus-tree` crate: a B+ tree of minimum degree
  `t` specialised to `long → long`, with all values in leaves and a leaf linked
  list for range scans.
- Insert with leaf/internal splitting propagated bottom-up; delete with
  borrow-from-sibling / merge rebalancing and root shrinking; the leaf `next`
  chain kept in sync across splits and merges.
- API: `bpt_new` / `bpt_free`, `bpt_insert`, `bpt_delete`, `bpt_search` /
  `bpt_contains`, `bpt_min_key` / `bpt_max_key`, `bpt_full_scan` /
  `bpt_range_scan` (visitor callbacks over the leaf chain), `bpt_len`,
  `bpt_is_empty`, `bpt_height`, `bpt_is_valid`.
- Fixed-capacity nodes (`2t` keys, `2t+1` children); `t` clamped so `2t+1`
  cannot overflow. OOM-safe insert (each full node pre-allocates its split node
  before mutating); delete allocates nothing.
- Torture tests (1000–2000 out-of-order inserts at degrees 2/3/6, sorted
  full-scan and `is_valid` checks, deletion of half the keys) under GCC and
  Clang via `iso-harness`.
