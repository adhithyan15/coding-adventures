# Changelog

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
