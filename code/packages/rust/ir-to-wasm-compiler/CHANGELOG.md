# Changelog

## [0.2.0] — 2026-04-29

### Added

- **LANG20 `WasmCodeGenerator`** — new `codegen` module implementing
  `CodeGenerator<IrProgram, WasmModule>` from `codegen-core`.
  - `name()` → `"wasm"`
  - `validate(ir)` — dry-run compile; returns errors as `Vec<String>`
  - `generate(ir)` → `WasmModule` (panics on invalid IR — always call `validate` first)
  - 8 unit tests + 1 doc-test

### Changed

- Added `codegen-core` to `[dependencies]` to enable the `CodeGenerator` trait implementation.

## [0.1.0] — Initial release

- `IrToWasmCompiler::compile(program, signatures)` — lower `IrProgram` to `WasmModule`
- `infer_function_signatures_from_comments` — infer `FunctionSignature` from `COMMENT` IR instructions
