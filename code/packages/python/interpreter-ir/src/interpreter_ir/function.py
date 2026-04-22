"""IIRFunction — a named, parameterised sequence of IIRInstr.

A function is the unit of compilation in the LANG pipeline.  The JIT compiles
one function at a time; the profiler tracks call counts per function.

Type status
-----------
``FunctionTypeStatus`` mirrors the status used by Tetrad (TET02b) and will be
the shared enum across all languages:

- ``FULLY_TYPED``     — every instruction has a concrete ``type_hint``
- ``PARTIALLY_TYPED`` — some instructions are typed, some are ``"any"``
- ``UNTYPED``         — all instructions are ``"any"``

The JIT uses this to decide *when* to compile:

- ``FULLY_TYPED``     → compile before the first interpreted call
- ``PARTIALLY_TYPED`` → compile after 10 interpreter calls
- ``UNTYPED``         → compile after 100 interpreter calls

Example::

    fn = IIRFunction(
        name="add",
        params=[("a", "u8"), ("b", "u8")],
        return_type="u8",
        instructions=[
            IIRInstr("add", "v0", ["a", "b"], "u8"),
            IIRInstr("ret",  None, ["v0"],    "u8"),
        ],
        type_status=FunctionTypeStatus.FULLY_TYPED,
    )
"""

from __future__ import annotations

from dataclasses import dataclass, field
from enum import Enum, auto

from interpreter_ir.instr import IIRInstr


class FunctionTypeStatus(Enum):
    """Compilation tier based on how much type information is available."""

    FULLY_TYPED = auto()
    """All parameters and instructions carry concrete type hints."""

    PARTIALLY_TYPED = auto()
    """Some instructions are typed; others are ``"any"``."""

    UNTYPED = auto()
    """No static types; all type information must come from profiling."""


@dataclass
class IIRFunction:
    """A single function in an IIRModule.

    Parameters
    ----------
    name:
        The function name, used as the key in the JIT cache and the vm-core
        call-count metrics table.
    params:
        Ordered list of ``(param_name, type_hint)`` pairs.  The VM loads
        arguments into the first ``len(params)`` registers before the first
        instruction executes.
    return_type:
        Declared return type (``"u8"``, ``"void"``, ``"any"``, …).
    instructions:
        The body of the function as a flat list of ``IIRInstr``.  Labels are
        also represented as instructions (op ``"label"``) so the index of each
        instruction is its unique address within the function.
    register_count:
        Number of VM registers this function uses (including parameters).
        ``vm-core`` allocates exactly this many register slots per frame.
    type_status:
        Compilation tier.  Derived automatically from ``params`` and
        ``instructions`` if not supplied explicitly.
    call_count:
        Incremented by ``vm-core`` on each interpreted call.  Read by
        ``jit-core`` to trigger tier promotion.
    """

    name: str
    params: list[tuple[str, str]]   # [(param_name, type_hint), …]
    return_type: str
    instructions: list[IIRInstr]
    register_count: int = 8
    type_status: FunctionTypeStatus = FunctionTypeStatus.UNTYPED
    call_count: int = field(default=0, repr=False, compare=False)

    # -----------------------------------------------------------------------
    # Helpers
    # -----------------------------------------------------------------------

    def param_names(self) -> list[str]:
        """Return just the parameter names in order."""
        return [name for name, _ in self.params]

    def param_types(self) -> list[str]:
        """Return just the parameter type hints in order."""
        return [t for _, t in self.params]

    def infer_type_status(self) -> FunctionTypeStatus:
        """Derive type status from params and instruction type hints.

        A function is FULLY_TYPED if every param type and every instruction
        ``type_hint`` is a concrete type (not ``"any"``).  It is UNTYPED if
        none of them are.  Otherwise PARTIALLY_TYPED.
        """
        from interpreter_ir.opcodes import CONCRETE_TYPES

        all_hints: list[str] = [t for _, t in self.params] + [
            i.type_hint for i in self.instructions
        ]
        typed = sum(1 for h in all_hints if h in CONCRETE_TYPES)
        total = len(all_hints)
        if total == 0 or typed == 0:
            return FunctionTypeStatus.UNTYPED
        if typed == total:
            return FunctionTypeStatus.FULLY_TYPED
        return FunctionTypeStatus.PARTIALLY_TYPED

    def label_index(self, label_name: str) -> int:
        """Return the instruction index of the named label, or raise KeyError."""
        for idx, instr in enumerate(self.instructions):
            if instr.op == "label" and instr.srcs and instr.srcs[0] == label_name:
                return idx
        raise KeyError(f"label {label_name!r} not found in function {self.name!r}")

    def __repr__(self) -> str:
        params_str = ", ".join(f"{n}: {t}" for n, t in self.params)
        return (
            f"IIRFunction({self.name!r}, "
            f"params=[{params_str}], "
            f"return={self.return_type!r}, "
            f"instrs={len(self.instructions)}, "
            f"status={self.type_status.name})"
        )
