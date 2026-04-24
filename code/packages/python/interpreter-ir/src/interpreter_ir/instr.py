"""IIRInstr — the single instruction of InterpreterIR.

Design
------
Each instruction is a small, immutable-by-convention record with:

- ``op``      — a mnemonic string identifying the operation (e.g. ``"add"``)
- ``dest``    — the SSA variable produced, or ``None`` for void ops
- ``srcs``    — source operands: variable names (str) or literals (int/float/bool)
- ``type_hint`` — the declared type (``"u8"``, ``"bool"``, ``"any"``, …)

At runtime, extra fields are written by the ``vm-core`` profiler:

- ``observed_slot``    — a ``SlotState`` holding the V8 Ignition-style
                         feedback state machine (UNINIT → MONO → POLY → MEGA)
- ``observed_type``    — derived view: the single observed type for
                         MONOMORPHIC slots, ``"polymorphic"`` for POLY/MEGA,
                         or ``None`` before any observation
- ``observation_count`` — derived view: ``observed_slot.count`` (kept as a
                         separate field for backwards compatibility)

A ``deopt_anchor`` marks the interpreter instruction index the JIT must revert
to if a type guard fails.

Example::

    # Statically typed (Tetrad)
    IIRInstr("add", "v0", ["a", "b"], "u8")

    # Dynamically typed (before profiling)
    IIRInstr("add", "v0", ["a", "b"], "any")

    # After profiling
    instr.observed_slot.kind         # SlotKind.MONOMORPHIC
    instr.observed_type              # "u8"
    instr.observation_count          # 47
"""

from __future__ import annotations

from dataclasses import dataclass, field

from interpreter_ir.slot_state import SlotKind, SlotState

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
    """Runtime type observed by the vm-core profiler (legacy view).

    ``None``           — not yet observed
    ``"u8"`` etc.      — concrete single type observed on all calls so far
    ``"polymorphic"``  — multiple different types seen; do not specialise

    This field is kept as a real slot (not a property) for backwards
    compatibility with tests and external code that assign directly to
    it.  ``record_observation`` keeps it in sync with ``observed_slot``.
    Prefer ``observed_slot`` for new code — it distinguishes polymorphic
    from megamorphic sites, which ``observed_type`` cannot.
    """

    observation_count: int = field(default=0, repr=False, compare=False)
    """Number of times this instruction has been profiled by vm-core.

    Mirrors ``observed_slot.count`` once ``record_observation`` has been
    called at least once.  Kept as a separate field for backwards
    compatibility.
    """

    observed_slot: SlotState | None = field(default=None, repr=False, compare=False)
    """V8 Ignition-style feedback slot (LANG17).

    ``None`` until the profiler samples this instruction for the first
    time; thereafter holds a :class:`SlotState` the profiler updates in
    place.  See :mod:`interpreter_ir.slot_state` for the state machine.
    """

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

        Called by vm-core's profiler after each execution of this
        instruction.  Advances the V8 Ignition-style state machine on
        ``observed_slot`` and keeps the legacy ``observed_type`` /
        ``observation_count`` fields in sync.

        The state machine distinguishes *polymorphic* (2–4 types, still
        JIT-friendly via a small dispatch table) from *megamorphic* (≥5
        types, not worth specialising).  Callers that only need the
        older two-state view can continue reading ``observed_type``.
        """
        if self.observed_slot is None:
            self.observed_slot = SlotState()
        self.observed_slot.record(runtime_type)

        # Keep the legacy mirror fields in sync so existing callers that
        # read ``observed_type`` / ``observation_count`` keep working
        # without modification.
        self.observation_count = self.observed_slot.count
        if self.observed_slot.kind is SlotKind.MONOMORPHIC:
            self.observed_type = self.observed_slot.observations[0]
        elif self.observed_slot.kind in (
            SlotKind.POLYMORPHIC,
            SlotKind.MEGAMORPHIC,
        ):
            self.observed_type = "polymorphic"
        # UNINITIALIZED is unreachable here — ``record`` always leaves
        # the slot in one of the three active kinds above.

    def __repr__(self) -> str:
        dest_str = f"{self.dest} = " if self.dest is not None else ""
        srcs_str = ", ".join(repr(s) for s in self.srcs)
        type_str = f" : {self.type_hint}"
        obs = ""
        if self.observation_count > 0:
            obs = f" [obs={self.observed_type!r}×{self.observation_count}]"
        return f"IIRInstr({dest_str}{self.op}({srcs_str}){type_str}{obs})"
