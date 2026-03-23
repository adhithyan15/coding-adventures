# Changelog

All notable changes to the `coding-adventures-bitset` package will be
documented in this file.

## [0.1.0] - 2026-03-23

### Added

- Initial release of the pure Python bitset implementation.
- `Bitset` class with `list[int]` storage using 64-bit words.
- Constructors: `Bitset(size)`, `from_integer(value)`, `from_binary_str(s)`.
- Single-bit operations: `set`, `clear`, `test`, `toggle` with ArrayList-style
  auto-growth on `set` and `toggle`.
- Bulk bitwise operations: `bitwise_and`, `bitwise_or`, `bitwise_xor`,
  `bitwise_not`, `and_not` -- all return new bitsets.
- Operator overloads: `&`, `|`, `^`, `~`.
- Counting and queries: `popcount`, `any`, `all`, `none`, `capacity`.
- Iteration: `iter_set_bits` with trailing-zero-count trick for efficiency.
- Conversion: `to_integer`, `to_binary_str`.
- Python protocols: `__len__`, `__contains__`, `__iter__`, `__eq__`,
  `__hash__`, `__repr__`.
- `BitsetError` exception for invalid inputs.
- PEP 561 `py.typed` marker for type checker support.
- Comprehensive test suite with 95%+ coverage.
- Literate programming style with inline explanations, truth tables,
  and diagrams.
