# Changelog

All notable changes to `@coding-adventures/bitset` will be documented in this
file.

## [0.1.0] - 2026-03-23

### Added

- Initial release of the TypeScript bitset implementation
- `Bitset` class using `Uint32Array` with 32-bit words
- Constructors: `new Bitset(size)`, `Bitset.fromInteger(value)`,
  `Bitset.fromBinaryStr(s)`
- Single-bit operations: `set`, `clear`, `test`, `toggle`
- Bulk bitwise operations: `and`, `or`, `xor`, `not`, `andNot`
- Counting and queries: `popcount`, `size`, `capacity`, `any`, `all`, `none`,
  `isEmpty`
- Iteration: `iterSetBits` generator for efficient set-bit traversal
- Conversion: `toInteger`, `toBinaryStr`, `toString`
- Equality: `equals` method
- ArrayList-style automatic growth with doubling strategy
- Clean-trailing-bits invariant for correctness
- `BitsetError` class for error reporting
- Comprehensive test suite with 95%+ coverage
- Literate programming style with inline explanations
