# Changelog — vm-core

All notable changes to this package will be documented in this file.

---

## [Unreleased]

### Added — LANG17 PR3: ``execute_traced`` and the ``VMTracer`` helper

- `vm_core.tracer` — new module with `VMTrace` dataclass and
  `VMTracer` accumulator.  `VMTrace` captures one dispatch event:
  `frame_depth`, `fn_name`, `ip`, `instr` (reference), shallow copies
  of the register file before and after dispatch, and `slot_delta`
  recording any feedback-slot changes produced by this instruction.
- `VMCore.execute_traced(module, fn, args) -> (result, list[VMTrace])`
  — opt-in tracing path.  Runs the normal dispatch loop with a fresh
  `VMTracer` installed for the duration of the call, returns the
  function result alongside the accumulated trace records.  Normal
  `execute` pays zero tracing overhead.
- Re-exports `VMTrace` and `VMTracer` from the package root.

### Changed (PR3)

- `run_dispatch_loop` now consults `vm._tracer`; when set, snapshots
  registers + frame depth before dispatch, then records a `VMTrace`
  after.  Frame depth is captured *before* dispatch so `ret`
  instructions still report the correct depth even though the frame
  is popped during the same dispatch step.

### Added — LANG17 PR2: branch and loop iteration counters (already on main)

- `BranchStats` — new dataclass in `vm_core.metrics` holding
  `taken_count` / `not_taken_count` for one conditional-branch site,
  with derived `taken_ratio` and `total` properties.  JITs use
  `taken_ratio` to decide branch layout (hot-body inline vs.
  out-of-line).  Re-exported from the package root.
- `VMMetrics.branch_stats: dict[str, dict[int, BranchStats]]` —
  per-function per-branch-site counts, keyed by IIR instruction index.
- `VMMetrics.loop_back_edge_counts: dict[str, dict[int, int]]` —
  per-function per-back-edge iteration counts.  A back-edge is any
  jump whose target index is strictly less than the source index.
- `VMCore.branch_profile(fn_name, source_ip) -> BranchStats | None` —
  live counter lookup.
- `VMCore.loop_iterations(fn_name) -> dict[int, int]` — fresh copy of
  per-back-edge counts.
- `VMCore.hot_functions(threshold=100)` — function names whose call
  count meets the threshold.  JITs use this for tier promotion.
- `VMCore.reset_metrics()` — zero all aggregate counters including
  branch / loop state.  Does NOT reset per-IIRInstr observations
  (those live on the module).
- `VMCore.metrics()` now returns deep copies of the branch and loop
  dicts so callers can mutate the snapshot without affecting live
  state.

### Changed (PR2)

- `handle_jmp` now detects back-edges (target < source) and bumps the
  loop counter.  `handle_jmp_if_true` and `handle_jmp_if_false` now
  record (taken, not-taken) counters and also bump the loop counter
  when the branch is taken to an earlier index.

### Added — LANG17 PR1: feedback-slot state machine (already on main)

- `VMProfiler` gained a pluggable `type_mapper` parameter — a callable
  from runtime value to IIR type string.  Defaults to
  `default_type_mapper` (Python primitives).  Frontends hosting a
  non-primitive runtime (Lisp cons cells, Ruby tagged pointers, JS
  Values, etc.) pass a custom mapper so the profiler records
  language-specific type names in `SlotState.observations`.
- `VMCore(type_mapper=...)` constructor kwarg threads the mapper
  through to the profiler.
- `TypeMapper` and `default_type_mapper` are re-exported from the
  package root for consumers declaring their own mappers.
- The profiler now drives the V8 Ignition-style state machine on
  `IIRInstr.observed_slot` (defined in `interpreter-ir`).  Legacy
  `observed_type` / `observation_count` fields remain populated for
  backwards compatibility.

### Changed (PR1)

- `VMProfiler.__init__` signature is now `VMProfiler(type_mapper=None)`
  instead of `VMProfiler()`.  Callers passing no arguments are
  unaffected; the old call site is still valid.

---

## [0.1.0] — 2026-04-21

### Added

- **`VMCore`** — generic register VM interpreter (LANG02).  Executes `IIRModule`
  objects produced by any language front-end that targets `interpreter-ir`.
- **`RegisterFile`** — flat list of value slots with `snapshot()`/`restore()` for
  REPL error-recovery rollback.
- **`VMFrame`** — per-call-frame state: `fn`, `ip`, `registers`, `name_to_reg`,
  `return_dest`.  `VMFrame.for_function()` pre-assigns parameter registers.
- **`VMProfiler`** — inline type profiler.  Runs with constant overhead alongside
  the dispatch loop; maps Python runtime values to IIR type strings and calls
  `IIRInstr.record_observation()`.
- **`VMMetrics`** — immutable snapshot of lifetime execution statistics.  Fields:
  `function_call_counts`, `total_instructions_executed`, `total_frames_pushed`,
  `total_jit_hits`.
- **`BuiltinRegistry`** — maps builtin names to host-provided Python callables.
  Pre-registers `noop` and `assert_eq`.  Used by the `call_builtin` opcode.
- **`run_dispatch_loop()`** — tight `while frames:` dispatch loop with O(1) opcode
  lookup, profiler observation, and interrupt-flag check.
- **Standard opcode table** — 35 handlers covering all `interpreter_ir.ALL_OPS`
  mnemonics plus `const`:
  - Arithmetic: `add`, `sub`, `mul`, `div`, `mod`, `neg`
  - Bitwise: `and`, `or`, `xor`, `not`, `shl`, `shr`
  - Comparison: `cmp_eq`, `cmp_ne`, `cmp_lt`, `cmp_le`, `cmp_gt`, `cmp_ge`
  - Control flow: `label`, `jmp`, `jmp_if_true`, `jmp_if_false`, `ret`, `ret_void`
  - Memory: `load_reg`, `store_reg`, `load_mem`, `store_mem`
  - Calls: `call` (interpreter + JIT paths), `call_builtin`
  - I/O: `io_in`, `io_out`
  - Coercions: `cast`, `type_assert`
- **JIT handler integration** — `VMCore.register_jit_handler(fn_name, handler)`
  short-circuits the interpreter.  Registered handlers are checked before any
  frame is pushed; `total_jit_hits` is incremented on each hit.
- **`u8_wrap` mode** — all arithmetic results masked with `& 0xFF` when
  `VMCore(u8_wrap=True)`.  Required for Tetrad back-compat.
- **`VMCore.interrupt()`** — sets a flag; the dispatch loop raises `VMInterrupt`
  at the next cycle boundary.  Safe to call from other threads.
- **`VMCore.reset()`** — clears frame stack, memory, and I/O ports between
  independent program executions (lifetime metrics survive).
- **Exception hierarchy**: `VMError`, `UnknownOpcodeError`, `FrameOverflowError`,
  `UndefinedVariableError`, `VMInterrupt`.
- **Test suite**: 124 tests, 97.58% line coverage.  All modules except `dispatch.py`
  (96%) reach 100%.
