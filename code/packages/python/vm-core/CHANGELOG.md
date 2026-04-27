# Changelog — vm-core

All notable changes to this package will be documented in this file.

---

## [Unreleased]

### Added — LANG18: Lightweight VM coverage mode

- **`VMCore._coverage_mode` / `VMCore._coverage`** — two new internal fields that
  form the LANG18 coverage subsystem.  `_coverage_mode` is the master gate (False
  by default, checked once per dispatch iteration); `_coverage` accumulates executed
  IIR instruction indices per function as `dict[str, set[int]]`.  Zero allocation
  and zero dict writes when coverage is disabled.

- **`VMCore.enable_coverage()`** — enter coverage mode.  Idempotent; existing data
  is preserved so multiple partial runs accumulate cleanly.

- **`VMCore.disable_coverage()`** — exit coverage mode.  Collected data is preserved;
  call `reset_coverage()` to clear it.  Idempotent (no-op if already off).

- **`VMCore.is_coverage_mode() -> bool`** — True when coverage collection is active.
  Coverage mode and debug mode (`is_debug_mode()`) are independent flags; both can
  be True simultaneously without interference.

- **`VMCore.coverage_data() -> dict[str, frozenset[int]]`** — point-in-time snapshot
  of executed IIR instruction indices per function.  Returns `frozenset` values so
  callers cannot accidentally mutate the live coverage sets.  Subsequent execution
  does not retroactively change the returned snapshot.

- **`VMCore.reset_coverage()`** — clear all coverage data and disable coverage mode.
  After this call `coverage_data()` returns `{}` and `is_coverage_mode()` is False.

- **Dispatch loop change** (`vm_core/dispatch.py`) — inserted an
  `if vm._coverage_mode:` block immediately after the LANG06 debug-mode check.
  When coverage is on, the current `ip_before` is added to
  `vm._coverage[frame.fn.name]`.  This is the only hot-path change; coverage and
  debug mode are checked in separate `if`-blocks so neither feature pays the cost
  of the other.

- **`tests/test_coverage.py`** — 23 new tests organised into six classes:
  `TestCoverageDefaultState`, `TestCoverageCollection`, `TestBranchCoverage`,
  `TestCoverageAccumulation`, `TestDisableCoverage`, `TestCoverageAndDebugMode`.
  All pass with the full test suite at 97.69% total coverage.

### Added — LANG06: Debug hooks, breakpoints, and step-mode API

- `vm_core.debug` — new module containing:
  - `StepMode` enum with three values: `IN` (pause at very next IIR instruction),
    `OVER` (pause at next instruction in same or outer frame), `OUT` (pause after
    the current frame returns).
  - `DebugHooks` — base class with four no-op callback methods: `on_instruction`,
    `on_call`, `on_return`, `on_exception`.  Debug adapters subclass this.
- `VMCore.attach_debug_hooks(hooks)` — registers a `DebugHooks` adapter and
  enables debug mode (`_debug_mode = True`).  Zero overhead when no hooks are
  attached: the dispatch loop gates the entire debug path behind a single boolean.
- `VMCore.detach_debug_hooks()` — removes the adapter and disables debug mode.
- `VMCore.is_debug_mode() -> bool` — True when hooks are attached.
- `VMCore.pause()` — requests that the dispatch loop fire `on_instruction` at
  the very next instruction.
- `VMCore.step_in()` — set `StepMode.IN`: pause at every next instruction in any
  frame.  Call from inside `on_instruction` to single-step.
- `VMCore.step_over()` — set `StepMode.OVER`: pause at next instruction in the
  current or a shallower frame (skips callee internals).
- `VMCore.step_out()` — set `StepMode.OUT`: let execution continue until the
  current frame returns, then pause at the next instruction in the caller.
- `VMCore.continue_()` — clear step mode; execution resumes without any
  additional pauses until the next breakpoint.
- `VMCore.set_breakpoint(instr_idx, fn_name, condition=None)` — register an
  unconditional or conditional breakpoint.  A `condition` string is a Python
  expression evaluated with the frame's named register values as locals; the
  breakpoint only fires when the expression is truthy.  Invalid expressions are
  silently ignored.
- `VMCore.clear_breakpoint(instr_idx, fn_name)` — remove a registered
  breakpoint.
- `VMCore.call_stack() -> list[VMFrame]` — return a shallow copy of the current
  frame stack (bottom to top).  Safe to inspect outside `on_instruction`.
- `VMCore.patch_function(fn_name, new_fn)` — hot-swap a function body mid-run;
  raises `KeyError` if the function is not in the module.
- `run_dispatch_loop` — fires `on_instruction` before each dispatch when debug
  mode is on; fires `on_call` in `handle_call` before pushing the callee frame;
  fires `on_return` in `handle_ret` / `handle_ret_void` after popping the frame;
  fires `on_exception` (best-effort, never masked) when an unhandled error
  propagates.  `StepMode.OUT` is handled in `_fire_on_return` by converting a
  return at the watched depth into a `_paused = True` for the next instruction.
- Re-exports `DebugHooks` and `StepMode` from the package root.
- `tests/test_debug_hooks.py` — 184 tests (28 new debug-hook tests added to the
  existing 156): attach/detach, on_instruction at breakpoints, conditional
  breakpoints, step_in / step_over / step_out, call_stack, patch_function,
  on_call / on_return / on_exception, adapter-error robustness.

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
