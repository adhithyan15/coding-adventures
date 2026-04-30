# Changelog

## [0.6.0] — 2026-04-27

### Added — LANG20: `WASMCodeGenerator` — `CodeGenerator[IrProgram, WasmModule]` adapter

**New module: `ir_to_wasm_compiler.generator`**

- `WASMCodeGenerator` — thin adapter satisfying the
  `CodeGenerator[IrProgram, WasmModule]` structural protocol (LANG20).

  ```
  [Optimizer] → [WASMCodeGenerator] → WasmModule
                                        ├─→ encode() → bytes → .wasm file  (AOT)
                                        └─→ [WASM runtime / Wasmer]        (JIT/sim)
  ```

  - `name = "wasm"` — unique backend identifier.
  - `validate(ir) -> list[str]` — delegates to `validate_for_wasm()`.  Never
    raises; returns `[]` for valid programs.
  - `generate(ir) -> WasmModule` — delegates to `IrToWasmCompiler().compile(ir)`.
    Returns a structured WASM 1.0 module; call `.encode()` for raw bytes.

- `WASMCodeGenerator` exported from `ir_to_wasm_compiler.__init__`.

**New tests: `tests/test_codegen_generator.py`** — 11 tests covering: `name`,
`isinstance(gen, CodeGenerator)` structural check, `validate()` on valid /
bad-opcode / unsupported-SYSCALL IR, `generate()` returns `WasmModule`,
encoded bytes start with WASM magic `\x00asm`, non-empty encoded output,
round-trip, export check.

---

## [Unreleased]

### Added

- Lowered `IrOp.F64_SQRT` to the native WASM `f64.sqrt` instruction and
  inferred its destination register as `f64`.
- Lowered `IrOp.F64_SIN`, `IrOp.F64_COS`, `IrOp.F64_ATAN`, `IrOp.F64_LN`,
  and `IrOp.F64_EXP` through typed `compiler_math` host imports and inferred
  their destination registers as `f64`.
- **Oct 8-bit arithmetic e2e tests** (`tests/test_oct_8bit_e2e.py`):
  7 end-to-end tests confirming the WASM backend correctly compiles and
  executes 8-bit integer arithmetic IR — the same IR that the Oct compiler
  generates.  Tests cover: LOAD_IMM, ADD, SUB, AND (inc. 0xFF masking),
  multi-output programs, and validation of Oct's unsupported SYSCALL numbers.
  Key findings:
  - Pure 8-bit arithmetic (LOAD_IMM/ADD/SUB/AND) compiles and runs correctly
    through the full IR → WASM → WASI pipeline.
  - Oct's I/O intrinsics (``out(PORT, val)`` → SYSCALL 40+PORT,
    ``in(PORT)`` → SYSCALL 20+PORT) are Intel 8008-specific and are
    correctly rejected by the WASM validator with a clear error message.
    To target WASM from Oct, I/O would need to use WASM's WASI ABI
    (SYSCALL 1/2/10) instead.

- **`IrOp.OR` / `IrOp.OR_IMM`**: bitwise OR in register-register and
  register-immediate forms.  Lowers to WASM `i32.or`.
- **`IrOp.XOR` / `IrOp.XOR_IMM`**: bitwise XOR in register-register and
  register-immediate forms.  Lowers to WASM `i32.xor`.
- **`IrOp.NOT`**: bitwise complement (one-operand form).  WASM has no dedicated
  NOT opcode; the backend emits `i32.xor` with the all-ones mask `0xFFFFFFFF`
  which flips every bit of the 32-bit value.
- `i32.or` and `i32.xor` added to the `_OPCODE` lookup table.
- 9 new runtime tests covering OR, OR_IMM, XOR, XOR_IMM, and NOT with
  multiple input cases each.

### Changed

- `_WASM_SUPPORTED_OPCODES` frozenset extended to include `OR`, `OR_IMM`,
  `XOR`, `XOR_IMM`, and `NOT`.  The V1 validator no longer rejects programs
  that use these opcodes.
- `CALL` lowering now accepts optional explicit argument registers after the
  target label. Calls without explicit operands keep the legacy v2, v3, ...
  convention.
- Function signatures can require explicit call operands for generated callers
  that must not fall back to the legacy v2, v3, ... convention.
- F64-returning functions now return through the dedicated f64 scratch register
  so real results no longer conflict with integer call results in `v1`.

## [0.5.0] — 2026-04-20

### Added

- **`validate_for_wasm(program)` pre-flight validator**: inspects an
  `IrProgram` for WASM backend incompatibilities *before* any module bytes
  are generated.  Returns a list of human-readable error strings (empty list
  = valid).  Three rules are checked:
  1. **Opcode support** — every opcode must appear in the V1 supported set.
     Currently all `IrOp` values are handled; the check is future-proofing
     against new IR opcodes added before the WASM backend implements them.
  2. **Constant range** — `LOAD_IMM` and `ADD_IMM` immediates must fit in a
     WASM `i32` (−2 147 483 648 to 2 147 483 647).
  3. **SYSCALL number** — only SYSCALL 1 (`fd_write`), SYSCALL 2 (`fd_read`),
     and SYSCALL 10 (`proc_exit`) are wired up in the V1 WASM backend.
- `validate_for_wasm` exported from `ir_to_wasm_compiler.__init__`.
- `TestValidateForWasm` test class (8 tests) covering all three rules,
  boundary-value constants, and integration with `IrToWasmCompiler.compile`.

### Changed

