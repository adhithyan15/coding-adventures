"""VMCore — the public API for the vm-core register interpreter.

VMCore is a generic, language-agnostic register VM that executes
InterpreterIR (IIR) modules.  A language front-end produces an
``IIRModule`` containing ``IIRFunction`` objects; calling ``execute()``
runs the nominated function and returns the result.

Architecture overview
---------------------
                 ┌─────────────┐
   IIRModule ──▶ │  VMCore     │ ──▶  return value
                 │             │
                 │ ┌─────────┐ │
                 │ │ frame   │ │  ← VMFrame stack (one per call)
                 │ │ stack   │ │
                 │ └─────────┘ │
                 │ ┌─────────┐ │
                 │ │ opcode  │ │  ← handler lookup dict (O(1))
                 │ │ table   │ │
                 │ └─────────┘ │
                 │ ┌─────────┐ │
                 │ │profiler │ │  ← type observations → jit-core
                 │ └─────────┘ │
                 └─────────────┘

Key configuration knobs
-----------------------
u8_wrap:
    When True, all arithmetic results are masked with ``& 0xFF``.
    Required for Tetrad compatibility (8-bit register semantics).

profiler_enabled:
    Toggle the inline type profiler.  Disable for benchmarks where
    the profiling overhead is not desired.

max_frames:
    Hard limit on call-stack depth.  Prevents stack overflow from
    runaway recursion — raises ``FrameOverflowError`` instead.

opcodes:
    Language-specific opcode handlers.  Entries here shadow the
    standard table, allowing languages to override or extend any
    mnemonic without subclassing VMCore.

builtins:
    Pre-configured ``BuiltinRegistry``.  If None, a fresh registry
    with only ``noop`` and ``assert_eq`` is created.

JIT integration
---------------
Call ``register_jit_handler(fn_name, handler)`` to short-circuit
the interpreter for a compiled function.  The handler receives a
list of resolved argument values and returns the result.  The
interpreter path is bypassed entirely — no frame is pushed.

Thread safety
-------------
VMCore is NOT thread-safe.  Each executing thread must own its
own VMCore instance.
"""

from __future__ import annotations

from collections.abc import Callable
from typing import Any

from interpreter_ir import IIRModule
from interpreter_ir.function import IIRFunction

from vm_core.builtins import BuiltinRegistry
from vm_core.debug import DebugHooks, StepMode
from vm_core.dispatch import STANDARD_OPCODES, run_dispatch_loop
from vm_core.frame import VMFrame
from vm_core.metrics import BranchStats, VMMetrics
from vm_core.profiler import TypeMapper, VMProfiler
from vm_core.tracer import VMTrace, VMTracer


