# Changelog

All notable changes to the `coding_adventures_bitset` Ruby gem will be documented in this file.

## [0.1.0] - 2026-03-23

### Added

- Initial release of the pure Ruby bitset implementation
- `Bitset.new(size)` constructor with ArrayList-style capacity doubling
- `Bitset.from_integer(value)` constructor from non-negative integers
- `Bitset.from_binary_str(str)` constructor from binary strings
- Single-bit operations: `set`, `clear`, `test?`/`test`, `toggle`
- Auto-growth semantics: `set` and `toggle` grow the bitset when addressing bits beyond current length
- Bulk bitwise operations: `bitwise_and`, `bitwise_or`, `bitwise_xor`, `bitwise_not`, `and_not`
- Ruby operator overloads: `&`, `|`, `^`, `~`
- Counting and query methods: `popcount`, `size`, `capacity`, `any?`, `all?`, `none?`, `empty?`
- `each_set_bit` iterator using trailing-zero-count trick for efficient sparse iteration
- Conversion methods: `to_integer`, `to_binary_str`, `to_s`/`inspect`
- Equality comparison via `==`, `eql?`, `hash`
- `BitsetError` exception class for invalid binary strings
- Clean-trailing-bits invariant maintained across all operations
- Comprehensive test suite with 95%+ coverage
