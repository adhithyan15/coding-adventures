"""Generic pluggable register-based VM (GenericRegisterVM).

Motivation
----------
The ``RegisterVM`` class in ``vm.py`` is a complete, well-tested interpreter
for a JavaScript-style language.  Its opcode dispatch is hardcoded in a
``match/case`` statement: adding a new language means forking the entire
interpreter.

``GenericRegisterVM`` provides the same execution chassis — accumulator,
register file, fetch-decode-execute loop, tracing — with one key difference:
**opcodes are registered at runtime via callbacks**.

Architecture
------------

                  ┌────────────────────────────────────────────────────┐
  language backend │  TetradVM / LispVM / PythonVM / …                  │
                  │                                                      │
                  │  grvm = GenericRegisterVM()                          │
                  │  grvm.register_handler(Op.ADD,  _h_add)             │
                  │  grvm.register_handler(Op.CALL, _h_call)            │
                  │  grvm.register_handler(Op.HALT, lambda *a: grvm.halt()) │
                  └─────────────────┬──────────────────────────────────┘
                                    │ execute(frame)
                  ┌─────────────────▼──────────────────────────────────┐
  GenericRegisterVM │  fetch → dispatch → handler(grvm, frame, instr)    │
                  │  trace_builder hook for language-specific traces     │
                  │  halt() / ret(value) signals via BaseException        │
                  └────────────────────────────────────────────────────┘

Handler protocol
----------------
Each handler is a callable with signature::

    def handler(grvm: GenericRegisterVM, frame: RegisterFrame, instr: Any) -> None:
        # Reads/writes frame.acc, frame.registers[n], frame.user_data.
        # Call grvm.halt()    to stop execution and return frame.acc.
        # Call grvm.ret(v)    to return from the current frame with value v.
        # Raise GenericVMError for runtime errors.

The ``instr`` argument is any object with an ``opcode: int`` attribute and an
``operands: list[int]`` attribute.  Both ``register_vm.types.RegisterInstruction``
and ``tetrad_compiler.bytecode.Instruction`` satisfy this protocol.

Trace hook
----------
Setting ``grvm.trace_builder = fn`` enables per-instruction tracing.
After each instruction (including HALT and RET), the VM calls::

    fn(frame, instr, ip_before, acc_before, regs_before)

The ``frame`` fields have already been updated to their *after* values.
The ``acc_before`` and ``regs_before`` are the snapshots taken before the
instruction fired.

Languages that need richer trace events (e.g., feedback-vector deltas)
store side-data in ``frame.user_data`` during the handler and read it back
in the trace builder.

Control flow signals
--------------------
``grvm.halt()`` and ``grvm.ret(value)`` raise internal ``BaseException``
subclasses that are caught exclusively by ``_run_frame``.  Using
``BaseException`` prevents accidental capture by ``except Exception:``
clauses inside opcode handlers.

Thread safety
-------------
``GenericRegisterVM`` is single-threaded.  It uses ``self.trace_builder``
and the recursive ``_run_frame`` call stack.  Do not share an instance
across threads.

Public API
----------
::

    from register_vm.generic_vm import GenericRegisterVM, RegisterFrame, GenericVMError

    grvm = GenericRegisterVM()

    # Register language opcodes.
    grvm.register_handler(0x00, lambda g, f, i: setattr(f, "acc", i.operands[0]))
    grvm.register_handler(0xFF, lambda g, f, i: g.halt())

    frame = RegisterFrame(instructions=[...], acc=0, registers=[0]*8)
    result = grvm.run(frame)              # clean execution
    result, trace = grvm.run_traced(frame)  # with GenericTrace per instruction
"""

from __future__ import annotations

from collections.abc import Callable
from dataclasses import dataclass, field
from typing import Any

__all__ = [
    "GenericRegisterVM",
    "GenericTrace",
    "GenericVMError",
    "RegisterFrame",
]

# ---------------------------------------------------------------------------
# Public data structures
# ---------------------------------------------------------------------------


@dataclass
class RegisterFrame:
    """Mutable execution state for one function activation.

    ``instructions`` — list of instruction objects (any type with ``.opcode``
                       and ``.operands``).  The generic VM does not interpret
                       them; only handlers do.
    ``ip``           — instruction pointer (index into ``instructions``).
                       The VM pre-increments ip before calling the handler.
    ``acc``          — accumulator value.  Type is language-defined.
    ``registers``    — explicit register file.  Defaults to 8 slots of None.
    ``depth``        — call-stack depth (0 = outermost frame).
    ``caller_frame`` — back-pointer to the frame that called this one.
    ``user_data``    — dict for language-specific per-frame state.  Use this
                       to store locals dicts, feedback vectors, or any data
                       the generic VM doesn't know about.
    """

    instructions: list[Any]
    ip: int = 0
    acc: Any = None
    registers: list[Any] = field(default_factory=lambda: [None] * 8)
    depth: int = 0
    caller_frame: RegisterFrame | None = None
    user_data: dict[str, Any] = field(default_factory=dict)


