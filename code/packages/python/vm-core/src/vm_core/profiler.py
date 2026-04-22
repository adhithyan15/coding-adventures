"""VMProfiler — runtime type observation for InterpreterIR instructions.

The profiler watches each instruction execute and fills in the
``observed_type`` / ``observation_count`` fields on the IIRInstr object.
These fields are later read by jit-core to decide which type to specialise
on and whether a function is worth compiling.

The profiler only records observations for instructions whose ``type_hint``
is ``"any"`` — statically typed instructions already know their type and
don't need profiling.

Type mapping
------------
The profiler maps Python runtime types to IIR type strings:

    Python type / value             IIR type
    ──────────────────────────────  ────────
    bool                            "bool"
    int in 0..255                   "u8"
    int in 256..65535               "u16"
    int in 65536..2^32-1            "u32"
    int >= 2^32 or negative         "u64"
    float                           "f64"
    str                             "str"
    anything else                   "any"

Note: bool is checked before int because ``isinstance(True, int)`` is True
in Python (bool is a subclass of int).
"""

from __future__ import annotations

from typing import Any

from interpreter_ir import IIRInstr


def _python_type_to_iir(value: Any) -> str:
    """Map a Python runtime value to an IIR type string."""
    if isinstance(value, bool):
        return "bool"
    if isinstance(value, int):
        if 0 <= value <= 255:
            return "u8"
        if 0 <= value <= 65535:
            return "u16"
        if 0 <= value <= 0xFFFF_FFFF:
            return "u32"
        return "u64"
    if isinstance(value, float):
        return "f64"
    if isinstance(value, str):
        return "str"
    return "any"


class VMProfiler:
    """Observes instruction results and annotates IIRInstr feedback slots.

    The profiler is always-on: it runs inline in the dispatch loop with
    constant overhead (one isinstance check + one dict lookup per profiled
    instruction).

    Only instructions with ``type_hint == "any"`` are profiled.  Concrete
    type instructions are skipped.
    """

    def __init__(self) -> None:
        self._total_observations: int = 0

    def observe(self, instr: IIRInstr, result: Any) -> None:
        """Record the runtime type of ``result`` on ``instr``.

        Called by the dispatch loop after each instruction that produces a
        value (i.e. ``instr.dest is not None``).
        """
        if instr.type_hint != "any":
            return
        rt = _python_type_to_iir(result)
        instr.record_observation(rt)
        self._total_observations += 1

    @property
    def total_observations(self) -> int:
        """Total number of profiling events recorded this session."""
        return self._total_observations
