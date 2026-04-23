"""CLRState — an immutable snapshot of the CLR simulator state.

=== Why an Immutable Snapshot? ===

The CLRSimulator object holds *mutable* state — its evaluation stack, local
variables, program counter, and halt flag change with every instruction.  But
for testing and debugging we need a *point-in-time snapshot*: a value that
captures the complete VM state at a specific moment and never changes afterward.

Without immutability you could write:

    state_before = sim.get_state()   # BUG: state_before is just a reference
    sim.run(more_code)               #      it reflects the new state now!
    assert state_before.stack == ()  # fails — stack may have changed

With a frozen dataclass, ``get_state()`` returns a *copy* with all mutable
lists converted to immutable tuples.  The snapshot is a true value, not a
reference.

=== CLR Locals Can Be None ===

Unlike the WASM simulator (where locals are always initialized to 0), CLR
local variables start as ``None`` (uninitialized).  Attempting to read an
uninitialized local raises a ``RuntimeError``, just as in real .NET where the
verifier rejects methods that read potentially-uninitialized locals.

This means the ``locals`` tuple can hold object or ``None`` values.

=== Usage ===

    from clr_simulator import CLRSimulator, CLRState

    sim = CLRSimulator()
    result = sim.execute(program_bytes)
    state: CLRState = result.final_state

    # All fields are directly accessible
    print(state.pc)             # program counter
    print(state.halted)         # True if RET was executed
    print(state.stack)          # evaluation stack as a tuple
    print(state.locals[0])      # first local variable slot (None if uninitialized)
"""

from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True)
class CLRState:
    """Immutable snapshot of CLR simulator state at a point in time.

    All mutable collections (lists) from the live simulator are converted to
    tuples so this dataclass can be frozen.  The snapshot is a true value —
    it will not change even if the simulator continues executing.

    Attributes
    ----------
    stack:
        The evaluation stack at the moment of the snapshot.  Values are pushed
        and popped by CLR instructions like ``ldc.i4.1``, ``add``, etc.
        May contain ``None`` (pushed by ``ldnull``) or integers.
        Stored as a tuple (bottom-to-top order).
    locals:
        Local variable slots.  ``None`` means the slot has not been initialized
        yet (the CLR verifier ensures you never read an uninitialized local in
        real .NET).  Stored as a tuple.
    pc:
        Program counter (byte offset into the bytecode).  Points to the *next*
        instruction to fetch after the last executed instruction.
    halted:
        ``True`` if execution stopped because of a ``ret`` instruction.

    Examples
    --------
    >>> state = CLRState(
    ...     stack=(1, 2),
    ...     locals=(None,) * 16,
    ...     pc=3,
    ...     halted=False,
    ... )
    >>> state.stack
    (1, 2)
    >>> state.halted
    False
    >>> state.locals[0] is None
    True
    """

    stack: tuple[object | None, ...]
    locals: tuple[object | None, ...]
    pc: int
    halted: bool
