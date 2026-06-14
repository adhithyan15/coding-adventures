# Changelog

## [0.3.0] — 2026-05-11

### Added — Bitwise OR, XOR, NOT opcodes

- **`IrOp::Or`** — lowered to WASM `i32.or`.
- **`IrOp::OrImm`** — register + immediate OR; immediate pushed with `i32.const`.
- **`IrOp::Xor`** — lowered to WASM `i32.xor`.
- **`IrOp::XorImm`** — register + immediate XOR.
- **`IrOp::Not`** — one's-complement NOT synthesised as `i32.const -1; i32.xor`
  (WASM has no native bitwise-NOT instruction).
- 6 new unit tests: `or_produces_correct_result`, `or_imm_produces_correct_result`,
  `xor_produces_correct_result`, `xor_imm_produces_correct_result`,
  `not_inverts_all_bits`, `not_one_gives_minus_two`.

---

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
