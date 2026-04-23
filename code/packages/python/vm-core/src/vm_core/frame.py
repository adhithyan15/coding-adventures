"""VMFrame and RegisterFile — the per-call-frame state of vm-core.

Each function call in vm-core creates a fresh VMFrame.  The frame holds:

- ``fn``          — the IIRFunction being executed
- ``ip``          — instruction pointer (index into fn.instructions)
- ``registers``   — a flat register file (one slot per virtual variable)
- ``return_dest`` — register index in the *caller* frame where the return
                    value will be stored (None for the root frame)

The RegisterFile is a plain Python list, which gives O(1) reads and writes.
Named variables are resolved by a ``name_to_reg`` dict built once per frame
from the function's parameter list — parameters occupy the first N registers,
with indices assigned in parameter order.

Example::

    fn = IIRFunction(name="add", params=[("a", "u8"), ("b", "u8")], ...)
    # a → register 0, b → register 1
    frame = VMFrame.for_function(fn)
    frame.registers[0] = 10   # a = 10
    frame.registers[1] = 20   # b = 20
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any

from interpreter_ir import IIRFunction


class RegisterFile:
    """A flat array of value slots, one per virtual variable.

    The file is pre-sized to ``register_count`` slots, all initialised to 0.
    Named-variable access goes through VMFrame.resolve(); direct integer-index
    access is used by the argument-passing code.
    """

    __slots__ = ("_slots",)

    def __init__(self, count: int = 8) -> None:
        self._slots: list[Any] = [0] * count

    def __getitem__(self, idx: int) -> Any:
        return self._slots[idx]

    def __setitem__(self, idx: int, value: Any) -> None:
        self._slots[idx] = value

    def __len__(self) -> int:
        return len(self._slots)

    def reset(self) -> None:
        """Zero all slots (used by REPL snapshot rollback)."""
        for i in range(len(self._slots)):
            self._slots[i] = 0

    def snapshot(self) -> list[Any]:
        """Return a shallow copy of all slots."""
        return list(self._slots)

    def restore(self, saved: list[Any]) -> None:
        """Restore slots from a snapshot."""
        for i, v in enumerate(saved):
            self._slots[i] = v


@dataclass
class VMFrame:
    """State for a single function call.

    Parameters
    ----------
    fn:
        The function being executed.
    registers:
        The register file for this frame.
    name_to_reg:
        Maps variable names to register indices.  Populated from fn.params
        at frame creation; extended by the dispatch loop as new dest names
        appear.
    ip:
        Current instruction pointer.
    return_dest:
        Register index in the *caller* frame where the return value will be
        stored, or None for the root frame.
    """

    fn: IIRFunction
    registers: RegisterFile
    name_to_reg: dict[str, int] = field(default_factory=dict)
    ip: int = 0
    return_dest: int | None = None

    @classmethod
    def for_function(
        cls,
        fn: IIRFunction,
        return_dest: int | None = None,
    ) -> VMFrame:
        """Allocate a fresh VMFrame for ``fn``.

        Parameter names are pre-assigned to registers 0..len(params)-1.
        """
        regs = RegisterFile(max(fn.register_count, len(fn.params), 1))
        name_to_reg = {name: i for i, (name, _) in enumerate(fn.params)}
        return cls(
            fn=fn,
            registers=regs,
            name_to_reg=name_to_reg,
            return_dest=return_dest,
        )

    def resolve(self, operand: Any) -> Any:
        """Resolve an operand to its current value.

        - ``str`` → look up the variable in ``name_to_reg`` and read the register
        - anything else → return as a literal
        """
        if isinstance(operand, str):
            idx = self.name_to_reg.get(operand)
            if idx is None:
                from vm_core.errors import UndefinedVariableError
                raise UndefinedVariableError(
                    f"variable {operand!r} not defined in function {self.fn.name!r}"
                )
            return self.registers[idx]
        return operand

    def assign(self, dest: str, value: Any) -> None:
        """Assign ``value`` to the variable named ``dest``.

        Allocates a new register slot if this is the first assignment to ``dest``.
        """
        if dest not in self.name_to_reg:
            next_idx = len(self.name_to_reg)
            self.name_to_reg[dest] = next_idx
        self.registers[self.name_to_reg[dest]] = value
