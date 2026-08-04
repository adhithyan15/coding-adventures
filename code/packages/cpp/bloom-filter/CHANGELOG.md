# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-11

### Added

- Pure ISO C++17, header-only port of the Rust `bloom-filter` crate (DT22), in
  namespace `ca`.
- `ca::bloom_filter` with a throwing constructor and `from_params`, plus
  non-throwing `try_create` / `try_from_params` factories returning
  `std::optional`.
- `add` / `contains` (raw bytes or `std::string`) using double hashing (FNV-1a +
  djb2 finalised with fmix32), matching the Rust crate's scheme.
- Accessors (`bit_count`, `hash_count`, `bits_set`, `size_bytes`, `fill_ratio`,
  `estimated_false_positive_rate`, `is_over_capacity`) and static sizing helpers
  (`optimal_m`, `optimal_k`, `capacity_for_memory`).
- A self-contained, libm-free natural log (range reduction + `2·atanh` series)
  so sizing needs no external math library; `ratio^k` computed with a multiply
  loop instead of `std::pow`.
- Tests mirroring the Rust suite (no false negatives, consistent stats, sizing
  formulas, throwing + optional validation paths), run under GCC and Clang via
  `iso-harness`.