- `IrToWasmCompiler.compile()` now calls `validate_for_wasm()` as a
  pre-flight check before any WASM module bytes are produced.  Any violation
  raises `WasmLoweringError` with message prefix
  `"IR program failed WASM pre-flight validation"`.
- Added word-size constants `_WASM_I32_MIN = -(1 << 31)` and
  `_WASM_I32_MAX = (1 << 31) - 1` for single-source-of-truth boundary checks.

## [0.4.0] — 2026-04-20

### Changed

- **`syscall_arg_reg` parameter removed from `IrToWasmCompiler.compile()`.**
  The SYSCALL IR instruction now carries the arg register as `operands[1]`
  (an `IrRegister`), so the backend reads the register index directly from the
  instruction rather than from a config parameter.  Callers that previously
  passed `syscall_arg_reg=0` or `syscall_arg_reg=4` should remove that
  keyword argument; the correct register is now embedded in the IR.

- **`_emit_wasi_write`, `_emit_wasi_read`, `_emit_wasi_exit` now accept
  `arg_reg: int` directly.**  The private helpers previously closed over
  `self.syscall_arg_reg`; they now receive the register index from
  `_emit_syscall`, which extracts it from `instruction.operands[1]`.

- **`_make_function_ir` no longer inflates `max_reg` for the syscall arg.**
  The old code manually added `syscall_arg_reg` to `max_reg` so that WASM
  would allocate a local for it.  Now that the SYSCALL instruction carries
  `IrRegister(arg_reg)` as an operand, the normal operand scan already finds
  the register and includes it in the local-variable allocation.

## [0.3.0] — 2026-04-19

### Fixed

- **Brainfuck SYSCALL register mismatch**: `_SYSCALL_ARG0` was previously
  hard-coded to 0 (BASIC convention), breaking `brainfuck-wasm-compiler` which
  uses register 4 as the SYSCALL print argument.  The constant is restored to 4
  (Brainfuck default) and the lowerer now accepts an explicit `syscall_arg_reg`
  parameter so BASIC can pass `syscall_arg_reg=0` without affecting Brainfuck.

- **WASI errno clobber** (`_emit_wasi_write` / `_emit_wasi_read`): the errno
  return value from `fd_write`/`fd_read` is now discarded with `drop` instead
  of being stored in `_REG_SCRATCH` (register 1).  Previously, every `PRINT`
  in a Brainfuck or BASIC program silently zeroed register 1, corrupting
  variable values mid-loop (e.g., Fibonacci producing `0,1,1,1,1...`).

## [0.2.0] — 2026-04-19

### Added

- **Dispatch-loop lowering strategy** (`_DispatchLoopLowerer`): a second
  lowering strategy that handles arbitrary unstructured control flow (arbitrary
  `JUMP` and `BRANCH_Z`/`BRANCH_NZ` targets).  Enabled via
  `IrToWasmCompiler.compile(..., strategy="dispatch_loop")`.

  The strategy wraps all function segments in a `block { loop { … } }` dispatch
  table driven by a virtual program counter (`$pc`) stored in a WASM `i32`
  local.  Each IR `LABEL` becomes a "segment block" that is skipped unless
  `$pc` matches its index.  `JUMP` / `BRANCH` instructions set `$pc` and
  restart the loop; `HALT`/`RET` break out of the outer block.  See
  `code/specs/IR02-dispatch-loop-wasm-strategy.md` for the full design.

- **`strategy` parameter** on `IrToWasmCompiler.compile()` (keyword-only,
  default `"structured"`):
  - `"structured"` — existing behaviour; raises `WasmLoweringError` on
    unstructured control flow.
  - `"dispatch_loop"` — new behaviour; handles arbitrary jumps.
  - Any other value raises `WasmLoweringError: unknown lowering strategy`.

- `TestDispatchLoopLowerer` test class in `tests/test_ir_to_wasm_compiler.py`
  covering: simple HALT, forward JUMP, backward JUMP (loop), `BRANCH_NZ`
  taken/not-taken, `BRANCH_Z` taken, fall-through between segments, unknown
  strategy error, SYSCALL through the dispatch loop, and a mixed multi-jump
  program.

## [0.1.0] — 2026-04-13

### Added

- Initial release of `ir-to-wasm-compiler`.
- `IrToWasmCompiler.compile(program, function_signatures)` — lowers a generic
  `IrProgram` to a `WasmModule` targeting the WASI preview-1 ABI.
- `_FunctionLowerer` — structured control-flow lowerer that recognises
  `loop_N_start`/`loop_N_end` and `if_N_else`/`if_N_end` label patterns.
- Full WASI support: `fd_write` (SYSCALL 1), `fd_read` (SYSCALL 2),
  `proc_exit` (SYSCALL 10).
- `FunctionSignature` dataclass; `infer_function_signatures_from_comments()`
  helper.
- `WasmLoweringError` exception.
- Support for `i32.mul` and `i32.div_s` opcodes (`IrOp.MUL`, `IrOp.DIV`).
- `_SYSCALL_ARG0 = 4` default (Brainfuck IR convention: register 4 holds the
  SYSCALL print argument).

### Changed

- `IrToWasmCompiler.compile()` now accepts a `syscall_arg_reg: int` keyword
  parameter (default 4, the Brainfuck register convention).  Callers whose IR
  uses a different register for the SYSCALL argument (e.g. Dartmouth BASIC IR
  which uses register 0) must pass `syscall_arg_reg=0` explicitly.  This
  makes the WASM lowerer IR-frontend-agnostic instead of hard-coding one
  convention.
