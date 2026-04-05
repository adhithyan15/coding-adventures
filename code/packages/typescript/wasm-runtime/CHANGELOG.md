# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-05

### Added

- WasmRuntime: complete pipeline from .wasm bytes to execution results.
  - load(): parse .wasm binary via WasmModuleParser.
  - validate(): semantic validation via wasm-validator.
  - instantiate(): allocate memory/tables/globals, resolve imports,
    apply data/element segments, call start function.
  - call(): call exported function by name with number[] args → number[] results.
  - loadAndRun(): convenience one-shot method.
- WasmInstance: holds all runtime state (memory, tables, globals, exports).
- WasiStub: minimal WASI host implementation.
  - fd_write: captures stdout/stderr output via callbacks.
  - proc_exit: throws ProcExitError with exit code.
  - All other WASI functions return ENOSYS (52) — clearly documented as stub.
- End-to-end test: hand-assembled square(n)=n*n module passes all cases
  including i32 overflow wrapping (square(2147483647)=1).
