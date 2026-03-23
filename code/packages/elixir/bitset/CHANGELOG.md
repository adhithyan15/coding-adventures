# Changelog

All notable changes to this project will be documented in this file.

## [0.1.0] - 2026-03-23

### Added

- Initial implementation of the pure Elixir bitset package.
- `Bitset` struct with `words` (list of 64-bit integers) and `len` (logical size).
- Constructors: `new/1`, `from_integer/1`, `from_binary_str/1`, `from_binary_str!/1`.
- Single-bit operations: `set/2`, `clear/2`, `test?/2`, `toggle/2`.
- Bulk bitwise operations: `bitwise_and/2`, `bitwise_or/2`, `bitwise_xor/2`, `flip_all/1`, `difference/2`.
- Query operations: `popcount/1`, `size/1`, `capacity/1`, `any?/1`, `all?/1`, `none?/1`.
- Iteration: `set_bits/1` returning list of set bit indices in ascending order.
- Conversion: `to_integer/1`, `to_binary_str/1`.
- Equality: `equal?/2`.
- Protocol implementations: `String.Chars` (for `to_string/1`) and `Inspect` (for iex display).
- `BitsetError` exception for invalid binary string input.
- Comprehensive test suite with 80%+ coverage.
- Literate programming style with inline explanations, truth tables, and examples.
