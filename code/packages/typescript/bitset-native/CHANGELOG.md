# Changelog

All notable changes to `@coding-adventures/bitset-native` will be documented in this file.

## [0.1.0] - 2026-03-23

### Added

- Initial release wrapping the Rust `bitset` crate via `node-bridge` N-API FFI.
- `Bitset` class with three constructor modes: size, integer, binary string.
- Single-bit operations: `set`, `clear`, `test`, `toggle` (with auto-growth).
- Bulk bitwise operations: `and`, `or`, `xor`, `not`, `andNot` (return new instances).
- Query operations: `popcount`, `len`, `capacity`, `any`, `all`, `none`, `isEmpty`.
- Conversion: `iterSetBits`, `toInteger`, `toBinaryStr`.
- TypeScript type definitions (`index.d.ts`).
- Comprehensive vitest test suite with v8 coverage.
- BUILD file for the project build tool.
