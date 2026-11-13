# Changelog

All notable changes to the `coding_adventures_bitset_native` gem will be documented in this file.

## Unreleased

### Fixed

- Windows link: the `build.rs` import-library symbol list now includes every `rb_*` function `ruby-bridge` declares (it was missing `rb_eval_string`, `rb_str_new`, `rb_num2dbl`, `rb_float_new`, `rb_string_value_ptr`, `rb_define_module_function`, `rb_hash_new`, `rb_hash_aset`), so the cdylib no longer fails with `rust-lld: undefined symbol: rb_eval_string ...` on the Windows runner (recorded as a Windows-only failure for this package).

- `to_integer`'s `-1` sentinel goes through `ruby_bridge::rb_int2inum`, which now takes an `intptr_t`: on 64-bit Windows the old `c_long` declaration returned it as `4294967295` (this is the likely cause of the Windows-only failure recorded for this package).

## [0.1.0] - 2026-03-23

### Added

- Initial release wrapping the Rust `bitset` crate via `ruby-bridge`
- `Bitset.new(size)` constructor with zero-initialized bits
- `Bitset.from_integer(n)` factory from non-negative integer
- `Bitset.from_binary_str(s)` factory from binary string (e.g., `"1010"`)
- Single-bit operations: `set`, `clear`, `test?`, `toggle`
- Auto-growth on `set` and `toggle` beyond current length
- Bulk bitwise operations: `and`, `or`, `xor`, `not`, `and_not`
- Counting/query methods: `popcount`, `len`, `capacity`, `any?`, `all?`, `none?`, `empty?`
- Iteration via `each_set_bit` returning array of set bit indices
- Conversion methods: `to_integer`, `to_binary_str`, `to_s`
- Equality comparison via `==`
- `BitsetError < StandardError` for domain-specific errors
- Comprehensive test suite with 50+ test cases
