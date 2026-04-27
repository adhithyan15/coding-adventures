# wasm-validator

Native Java wrapper for WebAssembly module validation.

This package currently provides:

- `WasmValidator.validate(...)` to wrap a parsed `WasmModule`
- `ValidatedModule` as the validated handoff type for the runtime
