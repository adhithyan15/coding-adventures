# Changelog

All notable changes to the `bitset` package will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-03-23

### Added

- Core `Bitset` struct backed by `Vec<u64>` with ArrayList-style doubling growth.
- Constructors: `new(size)`, `from_integer(u128)`, `from_binary_str(&str)`.
- Single-bit operations: `set(i)`, `clear(i)`, `test(i)`, `toggle(i)` with auto-growth for set/toggle.
- Bulk bitwise operations: `and`, `or`, `xor`, `not`, `and_not` -- all return new bitsets.
- Operator overloading: `BitAnd`, `BitOr`, `BitXor`, `Not` traits for `&`, `|`, `^`, `!` syntax.
- Counting operations: `popcount()`, `len()`, `capacity()`, `any()`, `all()`, `none()`.
- Efficient set-bit iteration via `iter_set_bits()` using trailing-zeros trick.
- Conversion: `to_integer() -> Option<u64>`, `to_binary_str()`, `Display` trait.
- Equality: `PartialEq`, `Eq` implementations.
- Error type: `BitsetError::InvalidBinaryString`.
- Clean-trailing-bits invariant maintained after `not()`, `toggle()`, `from_binary_str()`.
- Comprehensive test suite (60+ tests) covering constructors, single-bit ops, bulk ops,
  truth tables, counting, iteration, conversions, edge cases, and algebraic property tests
  (commutativity, associativity, De Morgan's laws, distributive law, idempotence).
- Literate programming style: extensive inline comments explaining every operation,
  with diagrams, truth tables, and worked examples.
