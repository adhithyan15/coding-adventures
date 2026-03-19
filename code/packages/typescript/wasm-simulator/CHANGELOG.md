# Changelog

## 0.1.0 (2026-03-19)

### Added
- Initial TypeScript port from Python wasm-simulator
- WasmDecoder: variable-width instruction decoding
- WasmExecutor: stack-based instruction execution
- WasmSimulator: complete simulator with decode-execute-advance cycle
- Encoding helpers: encodeI32Const, encodeI32Add, encodeI32Sub, encodeLocalGet, encodeLocalSet, encodeEnd
- assembleWasm helper for building bytecode programs
- Full test suite ported from Python with vitest
- Knuth-style literate programming comments preserved from Python source
