"""JVMState — an immutable snapshot of the JVM simulator state.

=== Why an Immutable Snapshot? ===

The JVMSimulator object holds *mutable* state — its stack, local variables,
program counter, and halt flag change with every instruction.  But for
testing and debugging we need a *point-in-time snapshot*: a value that
captures the complete VM state at a specific moment and never changes
afterward.

Without immutability you could write:

    state_before = sim.get_state()   # BUG: state_before is just a reference
    sim.run(more_code)               #      it reflects the new state now!
    assert state_before.stack == ()  # fails — stack may have changed

With a frozen dataclass, ``get_state()`` returns a *copy* with all mutable
lists converted to immutable tuples.  The snapshot is a true value, not a
reference.

=== Usage ===

    from jvm_simulator import JVMSimulator, JVMState

    sim = JVMSimulator()
    result = sim.execute(program_bytes)
    state: JVMState = result.final_state

    # All fields are directly accessible
    print(state.pc)             # program counter
    print(state.halted)         # True if RETURN/IRETURN was executed
    print(state.stack)          # operand stack as a tuple
    print(state.locals[0])      # first local variable slot
    print(state.return_value)   # value returned by IRETURN (None for RETURN)
"""

from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True)
class JVMState:
    """Immutable snapshot of JVM simulator state at a point in time.

    All mutable collections (lists) from the live simulator are converted to
    tuples so this dataclass can be frozen.  The snapshot is a true value —
    it will not change even if the simulator continues executing.

    Attributes
    ----------
    stack:
        The operand stack at the moment of the snapshot.  Values are pushed
        and popped by JVM instructions like ``iconst_1``, ``iadd``, etc.
        Stored as a tuple (bottom-to-top order, matching the list indexing).
    locals:
        Local variable slots.  The JVM assigns numbered slots (0, 1, 2, ...)
        for method parameters and local variables.  ``None`` means the slot
        has not been initialized yet.
    constants:
        The constant pool.  Values loaded by the ``ldc`` instruction.  May
        contain integers or strings.  Stored as a tuple.
    pc:
        Program counter (byte offset into the bytecode).  Points to the
        *next* instruction to fetch after the last executed instruction.
    halted:
        ``True`` if execution stopped because of a ``RETURN`` or ``IRETURN``
        instruction.
    return_value:
        The integer value returned by an ``IRETURN`` instruction, or ``None``
        if the method returned void (``RETURN``) or has not yet returned.

    Examples
    --------
    >>> state = JVMState(
    ...     stack=(1, 2),
    ...     locals=(None,) * 16,
    ...     constants=(),
    ...     pc=4,
    ...     halted=False,
    ...     return_value=None,
    ... )
    >>> state.stack
    (1, 2)
    >>> state.halted
    False
    >>> state.locals[0] is None
    True
    """

    stack: tuple[object, ...]
    locals: tuple[object | None, ...]
    constants: tuple[object, ...]
    pc: int
    halted: bool
    return_value: object | None
