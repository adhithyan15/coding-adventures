# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-12

### Added

- WebAssembly bindings for `url-parser` via `wasm-bindgen`
- `WasmUrl` class with constructor, getters, `resolve()`, `effectivePort()`, `authority()`, `toUrlString()`
- `percentEncode()` and `percentDecode()` free functions
- 8 unit tests verifying the adapter layer
