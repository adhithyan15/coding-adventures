# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-11

### Added

- Pure ISO C17 port of the Rust `bloom-filter` crate (DT22).
- `bloom_init` (size for n items + false-positive rate) and
  `bloom_init_params` (explicit bit/hash counts), paired with `bloom_free`.
- `bloom_add` / `bloom_contains` using double hashing (FNV-1a + djb2 finalised
  with fmix32), matching the Rust crate's scheme.
- Accessors and sizing helpers: `bloom_bit_count`, `bloom_hash_count`,
  `bloom_bits_set`, `bloom_size_bytes`, `bloom_fill_ratio`,
  `bloom_estimated_false_positive_rate`, `bloom_is_over_capacity`,
  `bloom_optimal_m`, `bloom_optimal_k`, `bloom_capacity_for_memory`.
- A self-contained, libm-free natural log (range reduction + `2·atanh` series)
  so the optimal-m/k sizing needs no external math library; `ratio^k` computed
  with a multiply loop instead of `pow`.
- Overflow-guarded, `calloc`'d bit-array allocation.
- Tests mirroring the Rust suite (no false negatives, consistent stats, sizing
  formulas, parameter validation), run under GCC and Clang via `iso-harness`.
