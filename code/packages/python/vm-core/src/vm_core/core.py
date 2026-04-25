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

from vm_core.builtins import BuiltinRegistry
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
