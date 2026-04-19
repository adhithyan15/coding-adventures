# Changelog

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
- `_SYSCALL_ARG0 = 0` (corrected from 4 — register 0 is the SYSCALL argument
  register).