@dataclass
class GenericTrace:
    """One step of a per-instruction execution trace.

    ``frame_depth``      — call-stack depth (0 = outermost).
    ``ip``               — instruction pointer *before* the instruction fired.
    ``opcode``           — the integer opcode that was dispatched.
    ``operands``         — copy of ``instr.operands``.
    ``acc_before``       — accumulator value *before* the instruction.
    ``acc_after``        — accumulator value *after* the instruction.
    ``registers_before`` — shallow copy of the register file before.
    ``registers_after``  — shallow copy of the register file after.
    """

    frame_depth: int
    ip: int
    opcode: int
    operands: list[int]
    acc_before: Any
    acc_after: Any
    registers_before: list[Any]
    registers_after: list[Any]


class GenericVMError(Exception):
    """Raised by the generic VM for dispatch errors (unregistered opcode, etc.).

    Language-specific runtime errors should use the language's own error type
    and be raised inside opcode handlers.
    """


# ---------------------------------------------------------------------------
# Internal control-flow signals
# ---------------------------------------------------------------------------
# We use BaseException so that "except Exception" clauses inside handlers
# don't accidentally swallow them.

class _HaltSignal(BaseException):
    """Raised by grvm.halt() to stop the dispatch loop and return frame.acc."""


class _ReturnSignal(BaseException):
    """Raised by grvm.ret(value) to exit the current frame with a value."""

    def __init__(self, value: Any) -> None:
        self.value = value


# ---------------------------------------------------------------------------
# Handler type alias
# ---------------------------------------------------------------------------

HandlerFn = Callable[["GenericRegisterVM", RegisterFrame, Any], None]
"""Signature for opcode handler callbacks.

``grvm``  — the GenericRegisterVM instance (use to call halt/ret).
``frame`` — the currently-executing call frame.
``instr`` — the instruction object (has .opcode and .operands).
"""

TraceBuilder = Callable[[RegisterFrame, Any, int, Any, list[Any]], None]
"""Signature for the optional post-instruction trace hook.

Arguments: (frame, instr, ip_before, acc_before, regs_before)

Called *after* each instruction (and after halt/ret) with the frame's
post-execution state.  ``acc_before`` and ``regs_before`` are pre-execution
snapshots.  Language backends use this to record rich trace events.
"""

# ---------------------------------------------------------------------------
# GenericRegisterVM
# ---------------------------------------------------------------------------


