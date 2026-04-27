"""``VMTrace`` and ``VMTracer`` — opt-in per-instruction execution records.

An execution trace is a flat list of ``VMTrace`` records, one per
dispatched IIR instruction, each describing:

- the frame depth and function name at dispatch time
- the instruction pointer (IIR index) before the instruction fired
- the ``IIRInstr`` object itself (a reference, not a copy)
- the full register-file snapshot before and after dispatch
- any feedback-slot changes produced by this instruction

Tracing is **opt-in**: normal ``VMCore.execute`` does not allocate any
``VMTrace`` objects.  Only ``VMCore.execute_traced`` installs a tracer
and returns the accumulated records.  The per-instruction overhead of
tracing is dominated by two register-file copies, so this path is
intended for debuggers, test harnesses, and reproducer generators —
not for benchmarks or production runs.

Why live beside the metrics but in its own module?
--------------------------------------------------

``VMTrace`` is runtime state, like ``VMMetrics``.  But traces are much
higher-volume (one per instruction versus one aggregate per execution)
and their lifecycle differs (accumulated during execute_traced,
returned, then discarded).  Keeping them in a separate module documents
the "opt-in, high-volume" contract cleanly and avoids bloating
``metrics.py`` with a type most callers never instantiate.

Language-agnosticism
--------------------

Nothing in ``VMTrace`` knows the runtime-value types.  ``registers_before``
and ``registers_after`` are ``list[Any]``, holding whatever the active
frame's ``RegisterFile`` contained at the snapshot points.  ``fn_name``
is just a string.  Lisp cons cells, Ruby tagged pointers, JS Values,
Python objects — everything flows through identically.  A debugger
frontend can re-interpret the captured values through its own type
mapper if it needs nicer display.
"""

from __future__ import annotations

import copy
from dataclasses import dataclass, field
from typing import TYPE_CHECKING, Any

if TYPE_CHECKING:
    from interpreter_ir import IIRInstr, SlotState


__all__ = ["VMTrace", "VMTracer"]


@dataclass
class VMTrace:
    """A snapshot of VM state before and after one instruction.

    Attributes
    ----------
    frame_depth:
        Zero-based depth of the executing frame.  The root (entry-point)
        frame is depth 0; a CALL pushes to depth 1; that callee's CALL
        pushes to depth 2; and so on.
    fn_name:
        The name of the function whose instruction dispatched.
    ip:
        The IIR instruction index *before* dispatch.  Note that this
        might differ from the index at the point the trace is recorded
        for branch / call instructions, which mutate ``frame.ip``.
    instr:
        A reference to the ``IIRInstr`` that dispatched.  Not copied —
        subsequent profiler observations on the same instr (from later
        executions) will be visible through this reference if the
        trace is consulted after the fact.  Use ``slot_delta`` for the
        *snapshot at trace time*.
    registers_before:
        Shallow copy of the frame's register file before the instruction
        executed.
    registers_after:
        Shallow copy of the frame's register file after the instruction
        executed.
    slot_delta:
        List of ``(instr_idx, SlotState)`` tuples for any IIR instructions
        whose ``observed_slot`` changed during this dispatch.  Almost
        always ``[]`` or a single-entry list (only the current instr's
        slot can change during its own dispatch), but the type is a list
        for forward-compatibility with multi-observation dispatch
        schemes (e.g. a future ``call`` that observes both return type
        and argument type).  ``SlotState`` entries are deep-copied so
        they preserve the trace-time state even if the slot continues
        to advance in later executions.
    """

    frame_depth: int
    fn_name: str
    ip: int
    instr: IIRInstr
    registers_before: list[Any]
    registers_after: list[Any]
    slot_delta: list[tuple[int, SlotState]] = field(default_factory=list)


class VMTracer:
    """Accumulates ``VMTrace`` records during a traced execution.

    A fresh tracer is created at the start of each ``execute_traced``
    call.  It grows one entry per dispatched instruction and is returned
    by ``execute_traced`` along with the final result.

    The class is deliberately minimal: a list, an append method, and a
    read-only view.  Debuggers that want richer capture (e.g., capturing
    heap snapshots, differential stack traces) subclass and override
    ``observe``.
    """

    def __init__(self) -> None:
        self._traces: list[VMTrace] = []

    def observe(
        self,
        *,
        frame_depth: int,
        fn_name: str,
        ip: int,
        instr: IIRInstr,
        registers_before: list[Any],
        registers_after: list[Any],
        slot_delta: list[tuple[int, SlotState]] | None = None,
    ) -> None:
        """Record one instruction-dispatch event.

        All values are captured by reference except ``slot_delta``,
        whose entries are deep-copied so the trace retains the
        at-dispatch snapshot of the slot.  Copying the register files
        is the caller's responsibility — dispatch does this before
        calling us.
        """
        copied_delta: list[tuple[int, SlotState]] = []
        if slot_delta:
            copied_delta = [(idx, copy.deepcopy(state)) for idx, state in slot_delta]

        self._traces.append(
            VMTrace(
                frame_depth=frame_depth,
                fn_name=fn_name,
                ip=ip,
                instr=instr,
                registers_before=registers_before,
                registers_after=registers_after,
                slot_delta=copied_delta,
            )
        )

    @property
    def traces(self) -> list[VMTrace]:
        """The accumulated trace records, in dispatch order.

        Returns the live list — callers MAY iterate but SHOULD NOT
        mutate.  For independent analysis, take a ``list(vm_tracer.traces)``.
        """
        return self._traces

    def __len__(self) -> int:
        return len(self._traces)
