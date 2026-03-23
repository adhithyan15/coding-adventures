# Changelog

All notable changes to the bitset-wasm package will be documented in this file.

## [0.1.0] - 2026-03-23

### Added

- Initial release of the WASM bitset extension.
- `WasmBitset` wrapper struct exported via `wasm-bindgen`.
- Constructors: `new(size)`, `fromInteger(n)`, `fromBinaryStr(s)`.
- Bit manipulation: `set`, `clear`, `test`, `toggle`.
- Bitwise operations: `and`, `or`, `xor`, `not`, `andNot`.
- Query methods: `popcount`, `len`, `capacity`, `any`, `all`, `none`, `isEmpty`.
- Iteration: `iterSetBits` returns a JS array of set bit indices.
- Conversion: `toInteger` (returns number or null), `toBinaryStr`.
- Native Rust unit tests guarded with `#[cfg(not(target_arch = "wasm32"))]`.
- BUILD file for build-tool integration.