class GenericRegisterVM:
    """Register-based VM with pluggable opcode dispatch.

    Languages register handlers once at startup, then call ``run(frame)`` or
    ``run_traced(frame)`` to execute.  All execution state lives in the
    ``RegisterFrame``; the VM itself is stateless between calls (only
    ``trace_builder`` persists across calls).

    Example — a minimal two-opcode interpreter::

        grvm = GenericRegisterVM()
        grvm.register_handler(0x00, lambda g, f, i: setattr(f, "acc", i.operands[0]))
        grvm.register_handler(0xFF, lambda g, f, i: g.halt())

        @dataclass
        class Instr:
            opcode: int
            operands: list[int]

        frame = RegisterFrame(
            instructions=[Instr(0x00, [42]), Instr(0xFF, [])],
            acc=0,
            registers=[0] * 8,
        )
        result = grvm.run(frame)
        assert result == 42
    """

    def __init__(self) -> None:
        """Create a VM with no registered handlers.

        All opcodes must be registered before calling ``run()``.  Attempting
        to execute an unregistered opcode raises ``GenericVMError``.
        """
        self._handlers: dict[int, HandlerFn] = {}
        # Optional language-supplied post-instruction trace hook.
        self.trace_builder: TraceBuilder | None = None

    # ------------------------------------------------------------------
    # Handler registration
    # ------------------------------------------------------------------

    def register_handler(self, opcode: int, fn: HandlerFn) -> None:
        """Register an opcode handler.

        ``opcode`` — integer opcode value (e.g. ``Op.ADD`` or ``0x20``).
        ``fn``     — callable; see ``HandlerFn`` for the signature.

        Registering the same opcode twice overwrites the previous handler.
        """
        self._handlers[opcode] = fn

    # ------------------------------------------------------------------
    # Execution entry points
    # ------------------------------------------------------------------

    def run(self, frame: RegisterFrame) -> Any:
        """Execute ``frame`` to completion; return the final accumulator value.

        Uses the fast path: no per-instruction overhead beyond the handler
        dispatch and optional ``trace_builder`` call.

        Raises:
            GenericVMError: If an unregistered opcode is encountered.
            Any exception raised by a handler propagates upward.
        """
        return self._run_frame(frame)

    def run_traced(self, frame: RegisterFrame) -> tuple[Any, list[GenericTrace]]:
        """Execute with per-instruction ``GenericTrace`` recording.

        Returns ``(result, traces)`` where ``traces`` is a list of
        ``GenericTrace`` objects in execution order, including instructions
        from nested frames (produced by handlers that call ``_run_frame``
        recursively).

        Note: If ``trace_builder`` is also set, both it and the generic
        trace recording run for each instruction.  Usually you want only
        one or the other.

        Raises:
            GenericVMError: If an unregistered opcode is encountered.
        """
        traces: list[GenericTrace] = []
        orig_builder = self.trace_builder

        def _generic_builder(
            frame_inner: RegisterFrame,
            instr: Any,
            ip_before: int,
            acc_before: Any,
            regs_before: list[Any],
        ) -> None:
            traces.append(GenericTrace(
                frame_depth=frame_inner.depth,
                ip=ip_before,
                opcode=instr.opcode,
                operands=list(getattr(instr, "operands", [])),
                acc_before=acc_before,
                acc_after=frame_inner.acc,
                registers_before=regs_before,
                registers_after=list(frame_inner.registers),
            ))
            if orig_builder is not None:
                orig_builder(frame_inner, instr, ip_before, acc_before, regs_before)

        self.trace_builder = _generic_builder
        try:
            result = self._run_frame(frame)
        finally:
            self.trace_builder = orig_builder

        return result, traces

    # ------------------------------------------------------------------
    # Control-flow signals
    # ------------------------------------------------------------------

    def halt(self) -> None:
        """Signal the dispatch loop to stop; returns ``frame.acc``."""
        raise _HaltSignal()

    def ret(self, value: Any = None) -> None:
        """Signal the dispatch loop to exit the current frame with ``value``.

        The caller frame (if any) receives this value as the result of the
        CALL that spawned this frame.  If there is no caller, the top-level
        ``run()`` call returns ``value``.
        """
        raise _ReturnSignal(value)

    # ------------------------------------------------------------------
    # Core dispatch loop (semi-public so handlers can recurse)
    # ------------------------------------------------------------------

    def _run_frame(self, frame: RegisterFrame) -> Any:
        """Fetch-decode-execute loop for one call frame.

        Called recursively by CALL handlers to execute nested frames.
        The recursion depth is bounded by the language's call-stack limit
        (not the generic VM's concern).

        Returns the value passed to ``ret(v)`` or ``frame.acc`` on halt.
        """
        while True:
            instr = frame.instructions[frame.ip]
            ip_before = frame.ip
            frame.ip += 1

            # Snapshot state *before* the handler fires.
            # Only allocate if the trace_builder is active (avoid cost on hot path).
            if self.trace_builder is not None:
                acc_before: Any = frame.acc
                regs_before: list[Any] = list(frame.registers)
            else:
                acc_before = None
                regs_before = []

            opcode: int = instr.opcode
            handler = self._handlers.get(opcode)
            if handler is None:
                raise GenericVMError(
                    f"no handler registered for opcode 0x{opcode:02X}"
                    f" at depth={frame.depth} ip={ip_before}"
                )

            # Dispatch.  We catch _HaltSignal and _ReturnSignal here so
            # they don't escape past the trace hook.
            halt_or_ret: _HaltSignal | _ReturnSignal | None = None
            try:
                handler(self, frame, instr)
            except _HaltSignal as sig:
                halt_or_ret = sig
            except _ReturnSignal as sig:
                # Update acc to the return value so the trace captures it.
                frame.acc = sig.value
                halt_or_ret = sig

            # Post-instruction hook (tracing, debugger, profiler, …).
            if self.trace_builder is not None:
                self.trace_builder(frame, instr, ip_before, acc_before, regs_before)

            if halt_or_ret is not None:
                if isinstance(halt_or_ret, _ReturnSignal):
                    return halt_or_ret.value
                return frame.acc
