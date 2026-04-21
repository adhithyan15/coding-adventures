# Changelog

All notable changes to `tetrad-vm` are documented here.

## [0.1.0] — 2026-04-20

### Added

- `TetradVM` — Tetrad bytecode interpreter built on `GenericRegisterVM` from the
  `register-vm` package.  Registers 39 opcode handlers (LDA/STA/ADD/SUB/MUL/DIV/MOD/
  AND/OR/XOR/NOT/SHL/SHR/AND_IMM/EQ/NEQ/LT/LTE/GT/GTE/LOGICAL_NOT/AND/OR/
  JMP/JZ/JNZ/JMP_LOOP/CALL/RET/IO_IN/IO_OUT/HALT).
- `VMError` — Tetrad-specific runtime exception.
- `_update_slot(slot, ty)` — feedback slot state machine (UNINITIALIZED →
  MONOMORPHIC → POLYMORPHIC → MEGAMORPHIC).
- `tetrad_vm.metrics` module — `SlotKind`, `SlotState`, `BranchStats`, `VMMetrics`,
  `VMTrace`.
- Feedback-vector system: each function gets a persistent `list[SlotState]` that
  accumulates type observations across multiple `execute()` calls.
- Metrics API: `hot_functions()`, `feedback_vector()`, `type_profile()`,
  `branch_profile()`, `loop_iterations()`, `call_site_shape()`, `metrics()`,
  `reset_metrics()`.
- `execute_traced(code)` — returns `(result, list[VMTrace])` with per-instruction
  snapshots including `feedback_delta` for slots that changed.
- Immediate-JIT queue — `TetradVM.execute()` appends fully-typed function names to
  `metrics.immediate_jit_queue` for zero-warmup AOT/JIT compilation.
- `io_in` / `io_out` constructor callbacks for `IO_IN` / `IO_OUT` instructions.

### Architecture note

This package deliberately avoids writing its own dispatch loop.  All fetch-
decode-execute logic lives in `register_vm.GenericRegisterVM`.  This means
future languages (Lisp, μScheme, …) can reuse the same chassis and any
debugger, profiler, or trace tooling added to `GenericRegisterVM` automatically
benefits every language backend.
