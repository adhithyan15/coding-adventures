"""IIRInstr — the single instruction of InterpreterIR.

Design
------
Each instruction is a small, immutable-by-convention record with:

- ``op``      — a mnemonic string identifying the operation (e.g. ``"add"``)
- ``dest``    — the SSA variable produced, or ``None`` for void ops
- ``srcs``    — source operands: variable names (str) or literals (int/float/bool)
- ``type_hint`` — the declared type (``"u8"``, ``"bool"``, ``"any"``, …)

At runtime, two extra fields are written by the ``vm-core`` profiler:

- ``observed_type``    — the actual type seen so far (or ``"polymorphic"``)
- ``observation_count`` — how many times the profiler has sampled this instruction

A ``deopt_anchor`` marks the interpreter instruction index the JIT must revert
to if a type guard fails.

Example::

    # Statically typed (Tetrad)
    IIRInstr("add", "v0", ["a", "b"], "u8")

    # Dynamically typed (before profiling)
    IIRInstr("add", "v0", ["a", "b"], "any")

    # After profiling
    instr.observed_type = "u8"
    instr.observation_count = 47
"""

from __future__ import annotations

from dataclasses import dataclass, field

# An operand is either a variable name reference or an immediate literal.
Operand = str | int | float | bool


@dataclass
class IIRInstr:
    """A single InterpreterIR instruction.

    Parameters
    ----------
    op:
        Instruction mnemonic.  Must be one of the strings in
        ``interpreter_ir.opcodes.ALL_OPS`` plus ``"const"``.
    dest:
        Name of the SSA variable produced, or ``None`` for void instructions
        (branches, stores, returns).
    srcs:
        Source operands.  Each element is either a ``str`` (variable name)
        or a literal ``int``, ``float``, or ``bool``.
    type_hint:
        Declared type of ``dest``.  Use ``"any"`` for dynamically typed
        languages where the type is unknown at compile time.
    """

    op: str
    dest: str | None
    srcs: list[Operand]
    type_hint: str

    # -----------------------------------------------------------------------
    # Runtime feedback — written by vm-core profiler
    # -----------------------------------------------------------------------

    observed_type: str | None = field(default=None, repr=False, compare=False)
    """Runtime type observed by the vm-core profiler.

    ``None``           — not yet observed
    ``"u8"`` etc.     — concrete single type observed on all calls so far
    ``"polymorphic"`` — multiple different types seen; do not specialise
    """

    observation_count: int = field(default=0, repr=False, compare=False)
    """Number of times this instruction has been profiled by vm-core."""

    deopt_anchor: int | None = field(default=None, repr=False, compare=False)
    """Instruction index to resume interpreter at if a JIT type guard fails.

    Set by jit-core when emitting a type guard for this instruction.
    ``None`` means no guard has been emitted yet.
    """

    # -----------------------------------------------------------------------
    # Convenience helpers
    # -----------------------------------------------------------------------

    def is_typed(self) -> bool:
        """Return True if this instruction has a concrete (non-dynamic) type hint."""
        from interpreter_ir.opcodes import CONCRETE_TYPES
        return self.type_hint in CONCRETE_TYPES

    def has_observation(self) -> bool:
        """Return True if the profiler has recorded at least one observation."""
        return self.observation_count > 0

    def is_polymorphic(self) -> bool:
        """Return True if multiple types have been observed (do not specialise)."""
        return self.observed_type == "polymorphic"

    def effective_type(self) -> str:
        """The best available type: concrete hint first, then observed, then 'any'."""
        if self.is_typed():
            return self.type_hint
        if self.observed_type and self.observed_type != "polymorphic":
            return self.observed_type
        return "any"

    def record_observation(self, runtime_type: str) -> None:
        """Update the observation slot with a new runtime type.

        Called by vm-core's profiler after each execution of this instruction.
        Marks the slot as ``"polymorphic"`` if a second distinct type is seen.
        """
        if self.observed_type is None:
            self.observed_type = runtime_type
        elif self.observed_type != runtime_type:
            self.observed_type = "polymorphic"
        self.observation_count += 1

    def __repr__(self) -> str:
        dest_str = f"{self.dest} = " if self.dest is not None else ""
        srcs_str = ", ".join(repr(s) for s in self.srcs)
        type_str = f" : {self.type_hint}"
        obs = ""
        if self.observation_count > 0:
            obs = f" [obs={self.observed_type!r}×{self.observation_count}]"
        return f"IIRInstr({dest_str}{self.op}({srcs_str}){type_str}{obs})"
