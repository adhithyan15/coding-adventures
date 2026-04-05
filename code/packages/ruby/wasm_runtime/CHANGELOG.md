# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-05

### Added

- Complete WasmRuntime composing parser, validator, and execution engine
- WasmInstance struct representing a live WASM module instance
- Runtime#load: parse .wasm binary bytes
- Runtime#validate: structural validation
- Runtime#instantiate: resolve imports, allocate memory/tables/globals, apply data/element segments
- Runtime#call: invoke exported functions by name with automatic type conversion
- Runtime#load_and_run: convenience all-in-one method
- WasiStub: minimal WASI implementation (fd_write, proc_exit)
- ProcExitError for clean WASM program termination
- End-to-end tests: square(5)=25, square(0)=0, square(-3)=9
