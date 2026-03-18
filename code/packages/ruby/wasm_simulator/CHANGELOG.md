# Changelog

## [0.1.0] - 2026-03-18

### Added
- WasmDecoder: decodes i32.const, i32.add, i32.sub, local.get, local.set, end
- WasmExecutor: executes decoded WASM instructions against stack and locals
- Encoding helpers: encode_i32_const, encode_i32_add, encode_i32_sub, etc.
- WasmSimulator: standalone stack machine with load/step/run
- Immutable Data.define records: WasmInstruction, WasmStepTrace
