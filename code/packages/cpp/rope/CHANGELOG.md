# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-11

### Added

- Pure ISO C++17, header-only port of the Rust `rope` crate (DT16), in namespace
  `ca`: a weighted binary tree of string chunks (byte-oriented).
- Value semantics with structural sharing — immutable `std::shared_ptr` nodes, so
  `ca::rope` is cheaply copyable and operations reuse untouched subtrees.
- `from_string`, `concat` (O(1)), `split`, `insert`, `erase` (`delete` is a
  keyword), `rebalance`, `to_string`, `index` (→ `std::optional<char>`),
  `substring`, `len`, `empty`, `depth`, `is_balanced`.
- Tests pinned to the crate's assertions plus empty/clamping/weighted-index and
  copy-independence checks, run under GCC and Clang via `iso-harness`.