class VMCore:
    """Generic register VM interpreter.

    Parameters
    ----------
    max_frames:
        Maximum call-stack depth before ``FrameOverflowError`` is raised.
    opcodes:
        Optional dict of extra / override opcode handlers.  Entries shadow
        the standard table; the standard table is always the fallback.
    builtins:
        Optional pre-configured ``BuiltinRegistry``.  Created fresh if None.
    profiler_enabled:
        Whether to run the inline type profiler.
    u8_wrap:
        Mask arithmetic results to 8 bits (Tetrad / u8 mode).
    type_mapper:
        Callable from runtime value to IIR type string, used by the
        profiler to advance the V8 Ignition-style feedback-slot state
        machine.  If omitted, :func:`vm_core.profiler.default_type_mapper`
        is used, which handles Python primitives.  Supply a custom
        mapper when hosting a language whose runtime values are not
        Python primitives (Lisp cons cells, Ruby tagged pointers, JS
        Values, etc.) — the profiler otherwise classifies them all as
        ``"any"`` and the JIT never specialises.
    """

    def __init__(
        self,
        *,
        max_frames: int = 64,
        opcodes: dict[str, Any] | None = None,
        builtins: BuiltinRegistry | None = None,
        profiler_enabled: bool = True,
        u8_wrap: bool = False,
        type_mapper: TypeMapper | None = None,
    ) -> None:
        self._max_frames = max_frames
        self._u8_wrap = u8_wrap
        self._profiler_enabled = profiler_enabled

        # Merge standard table with any language-supplied overrides.
        self._opcode_table: dict[str, Any] = dict(STANDARD_OPCODES)
        if opcodes:
            self._opcode_table.update(opcodes)

        self._builtins: BuiltinRegistry = (
            builtins if builtins is not None else BuiltinRegistry()
        )
        self._profiler: VMProfiler = VMProfiler(type_mapper=type_mapper)

        # JIT handlers — registered by jit-core after compilation.
        self._jit_handlers: dict[str, Callable[[list[Any]], Any]] = {}

        # VMCOND00 Phase 3 — handler chain (Layer 3: Dynamic Handlers).
        #
        # A list of HandlerNode objects pushed by ``push_handler`` and popped
        # by ``pop_handler``.  SIGNAL / ERROR / WARN search this list from the
        # END (most recently pushed) to the BEGINNING (oldest), calling the
        # first node whose condition_type matches the thrown condition.
        #
        # The list is intentionally cleared by reset() between executions so
        # that handlers from a previous run never leak into the next one.
        self._handler_chain: list = []  # list[HandlerNode]

        # VMCOND00 Phase 1 — syscall dispatch table.
        #
        # Maps SYSCALL00 canonical syscall number (int) to an implementation
        # callable ``(arg: int) -> (value: int, error_code: int)``.  The
        # error_code convention follows SYSCALL00 Section 3: 0 on success,
        # -1 on EOF, <-1 for negated errno.
        #
        # Language frontends and host environments register implementations
        # via ``register_syscall``.  The dispatch handler falls back to
        # EINVAL for unknown numbers — matching the C ABI contract.
        #
        # Default table is empty; languages must explicitly wire up the
        # syscalls they use.  This keeps the VM agnostic about I/O strategy.
        self._syscall_table: dict[int, Callable[[int], tuple[int, int]]] = {}

        # Execution state — reset between execute() calls.
        self._frames: list[VMFrame] = []
        self._module: IIRModule | None = None
        self._interrupted: bool = False

        # Addressable memory and I/O ports.
        self._memory: dict[int, Any] = {}
        self._io_ports: dict[int, Any] = {}

        # Metrics accumulators — never reset automatically; aggregate
        # lifetime stats.  Call ``reset_metrics()`` to zero them.
        self._metrics_instrs: int = 0
        self._metrics_frames: int = 0
        self._metrics_jit_hits: int = 0
        self._fn_call_counts: dict[str, int] = {}

        # LANG17 branch / loop observation state — the dispatch-loop
        # handlers for ``jmp`` / ``jmp_if_true`` / ``jmp_if_false``
        # mutate these dicts directly via the helpers in
        # ``vm_core.dispatch``.  The public API exposes them via
        # :meth:`branch_profile`, :meth:`loop_iterations`, and through
        # deep copies inside :meth:`metrics`.
        self._branch_stats: dict[str, dict[int, BranchStats]] = {}
        self._loop_back_edges: dict[str, dict[int, int]] = {}

        # Active tracer (LANG17 PR3).  ``None`` on the normal execute
        # path so no overhead is paid.  Set transiently by
        # :meth:`execute_traced` for the duration of one run.
        self._tracer: VMTracer | None = None

        # ------------------------------------------------------------------
        # LANG06 debug state — all fields are None / False / empty when debug
        # mode is inactive so the dispatch loop pays zero overhead.
        # ``_debug_mode`` is the master gate: it is True iff a DebugHooks
        # instance is currently attached.  The dispatch loop checks only this
        # flag before calling ``_check_debug_pause``.
        # ------------------------------------------------------------------

        # The attached debug adapter.  None means debug mode is off.
        self._debug_hooks: DebugHooks | None = None

        # Convenient alias so the dispatch loop avoids an is-not-None test.
        self._debug_mode: bool = False

        # True when the dispatch loop should pause at the next instruction.
        # Set by ``pause()`` and cleared by the dispatch loop after
        # ``on_instruction`` returns.
        self._paused: bool = False

        # Step granularity requested by the most recent step call.
        self._step_mode: StepMode | None = None

        # Frame-stack depth at which a step_over / step_out was requested.
        # The dispatch loop uses this to decide whether to pause.
        self._step_frame_depth: int = 0

        # Breakpoints: {fn_name: {instr_idx: condition_expr_or_None}}
        # condition_expr is a source-level expression string evaluated over
        # the frame's register file.  None means unconditional.
        self._breakpoints: dict[str, dict[int, str | None]] = {}

        # ------------------------------------------------------------------
        # LANG18 coverage state — zero cost when coverage mode is off.
        #
        # ``_coverage_mode`` is the master gate.  When True, the dispatch
        # loop records every IIR instruction index that is reached, keyed
        # by function name, in ``_coverage``.  The gate costs exactly one
        # boolean comparison per instruction (same pattern as _debug_mode).
        #
        # Coverage and debug mode are fully independent — both can be
        # active simultaneously without interference.
        # ------------------------------------------------------------------

        # True → dispatch loop updates ``_coverage`` each instruction.
        self._coverage_mode: bool = False

        # fn_name → set of IIR instruction indices that have been executed.
        # Populated incrementally; empty until ``enable_coverage()`` is called.
        self._coverage: dict[str, set[int]] = {}

    # ------------------------------------------------------------------
    # Public API
    # ------------------------------------------------------------------

    def execute(
        self,
        module: IIRModule,
        *,
        fn: str = "main",
        args: list[Any] | None = None,
    ) -> Any:
        """Execute function ``fn`` in ``module`` and return its result.

        Parameters
        ----------
        module:
            The ``IIRModule`` containing the function to execute.
        fn:
            Name of the entry-point function.  Defaults to ``"main"``.
        args:
            Positional arguments passed to the entry-point function.
            Must match the function's parameter list in order.

        Raises
        ------
        KeyError
            If ``fn`` is not defined in ``module``.
        FrameOverflowError
            If the call stack exceeds ``max_frames``.
        UnknownOpcodeError
            If the function uses an opcode with no registered handler.
        VMInterrupt
            If ``interrupt()`` is called during execution.
        """
        entry = module.get_function(fn)
        if entry is None:
            raise KeyError(f"function {fn!r} not found in module")

        self._module = module
        self._frames = []
        self._interrupted = False

        root_frame = VMFrame.for_function(entry, return_dest=None)

        # Copy positional arguments into the root frame's registers.
        if args:
            for i, value in enumerate(args[: len(entry.params)]):
                root_frame.registers[i] = value

        self._frames.append(root_frame)
        self._metrics_frames += 1
        self._fn_call_counts[fn] = self._fn_call_counts.get(fn, 0) + 1

        return run_dispatch_loop(self)

    def execute_traced(
        self,
        module: IIRModule,
        *,
        fn: str = "main",
        args: list[Any] | None = None,
    ) -> tuple[Any, list[VMTrace]]:
        """Execute ``fn`` and return ``(result, list_of_VMTraces)``.

        For every instruction dispatched during this run, the tracer
        records a :class:`vm_core.tracer.VMTrace` capturing:

        - frame depth at dispatch
        - function name and instruction-pointer before dispatch
        - a reference to the ``IIRInstr`` that ran
        - shallow-copied register files before and after the dispatch
        - any feedback-slot changes produced by this instruction

        Overhead: two ``list`` copies and one ``VMTrace`` allocation
        per instruction.  Intended for debuggers, test harnesses, and
        reproducer generation — not for hot-path runs.

        A fresh :class:`VMTracer` is installed for this call only; the
        normal :meth:`execute` path still pays zero tracing cost.
        """
        tracer = VMTracer()
        previous_tracer = self._tracer
        self._tracer = tracer
        try:
            result = self.execute(module, fn=fn, args=args)
        finally:
            # Restore any outer tracer (for nested execute_traced calls,
            # though that is a corner case).
            self._tracer = previous_tracer
        return result, list(tracer.traces)

    def metrics(self) -> VMMetrics:
        """Return a point-in-time snapshot of execution statistics.

        The snapshot is a deep copy — subsequent execution will not
        change it, and the caller can freely mutate the returned dicts
        without affecting the running VM.
        """
        branch_snapshot: dict[str, dict[int, BranchStats]] = {
            fn: {
                ip: BranchStats(
                    taken_count=stats.taken_count,
                    not_taken_count=stats.not_taken_count,
                )
                for ip, stats in per_fn.items()
            }
            for fn, per_fn in self._branch_stats.items()
        }
        loop_snapshot: dict[str, dict[int, int]] = {
            fn: dict(per_fn) for fn, per_fn in self._loop_back_edges.items()
        }
        return VMMetrics(
            function_call_counts=dict(self._fn_call_counts),
            total_instructions_executed=self._metrics_instrs,
            total_frames_pushed=self._metrics_frames,
            total_jit_hits=self._metrics_jit_hits,
            branch_stats=branch_snapshot,
            loop_back_edge_counts=loop_snapshot,
        )

    # ------------------------------------------------------------------
    # LANG17 typed accessors — sugar over ``metrics()`` so callers can
    # look up one fn/site without walking a full snapshot.
    # ------------------------------------------------------------------

    def hot_functions(self, threshold: int = 100) -> list[str]:
        """Return names of functions whose call count meets ``threshold``.

        JITs use this to decide what to promote to the compiled tier.
        Returns function names in insertion order (Python dict order
        preserves call-count registration order).
        """
        return [
            name
            for name, count in self._fn_call_counts.items()
            if count >= threshold
        ]

    def branch_profile(self, fn_name: str, source_ip: int) -> BranchStats | None:
        """Return the live ``BranchStats`` for one conditional branch.

        ``source_ip`` is the IIR instruction index of the branch
        (``jmp_if_true`` / ``jmp_if_false``) within ``fn_name``'s
        instruction list.  Returns ``None`` if the branch has never
        been reached.

        The returned object is the *live* counter — subsequent VM
        execution will mutate it.  Use ``metrics().branch_stats`` for a
        stable snapshot.
        """
        fn_stats = self._branch_stats.get(fn_name)
        if fn_stats is None:
            return None
        return fn_stats.get(source_ip)

    def loop_iterations(self, fn_name: str) -> dict[int, int]:
        """Return back-edge hit counts for ``fn_name`` keyed by source IP.

        Empty dict if the function has no back-edges or has never
        executed one.  The returned dict is a fresh copy so callers can
        mutate it freely without affecting the VM.
        """
        fn_loops = self._loop_back_edges.get(fn_name, {})
        return dict(fn_loops)

    def reset_metrics(self) -> None:
        """Zero all aggregate metrics.

        Clears:

        - ``function_call_counts`` / ``total_instructions_executed`` /
          ``total_frames_pushed`` / ``total_jit_hits``
        - ``branch_stats`` and ``loop_back_edge_counts``

        Does *not* reset per-instruction observations
        (``IIRInstr.observed_slot`` / ``observed_type``) — those live on
        the IIR module, not on the VM.  Callers that want a clean slate
        for observations should walk their module or construct a fresh
        one.  (LANG17 future work: add a helper for this.)
        """
        self._fn_call_counts = {}
        self._metrics_instrs = 0
        self._metrics_frames = 0
        self._metrics_jit_hits = 0
        self._branch_stats = {}
        self._loop_back_edges = {}

    def register_builtin(self, name: str, fn: Callable[[list[Any]], Any]) -> None:
        """Register a host callable under ``name`` in the builtin registry.

        The callable receives a single argument: a list of resolved values.
        It should return a value (or None for void builtins).

        Example::

            vm.register_builtin("print", lambda args: print(*args))
        """
        self._builtins.register(name, fn)

    def register_syscall(
        self,
        n: int,
        impl: Callable[[int], tuple[int, int]],
    ) -> None:
        """Register a host implementation for SYSCALL00 syscall number ``n``.

        The implementation receives the single argument register value and
        returns a ``(value, error_code)`` pair following the SYSCALL00 error
        convention (Section 3 of SYSCALL00 spec):

        - ``(value, 0)``    — success; ``value`` is the result (bytes written,
                               byte read, fd, …)
        - ``(0, -1)``       — EOF (read operations only)
        - ``(0, -errno)``   — error; errno is the POSIX error number (positive)

        The implementation must NOT raise Python exceptions — wrap I/O in
        try/except and return an error pair instead.  Any exception that
        escapes will be caught by the dispatch handler and converted to
        ``(0, EINVAL)`` with no further information.

        Example — registering write-byte (syscall 1) backed by stdout::

            import sys

            def write_byte_impl(arg: int) -> tuple[int, int]:
                try:
                    sys.stdout.buffer.write(bytes([arg & 0xFF]))
                    sys.stdout.buffer.flush()
                    return (0, 0)
                except OSError as e:
                    return (0, -e.errno)

            vm.register_syscall(1, write_byte_impl)

        Parameters
        ----------
        n:
            SYSCALL00 canonical syscall number (1 = write-byte, 2 = read-byte,
            10 = exit, …).  Must be in the range [1, 255].
        impl:
            Callable ``(arg: int) -> (value: int, error_code: int)``.

        Raises
        ------
        ValueError
            If ``n`` is outside the valid SYSCALL00 range [1, 255].  Syscall 0
            is reserved by the ABI and numbers above 255 are beyond the
            canonical table, so both extremes are rejected to catch programming
            errors early rather than at dispatch time.
        """
        if not (1 <= n <= 255):  # noqa: PLR2004 — magic numbers are the spec range
            raise ValueError(
                f"syscall number {n!r} is outside the valid range [1, 255]; "
                "see SYSCALL00 §2 for the canonical table"
            )
        self._syscall_table[n] = impl

    def unregister_syscall(self, n: int) -> None:
        """Remove the registered implementation for syscall number ``n``.

        No-op if the syscall is not registered.

        Parameters
        ----------
        n:
            SYSCALL00 canonical syscall number to remove.
        """
        self._syscall_table.pop(n, None)

    def register_jit_handler(
        self, fn_name: str, handler: Callable[[list[Any]], Any]
    ) -> None:
        """Register a JIT handler for ``fn_name``.

        When the interpreter encounters a ``call fn_name`` instruction,
        it calls ``handler(args)`` instead of pushing a new frame.
        The handler bypasses the interpreter completely — no frame is
        pushed, no instructions are executed, and the profiler is not
        updated.

        This is the primary integration point for jit-core.
        """
        self._jit_handlers[fn_name] = handler

    def unregister_jit_handler(self, fn_name: str) -> None:
        """Remove the JIT handler for ``fn_name``, reverting to interpreted calls."""
        self._jit_handlers.pop(fn_name, None)

    def interrupt(self) -> None:
        """Signal the dispatch loop to raise ``VMInterrupt`` at the next cycle.

        Safe to call from a different thread.  The interrupt is delivered
        asynchronously — the current instruction completes before the
        exception is raised.
        """
        self._interrupted = True

    def reset(self) -> None:
        """Reset per-execution state.

        Clears the frame stack, memory, I/O ports, and the interrupted
        flag.  Lifetime metrics (instruction counts, JIT hits) are NOT
        reset — they accumulate across all executions.

        Call this between independent programs when reusing a VMCore
        instance (e.g., in a REPL).
        """
        self._frames = []
        self._module = None
        self._interrupted = False
        self._memory = {}
        self._io_ports = {}
        self._handler_chain = []  # VMCOND00 Phase 3 — clear per execution

    # ------------------------------------------------------------------
    # LANG06 debug API
    # ------------------------------------------------------------------

    def attach_debug_hooks(self, hooks: DebugHooks) -> None:
        """Attach a debug adapter and enter debug mode.

        While debug hooks are attached:
        - ``is_debug_mode()`` returns True.
        - The dispatch loop fires ``hooks.on_instruction`` whenever the VM
          pauses (breakpoint hit or step mode).
        - ``hooks.on_call`` fires before every CALL pushes a new frame.
        - ``hooks.on_return`` fires after every RET pops a frame.
        - JIT handlers should not be registered — check ``is_debug_mode()``
          before calling ``register_jit_handler`` in the JIT tier.

        Calling this with a new hooks object replaces the previous adapter.

        Parameters
        ----------
        hooks:
            A ``DebugHooks`` instance (or subclass).
        """
        self._debug_hooks = hooks
        self._debug_mode = True

    def detach_debug_hooks(self) -> None:
        """Remove the debug adapter and exit debug mode.

        After this call, ``is_debug_mode()`` returns False and the dispatch
        loop resumes zero-overhead execution.
        """
        self._debug_hooks = None
        self._debug_mode = False
        self._paused = False
        self._step_mode = None

    def is_debug_mode(self) -> bool:
        """Return True when a debug adapter is attached.

        JIT tiers should consult this before registering JIT handlers::

            if not vm.is_debug_mode():
                jit.compile_and_register(fn_name)

        This ensures that all functions remain interpreted while debugging
        so that ``on_instruction`` fires for every instruction.
        """
        return self._debug_mode

    def pause(self) -> None:
        """Request the dispatch loop to pause before the next instruction.

        Safe to call from inside ``on_instruction`` (to stay paused) or from
        a signal handler / other thread.  The current instruction completes
        before the pause takes effect.

        The pause is delivered by firing ``on_instruction`` before the next
        IIR instruction is dispatched.
        """
        self._paused = True
        self._step_mode = None

    def step_in(self) -> None:
        """Resume and pause before the very next IIR instruction dispatched.

        Steps into called functions — the finest stepping granularity.
        Equivalent to the DAP ``stepIn`` request.
        """
        self._paused = False
        self._step_mode = StepMode.IN

    def step_over(self) -> None:
        """Resume and pause at the next instruction in the current or an outer frame.

        Skips the internals of any function called between now and the next
        instruction at the current call-stack depth.
        Equivalent to the DAP ``next`` (step-over) request.
        """
        self._paused = False
        self._step_mode = StepMode.OVER
        self._step_frame_depth = len(self._frames)

    def step_out(self) -> None:
        """Resume and pause at the instruction after the current frame returns.

        Runs the rest of the current function and pauses at the return site
        in the caller.  Equivalent to the DAP ``stepOut`` request.
        """
        self._paused = False
        self._step_mode = StepMode.OUT
        self._step_frame_depth = len(self._frames)

    def continue_(self) -> None:
        """Resume execution until the next breakpoint or end of program.

        Clears any pending pause or step mode.  Equivalent to the DAP
        ``continue`` request.
        """
        self._paused = False
        self._step_mode = None

    def set_breakpoint(
        self,
        instr_idx: int,
        fn_name: str,
        condition: str | None = None,
    ) -> None:
        """Register a breakpoint at ``instr_idx`` in ``fn_name``.

        The dispatch loop checks registered breakpoints before every
        instruction (when debug mode is on).  When the instruction pointer
        matches, ``on_instruction`` is called.

        To convert a source line number to an instruction index, use::

            from debug_sidecar import DebugSidecarReader
            reader = DebugSidecarReader(sidecar_bytes)
            idx = reader.find_instr("myprogram.tetrad", line=42)
            vm.set_breakpoint(idx, "main")

        Parameters
        ----------
        instr_idx:
            0-based IIR instruction index within ``fn_name``'s body.
        fn_name:
            Function name (must match the ``IIRFunction.name``).
        condition:
            Optional condition expression (source-level string).  The
            breakpoint only fires when the expression evaluates to a truthy
            value over the current frame's register file.  Pass ``None`` for
            an unconditional breakpoint.
        """
        if fn_name not in self._breakpoints:
            self._breakpoints[fn_name] = {}
        self._breakpoints[fn_name][instr_idx] = condition

    def clear_breakpoint(self, instr_idx: int, fn_name: str) -> None:
        """Remove the breakpoint at ``instr_idx`` in ``fn_name``.

        No-op if the breakpoint does not exist.
        """
        fn_bps = self._breakpoints.get(fn_name)
        if fn_bps is not None:
            fn_bps.pop(instr_idx, None)
            if not fn_bps:
                del self._breakpoints[fn_name]

    def call_stack(self) -> list[VMFrame]:
        """Return a copy of the current call stack, outermost frame first.

        The returned list is a shallow copy of the live frame stack — it is
        safe to inspect but mutating it has no effect on the running VM.

        Each frame exposes:
        - ``frame.fn.name``      — function name
        - ``frame.ip``           — next instruction index (already advanced)
        - ``frame.registers``    — the register file (live values)
        - ``frame.name_to_reg``  — variable name → register index mapping

        Use ``frame.ip - 1`` to get the index of the instruction that was
        last dispatched (the one that caused the pause).
        """
        return list(self._frames)

    def patch_function(self, fn_name: str, new_fn: IIRFunction) -> None:
        """Hot-swap ``fn_name`` with a new ``IIRFunction`` in the running module.

        Replaces the function definition in the current module so that
        subsequent CALL instructions targeting ``fn_name`` execute the new
        body.  Frames currently on the call stack that are executing the
        *old* function body are not affected — they continue running to
        completion.

        This supports live editing while paused at a breakpoint.  If the new
        function has a different register count or parameter list, the
        behaviour is undefined for any frame that was already mid-execution
        on the old body.

        Raises
        ------
        KeyError:
            If no module is currently loaded or if ``fn_name`` does not
            exist in the current module.
        RuntimeError:
            If called outside of a paused debug session (i.e. not from
            inside an ``on_instruction`` callback or after ``pause()`` has
            been called).
        """
        if self._module is None:
            raise KeyError("no module is currently loaded")
        existing = self._module.get_function(fn_name)
        if existing is None:
            raise KeyError(f"function {fn_name!r} not found in current module")
        # IIRModule stores functions in a list; find and replace by name.
        for i, fn in enumerate(self._module.functions):
            if fn.name == fn_name:
                self._module.functions[i] = new_fn
                return
        raise KeyError(f"function {fn_name!r} not found in module function list")

    # ------------------------------------------------------------------
    # LANG18 coverage API
    # ------------------------------------------------------------------

    def enable_coverage(self) -> None:
        """Enter coverage mode.

        While coverage mode is active the dispatch loop records every IIR
        instruction index that is reached, grouped by function name.  The
        overhead is a single boolean guard per instruction — identical to
        the LANG06 debug-mode gate — and is zero when disabled.

        Calling this more than once is safe (idempotent).  Existing
        coverage data is preserved across calls so you can enable, run a
        slice of a program, disable, re-enable, and accumulate hits across
        multiple partial runs.  Call ``reset_coverage()`` to start fresh.
        """
        self._coverage_mode = True

    def disable_coverage(self) -> None:
        """Exit coverage mode.

        The dispatch loop stops recording instruction hits.  The data
        already collected in ``_coverage`` is preserved — call
        ``coverage_data()`` to read it.

        Calling this when coverage mode is already off is safe (no-op).
        """
        self._coverage_mode = False

    def is_coverage_mode(self) -> bool:
        """Return True when coverage collection is active.

        Coverage mode and debug mode (``is_debug_mode``) are independent
        flags; both can be True simultaneously.
        """
        return self._coverage_mode

    def coverage_data(self) -> dict[str, frozenset[int]]:
        """Return a snapshot of the IIR instruction indices that have been executed.

        The snapshot maps function name → frozenset of IIR instruction
        indices (0-based within that function's instruction list).  The
        values are ``frozenset`` so callers cannot accidentally mutate the
        live coverage sets.

        The snapshot is taken at the moment of the call — subsequent
        execution will not retroactively change the returned value, but
        the *live* internal sets may gain new entries.

        Use ``debug_sidecar.DebugSidecarReader.lookup(fn_name, ip)`` to
        project each covered IIR index back to a source line::

            reader = DebugSidecarReader(sidecar_bytes)
            for fn_name, ips in vm.coverage_data().items():
                for ip in ips:
                    loc = reader.lookup(fn_name, ip)
                    if loc:
                        print(f"{loc.file}:{loc.line}")
        """
        return {fn: frozenset(ips) for fn, ips in self._coverage.items()}

    def reset_coverage(self) -> None:
        """Clear all coverage data and disable coverage mode.

        After this call ``coverage_data()`` returns an empty dict and
        ``is_coverage_mode()`` returns False.  Call ``enable_coverage()``
        again to start a fresh coverage run.
        """
        self._coverage_mode = False
        self._coverage = {}

    # ------------------------------------------------------------------
    # Properties — read-only access to internal state for tooling
    # ------------------------------------------------------------------

    @property
    def builtins(self) -> BuiltinRegistry:
        """The builtin registry.  Register host callables here."""
        return self._builtins

    @property
    def profiler(self) -> VMProfiler:
        """The inline type profiler.  Read by jit-core to inspect observations."""
        return self._profiler

    @property
    def io_ports(self) -> dict[int, Any]:
        """The I/O port map.  Read/write by the host or device drivers."""
        return self._io_ports

    @property
    def memory(self) -> dict[int, Any]:
        """The addressable memory map.  Sparse; unwritten addresses default to 0."""
        return self._memory

    @property
    def u8_wrap(self) -> bool:
        """Whether arithmetic results are masked to 8 bits."""
        return self._u8_wrap

    @property
    def profiler_enabled(self) -> bool:
        """Whether the inline type profiler is active."""
        return self._profiler_enabled

    @profiler_enabled.setter
    def profiler_enabled(self, value: bool) -> None:
        self._profiler_enabled = value

    @property
    def is_executing(self) -> bool:
        """True while a dispatch loop is running (frames are on the stack)."""
        return bool(self._frames)
