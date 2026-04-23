# wasm-module-parser

Native Kotlin parser for WebAssembly 1.0 binary modules.

This package provides:

- `WasmModuleParser` to parse `.wasm` bytes into `WasmModule`
- `WasmParseError` with byte offsets for malformed input
