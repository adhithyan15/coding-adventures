# Changelog — wasm-backend

All notable changes to this package will be documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.1] — 2026-04-27

### Added

- **`TestJITTypeInference` test class (4 tests)** — demonstrates the full JIT
  type-inference loop for untyped Tetrad programs:
  - `test_untyped_helper_compiled_after_100_calls`: an untyped `sum_pair(a,b)`
    function is called 100× in a `while` loop inside `main()`.  After one
    `execute_with_jit`, `_promote_hot_functions()` sees the 100-call count
    (≥ UNTYPED threshold), trusts the profiler's `u8` observations (≥
    `min_observations=5`), specialises to `add_u8` CIR, and compiles to WASM.
    Cross-validates that `sum_pair` is compiled and the result is correct
    (sum 0…99 = 4950 → 86 mod 256).
  - `test_below_threshold_not_compiled`: 50 iterations → 50 calls → below
    threshold (100) → no JIT compilation.  Verifies the gate works correctly.
  - `test_jit_result_matches_interpreter`: confirms the Phase 2 interpreter
    result equals `TetradRuntime.run()` (pure interpreter), and verifies the
    post-compilation CIR contains `add_u8` (type inference succeeded).
    Documents the architectural limitation: subroutine JIT handlers cannot
    receive arguments in the current `param_count=0` model.
  - `test_complex_multi_helper_program`: three-function call graph.  Leaf
    functions (`double`, `add`) compile to WASM; `compute` (which calls both)
    does not — WASMBackend creates standalone single-function modules and
    cannot link cross-function `IrOp.CALL` references.  Verifies `main`
    result matches the interpreter.

### Changed

- Test count increased from 53 → 57; coverage unchanged at 94%.

### Implementation notes

- **TETRAD_OPCODE_EXTENSIONS required for untyped programs**: tests that use
  `JITCore` with untyped Tetrad programs MUST create the VM via
  `TetradRuntime()._make_vm()` (not bare `VMCore(opcodes={})`).  The Tetrad
  IIR translator emits `tetrad.move` instructions (register-to-register copies)
  that have no handler in the bare dispatch table.  TETRAD_OPCODE_EXTENSIONS
  registers this handler.

- **Standalone WASM module limit**: `WASMBackend.compile()` wraps the CIR in
  a single-function WASM module.  Functions that contain `call` CIR opcodes
  (which LANG21 lowers to `IrOp.CALL`) cannot be compiled — the WASM linker
  has no other function in the module to call.  This limits JIT compilation
  to *leaf functions* (pure arithmetic, no sub-calls).  A future extension
  would compile the entire call graph into a multi-function WASM module.

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
