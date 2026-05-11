# Changelog

## [0.3.0] — 2026-05-11

**Rust WASM backend v1 — `MUL`, `DIV`, and `validate_for_wasm`.**

Closes the gap between the Rust backend and the Python v0.6.0 feature set
for the integer arithmetic opcodes already present in `compiler-ir`.

### New opcodes

- **`MUL`** — lowers to `i32.mul`.  Wraps on overflow (two's complement),
  matching the `compiler-ir` contract.
- **`DIV`** — lowers to `i32.div_s`.  Truncates toward zero (C-style).
  The WASM spec mandates a trap on division-by-zero *and* on the
  signed-overflow case (INT_MIN / -1), satisfying the IrOp requirement that
  "silently returning 0 is explicitly forbidden".

### New public API

- **`validate_for_wasm(program: &IrProgram) -> Vec<String>`** — standalone
  pre-flight validation function that mirrors the Python backend's
  `validate_for_wasm`.  Returns an empty `Vec` when the program can be
  lowered without errors, or a single-element `Vec` with the error message.
  Used by the LANG21 `cir-to-compiler-ir` round-trip tests and by any
  caller that wants to separate validation from code generation.

### Tests added (11 new)

- `lowers_mul_to_i32_mul` — compile-only smoke test
- `mul_produces_correct_result` — 6 × 7 = 42
- `mul_wraps_on_overflow` — i32::MAX × 2 = -2
- `mul_by_zero_produces_zero`
- `lowers_div_to_i32_div_s` — compile-only smoke test
- `div_produces_correct_result` — 20 / 4 = 5
- `div_truncates_toward_zero` — 7 / 2 = 3 (not 3.5 or 4)
- `div_negative_truncates_toward_zero` — -7 / 2 = -3
- `validate_for_wasm_returns_empty_on_valid_program`
- `validate_for_wasm_returns_errors_for_bad_syscall`
- `validate_for_wasm_accepts_mul_and_div`

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
