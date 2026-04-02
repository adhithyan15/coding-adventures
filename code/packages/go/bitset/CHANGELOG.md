# Changelog

All notable changes to the Go bitset package will be documented in this file.

## [0.2.0] - 2026-03-31

### Changed

- **Operations system integration**: All public functions and methods (`NewBitset`,
  `BitsetFromInteger`, `BitsetFromBinaryStr`, `Set`, `Clear`, `Test`, `Toggle`,
  `And`, `Or`, `Xor`, `Not`, `AndNot`, `Popcount`, `Len`, `Capacity`, `Any`,
  `All`, `None`, `IterSetBits`, `ToInteger`, `ToBinaryStr`, `String`, `Equal`)
  are now wrapped with `StartNew[T]` from the package's Operations infrastructure.
  Each call gains automatic timing, structured logging, and panic recovery.

## [0.1.0] - 2026-03-23

### Added

- Initial implementation of the Go bitset package.
- `Bitset` struct with `[]uint64` storage and logical length tracking.
- Constructors: `NewBitset(size)`, `BitsetFromInteger(uint64)`, `BitsetFromBinaryStr(string)`.
- Single-bit operations: `Set`, `Clear`, `Test`, `Toggle` with ArrayList-style auto-growth.
- Bulk bitwise operations: `And`, `Or`, `Xor`, `Not`, `AndNot` -- all return new bitsets.
- Query operations: `Popcount` (using `math/bits.OnesCount64`), `Len`, `Capacity`, `Any`, `All`, `None`.
- Iteration: `IterSetBits` returns `[]int` of set bit indices using trailing-zero-count trick.
- Conversion: `ToInteger`, `ToBinaryStr`, `String` (implements `fmt.Stringer`).
- Equality: `Equal` method comparing length and word contents.
- `BitsetError` type for invalid binary strings and overflow errors.
- Clean-trailing-bits invariant maintained across all operations.
- Comprehensive test suite with 95%+ coverage.
- Literate programming style with inline explanations and diagrams.
