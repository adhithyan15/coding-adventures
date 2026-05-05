# Changelog — vm-core

All notable changes to this package will be documented in this file.

---

## [Unreleased]

### Added — VMCOND00 Phase 3: Dynamic handler chain (Layer 3)

Implements VMCOND00 Layer 3 — non-unwinding condition handlers — in the vm-core
interpreter.  Five new IIR opcodes (`push_handler`, `pop_handler`, `signal`,
`error`, `warn`) are now fully dispatched by the VM.

**New module: `vm_core.handler_chain`**

- **`HandlerNode`** — dataclass representing one entry on the dynamic handler
  chain:
  - `condition_type: str` — `"*"` (catch-all) or a Python type name.
  - `handler_fn: object` — a string (IIR function name) in Phase 3; Phase 4
    will extend this to closures and native callables.
  - `stack_depth: int` — frame-stack depth at push time (reserved for Phase 5
    `EXIT_TO` unwinding).

**New error: `HandlerChainError`** (subclass of `VMError`) — raised by
`pop_handler` when the handler chain is empty (push/pop imbalance detected at
runtime).

**New `VMCore` state: `_handler_chain: list[HandlerNode]`** — mutable stack
cleared on `VMCore.reset()`.

**New dispatch handlers in `vm_core.dispatch`**

- `handle_push_handler` — reads `type_id` from `srcs[0]` and the handler
  function name from `frame.resolve(srcs[1])`, wraps them in a `HandlerNode`,
  and appends to `vm._handler_chain`.
- `handle_pop_handler` — pops the most recently pushed node; raises
  `HandlerChainError` on underflow.
- `handle_signal` — walks the chain most-recent → oldest via
  `_handler_type_matches`; on the first match calls the handler
  non-unwinding via `_invoke_handler_nonunwinding`; if no match, continues
  silently (no-op).
- `handle_error` — two-phase dispatch:
  1. Layer 2 static exception table checked first (same logic as `handle_throw`
     but without full unwind — just redirects `frame.ip` if in range).
  2. Layer 3 handler chain walked exactly as `handle_signal`.
  3. If neither matches, raises `UncaughtConditionError`.
- `handle_warn` — walks the chain exactly as `handle_signal`; if no match,
  emits `[vm-core WARN] <repr>` to `sys.stderr` using the same hardened
  repr strategy as `UncaughtConditionError`, then continues.

**Non-unwinding invocation protocol** — `_invoke_handler_nonunwinding` pushes
a fresh `VMFrame` for the handler function with `return_dest=None` (discarding
the return value), copies the condition into register 0 (the handler's first
parameter), and appends the frame to `vm._frames`.  The dispatch loop runs
inside the handler; when the handler executes `ret`, `handle_ret` pops it and
the original frame resumes at the instruction *after* the signaling opcode.

**Security** — `handle_warn`'s stderr output uses the same nested
`try/except` defence as `UncaughtConditionError.__init__` to guard against
guest objects whose `__repr__` raises or returns an unbounded string.

**Tests** — 33 new tests in `tests/test_vmcond00_phase3.py` covering:
`HandlerNode` construction, `HandlerChainError` hierarchy,
push/pop/underflow mechanics, `signal` no-op and match paths,
`error` Layer 2 priority and Layer 3 fallback, `warn` stderr emission,
cross-frame handler visibility, and LIFO handler ordering.

---

### Added — VMCOND00 Phase 2: throw dispatch + UncaughtConditionError

Implements VMCOND00 Layer 2 — unwind exceptions — in the vm-core interpreter.
A `throw` instruction now walks the per-function static exception tables of
every active frame from innermost to outermost, transferring control to the
first matching handler or aborting execution with `UncaughtConditionError`.

**New exception class: `UncaughtConditionError(VMError)`** (`errors.py`)

Raised when a `throw` propagates to the top of the call stack with no matching
handler anywhere.  Wraps the original condition object for inspection by the
host environment:

```python
from vm_core.errors import UncaughtConditionError
try:
    vm.execute(module)
except UncaughtConditionError as e:
    print(f"VM aborted: {e.condition!r}")
```

`UncaughtConditionError` is re-exported from the package root (`vm_core`).

**New dispatch handler: `handle_throw`** (`dispatch.py`)

Implements the full Layer 2 unwind algorithm:

1. Read the condition value from the source register.
2. Walk `vm._frames` from innermost (top) to outermost (bottom).
3. For each frame, compute `throw_ip = frame.ip - 1` (the dispatch loop
   increments `ip` **before** calling the handler, so the instruction that
   raised is always at `ip - 1`).
4. Scan the frame's `fn.exception_table` for an `ExceptionTableEntry` where
   `entry.from_ip <= throw_ip < entry.to_ip` (half-open range, matching JVM /
   CPython convention).
5. If the entry also matches `entry.type_id` (see below), redirect the frame:
   set `frame.ip = entry.handler_ip`, store the condition into `entry.val_reg`,
   and return normally.
