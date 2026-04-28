# Changelog — wasm-backend

All notable changes to this package will be documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] — 2026-04-27

### Added

- `WASMBackend` class implementing `BackendProtocol` from `codegen-core`.
  - `name = "wasm"` class attribute (required by `BackendProtocol` for
    diagnostics; satisfies `isinstance(WASMBackend(), BackendProtocol)`).
  - `compile(cir: list[CIRInstr]) -> bytes | None`
    Lowers CIR → IrProgram (LANG21) → WasmModule → bytes (LANG20).
    Always uses `"_start"` as the IrProgram entry label (required by the WASM
    compiler's `_split_functions`); exports under `self.entry_label` via
    `FunctionSignature.export_name`.
    Handles the WASM return-value fixup automatically.
  - `run(binary: bytes, args: list[Any]) -> Any`
    Executes a WASM binary on `WasmRuntime` and returns the first result.
- `_collect_cir_registers(cir)` — internal helper that mirrors LANG21's Pass 1
  to find the register index for the return variable before lowering.
- Return-value fixup: inserts `ADD_IMM IrRegister(1), result_reg, 0` before
  HALT when the result is not already in register 1 (the WASM scratch register).
- Integration with `TetradRuntime.run_with_jit(source, backend=WASMBackend())`.
- 53 tests covering unit CIR compilation, full Tetrad → WASM pipeline,
  `BackendProtocol` compatibility, register fixup edge cases, and JITCore
  integration.  Coverage: 94%.

### Implementation notes

- **Control flow**: the WASM compiler's structured strategy requires branch
  targets to match `if_N_else` / `if_N_end` naming.  Test CIRs and any
  hand-crafted programs must use this convention; free-form labels like `"taken"`
  cause `WasmLoweringError` which `compile()` catches and converts to `None`
  (deopt signal).

- **`tetrad.move` support**: handled by LANG21 (`cir-to-compiler-ir` v0.1.1);
  allows JIT-specialised multiplication and other multi-register programs to
  compile correctly.

- **`BackendProtocol` isinstance**: the `name = "wasm"` class attribute
  satisfies the `Backend` Protocol's `name: str` requirement, enabling the
  `isinstance(WASMBackend(), BackendProtocol)` check to pass.
