# Changelog

All notable changes to this project will be documented in this file.

## [0.1.0] - 2026-03-23

### Added

- Initial release of the native bitset extension
- `Bitset` class wrapping the Rust `bitset` crate via `python-bridge`
- Constructor: `Bitset(size=0)` with auto-growth on `set`/`toggle`
- Class methods: `from_integer(value)`, `from_binary_str(s)`
- Single-bit operations: `set(i)`, `clear(i)`, `test(i)`, `toggle(i)`
- Bulk bitwise operations: `bitwise_and`, `bitwise_or`, `bitwise_xor`, `bitwise_not`, `and_not`
- Query methods: `popcount()`, `capacity()`, `any()`, `all()`, `none()`
- Iteration: `iter_set_bits()` and `__iter__` protocol
- Conversion: `to_integer()`, `to_binary_str()`
- Python protocols: `__len__`, `__contains__`, `__repr__`, `__eq__`, `__hash__`
- Operator overloads: `&`, `|`, `^`, `~`
- `BitsetError` exception for invalid inputs
- Comprehensive test suite mirroring the pure Python bitset tests
