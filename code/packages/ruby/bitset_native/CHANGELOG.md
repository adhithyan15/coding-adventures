# Changelog

All notable changes to the `coding_adventures_bitset_native` gem will be documented in this file.

## Unreleased

### Fixed

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
