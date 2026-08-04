# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-11

### Added

- Pure ISO C17 port of the Rust `rope` crate (DT16): a weighted binary tree of
  string chunks (byte-oriented).
- Consuming (move) API mirroring the crate's by-value semantics: `rope_empty`,
  `rope_from_string`, `rope_free`, `rope_concat` (O(1), moves subtrees),
  `rope_split`, `rope_insert`, `rope_delete`, `rope_rebalance`.
- Non-consuming reads: `rope_to_string`, `rope_index` (weighted descent),
  `rope_substring`, `rope_len`, `rope_is_empty`, `rope_depth`, `rope_is_balanced`.
- Overflow-guarded concat length and delete end-offset; every consuming function
  frees its inputs on allocation failure (no leaks / dangling half-consumed
  ropes).
- Tests pinned to the crate's assertions plus empty/clamping/weighted-index edge
  cases, run under GCC and Clang via `iso-harness`.