6. If no entry matches in this frame, pop the frame and continue to the caller.
7. If all frames are exhausted, raise `UncaughtConditionError(condition)`.

The handler is registered in `STANDARD_OPCODES` under `"throw"`.

**New helper: `_throw_type_matches(condition, type_id) -> bool`** (`dispatch.py`)

Encapsulates Phase 2 type-matching logic:

- `type_id == "*"` (i.e. `CATCH_ALL`) — always matches; write a catch-all
  handler by setting `type_id="*"` in the `ExceptionTableEntry`.
- Any other string — exact match against `type(condition).__name__`.
  Example: `type_id="ValueError"` catches only `ValueError` instances, not
  subclasses.  Subtype hierarchy is deferred to Phase 3 (`is_subtype` lookup).

**Test additions (`tests/test_vmcond00_phase2.py` — 34 new tests):**

- `TestExceptionTableEntry` (6): construction, frozen immutability, equality,
  CATCH_ALL constant, typed entry, `compare=False` contract.
- `TestThrowSameFrame` (7): catch-all catches int/str/None, typed match, typed
  mismatch raises, val_reg assigned correctly, instructions after throw are
  unreachable.
- `TestThrowRangeBoundaries` (3): `from_ip` is inclusive (catches at boundary),
  `to_ip` is exclusive (does not catch), instruction before range not caught.
- `TestThrowAcrossFrames` (5): callee throw caught in caller, frames popped on
  propagation, three-level propagation, no handler raises
  `UncaughtConditionError`, innermost handler wins over outer.
- `TestUncaughtConditionError` (4): condition attribute preserved, string repr,
  `VMError` subclass, root-frame uncaught raises.
- `TestThrowTypeMatching` (9): catch-all matches int/str/list/None, exact type
  name matches int/str, type name mismatch, custom class name match/mismatch.

Coverage: 97.84% (267 tests pass; was 233 + 26 phase-1 tests).

**Spec reference:** VMCOND00 §3 Layer 2 — unwind exceptions.

---

### Added — VMCOND00 Phase 1: syscall_checked and branch_err dispatch + register_syscall API

Implements VMCOND00 Layer 1 — the result-value error protocol — in the vm-core
interpreter.  Languages that opt in can invoke numbered host syscalls without
trapping and route control flow based on the error code, all without touching
the condition system or allocating any handler objects.

**New opcode handlers (in `dispatch.py`):**

- **`handle_syscall_checked`** — Executes a SYSCALL00 canonical syscall by
  number.  Looks up the implementation in `VMCore._syscall_table[n]`, calls it
  with the resolved argument, and writes `(value, error_code)` into two named
  registers.  Error convention: 0 = success, -1 = EOF, <-1 = negated errno.
  The handler never raises Python exceptions: unknown syscall numbers return
  `(0, -EINVAL)`; implementations that raise are caught and also return
  `(0, -EINVAL)`.

- **`handle_branch_err`** — Reads the error-code register and jumps to the
  target label when it is non-zero.  Falls through when it is zero (success).
  Does not record branch statistics (the check is a typed error test, not an
  algorithmic conditional).

Both handlers are registered in `STANDARD_OPCODES` under the string mnemonics
`"syscall_checked"` and `"branch_err"`.

**New public API on `VMCore`:**

- **`register_syscall(n, impl)`** — Register a host implementation for
  SYSCALL00 syscall number `n`.  `impl` must have signature
  `(arg: int) -> (value: int, error_code: int)`.  `n` must be in `[1, 255]`
  (the SYSCALL00 canonical range); a `ValueError` is raised for out-of-range
  numbers.  See the docstring for the error-code convention and a complete
  write-byte example.
- **`unregister_syscall(n)`** — Remove a previously registered implementation.
  No-op if `n` is not registered.
- **`_syscall_table`** — Internal dict `{int: Callable}`.  Empty by default;
  languages wire it up by calling `register_syscall`.  The VM is agnostic about
  I/O strategy.

**Security hardening:**

- `register_syscall` validates that `n` is in `[1, 255]` before writing to
  the table.  Syscall 0 is reserved by the ABI; numbers above 255 are outside
  the canonical table.  Eager rejection surfaces programming errors at
  registration time rather than silently diverging at dispatch time.

**Test additions (`tests/test_vmcond00_phase1.py` — 26 new tests):**

- `TestRegisterSyscall` — 11 tests: populate, overwrite, remove, no-op, empty
  default, reject 0, reject >255, reject negative, accept boundary 1, accept
  boundary 255.
- `TestSyscallChecked` — 7 tests: success value, return value, EOF, errno, unknown
  number → EINVAL, impl raises → EINVAL, arg forwarding.
- `TestBranchErr` — 4 tests: branch on -1, branch on negated errno, branch on
  positive non-zero, fall-through on 0.
- `TestSyscallCheckedWithBranchErr` — 4 integration tests: full round-trip through
  mock read-byte (success and EOF), unknown syscall → error path, two-syscall program.

Coverage: 97.79% (233 tests pass).

---

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
