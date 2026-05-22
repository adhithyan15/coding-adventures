# Changelog — vm-core

## [0.2.1] — 2026-05-22

### Added (LANG74 follow-up — universal `mov` dispatch)

- `dispatch.rs`: new `handle_mov` + `"mov" => Some(handle_mov)` entry in
  the standard opcode table.  Implements the IIR canonical
  `mov dest = src` semantics — resolve `src`, assign to the named slot
  `dest` in the current frame.
- Unblocks the JIT chain for frontends that emit `mov` directly
  (e.g. `dartmouth-basic-iir-compiler`, `oct-iir-compiler`).
  Previously these programs ran fine through `lang-aot` (the AOT
  specialiser rewrites `mov` to the typed `mov_<ty>` CIR variant the
  backends handle) but tripped `VMError::UnknownOpcode("mov")` the
  moment `VMCore::execute` saw them.

### Proof

`dartmouth-basic-iir-compiler` ships a new
`tests/jit_smoke.rs` that runs four BASIC programs (PRINT-only, LET +
arithmetic + PRINT, FOR/NEXT, IF/THEN/GOTO) through
`JITCore::execute_with_jit`, registering `print_i64` on a custom
`BuiltinRegistry` to capture output.  All four pass — meaning every
language in the LANG74 roadmap now runs end-to-end through **both** the
AOT chain (`lang-aot`) and the JIT chain (`vm-core` + `jit-core`).

## [0.2.0] — 2026-05-11

### Changed (LANG32 — Operand::Str exhaustiveness)

- `dispatch.rs`: `resolve_operand` now handles `Operand::Str(s)` — converts
  the compile-time string literal to `Value::Str(s.clone())`.

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
