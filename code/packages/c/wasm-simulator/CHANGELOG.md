# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-12

### Added

- Initial pure-ISO C17 port of the Rust `wasm-simulator` crate: a stack-based
  WebAssembly virtual machine (i32 subset).
- `wasm_decode` (variable-length instruction decode), `WasmSimulator`
  (`new`/`free`/`load`/`step`/`run` + stack/locals/pc/halted/cycle accessors)
  producing a `WasmStepTrace` per instruction, and a `WasmProgram` bytecode
  assembler (`wasm_emit_*`).
- Arithmetic wraps modulo 2^32; Rust panics (unknown opcode, truncated code,
  stack underflow, stepping a halted VM, out-of-range local) become `WasmStatus`
  codes. Trace snapshots and growable buffers are malloc-owned and
  overflow-guarded.
- Tests via the shared `iso-harness` (GCC, Clang, MSVC): the crate's full-program
  vector (push/add/store/load/const/sub/end → stack [-2], locals[0]=3, 8 traces),
  decode, the error paths, and 2^32 wrapping.
