# Changelog — vm-core

## [0.1.0] — 2026-04-27

Initial Rust port of the Python `vm-core` package (LANG02).

### Added

- `Value` enum — `Int(i64) | Float(f64) | Bool(bool) | Str(String) | Null`.
  `iir_type_name()` performs range-aware integer classification
  (`0–255 → "u8"`, `0–65535 → "u16"`, …).

- `VMError` — `UnknownOpcode`, `FrameOverflow`, `UndefinedVariable`,
  `TypeError`, `DivisionByZero`, `UndefinedLabel`, `Custom`.

- `VMFrame` — per-call state: flat register file (`Vec<Value>`), variable
  name → register index map (`HashMap<String, usize>`), instruction pointer,
  and caller return-destination register.  `assign()` grows the register file
  on demand (no bounds-error on well-formed IIR).

- `VMProfiler` — observes runtime `Value` types for `"any"`-typed instructions
  and records them in the instruction's `SlotState`.  Supports custom type
  mapper functions (`VMProfiler::with_mapper`).

- `BuiltinRegistry` — named built-in handlers callable via `call_builtin`.
  Pre-registered: `noop`, `assert_eq`, `print`.

- `DispatchCtx` — all mutable execution state in one struct (frame stack,
  module functions, flat memory, builtins, counters).  `extra_opcodes` and
  `jit_handlers` are intentionally **not** fields — they are passed as
  separate `&HashMap` references to the dispatch loop to avoid Rust
  borrow-checker conflicts when handler closures also need to mutate ctx.

- Standard opcode handlers — `const`, `add/sub/mul/div/mod/neg`,
  `and/or/xor/not/shl/shr`, `cmp_eq/ne/lt/le/gt/ge`, `label/jmp/jmp_if_true/
  jmp_if_false`, `ret/ret_void`, `load_reg/store_reg`, `load_mem/store_mem`,
  `call/call_builtin`, `io_in/io_out`, `cast`, `type_assert`.

- `VMCore` — public execution API: `execute()`, `register_jit_handler()`,
  `register_opcode()`, `builtins_mut()`, `metrics_instrs()`,
  `metrics_jit_hits()`, `fn_call_counts()`, `total_observations()`.

- `u8_wrap` mode — masks all arithmetic results with `& 0xFF` for Tetrad
  8-bit register semantics.

- 29 unit tests + 6 doctests.

### Architecture notes

The borrow-checker challenge: the dispatch loop needs `&mut DispatchCtx` (to
mutate frame state) AND needs to call handlers that also take `&mut DispatchCtx`.
Solution: handlers take `&mut DispatchCtx` directly (no separate `&mut VMFrame`
parameter); each handler opens a nested block to release the frame borrow before
accessing other `DispatchCtx` fields.  Read-only lookup tables (`extra_opcodes`,
`jit_handlers`) are passed as separate parameters to `run_dispatch_loop`.
