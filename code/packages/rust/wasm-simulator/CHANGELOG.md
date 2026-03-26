# Changelog

All notable changes to this project will be documented in this file.

## [0.1.0] - 2026-03-19

### Added

- `WasmDecoder` -- variable-length bytecode decoder for 6 WASM opcodes
- `WasmExecutor` -- stack-based execution engine with local variable support
- `WasmSimulator` -- full simulation environment with stack and locals
- Encoding helpers: `encode_i32_const`, `encode_i32_add`, `encode_i32_sub`, `encode_local_get`, `encode_local_set`, `encode_end`
- 32-bit wrapping arithmetic for add/sub operations
- Step trace recording with stack snapshots
