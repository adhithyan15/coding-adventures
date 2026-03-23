# Changelog

All notable changes to the immutable-list-wasm package will be documented in this file.

## [0.1.0] - 2026-03-23

### Added

- Initial release of the WASM immutable-list extension.
- `WasmImmutableList` wrapper struct exported via `wasm-bindgen`.
- Constructor: `new()` creates an empty persistent list.
- Persistent operations: `push(value)`, `set(index, value)`, `pop()` -- all return new lists.
- Query: `get(index)` returns string or undefined, `length()`, `isEmpty()`.
- Conversion: `toArray()` materializes to a JS string array, `fromArray(arr)` constructs from a JS array.
- Native Rust unit tests guarded with `#[cfg(not(target_arch = "wasm32"))]`.
- BUILD file for build-tool integration.
