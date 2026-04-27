"""WasmState — an immutable snapshot of the WASM simulator state.

=== Why an Immutable Snapshot? ===

The WasmSimulator object holds *mutable* state — its operand stack, local
variables, program counter, halted flag, and cycle counter change with every
instruction.  But for testing and debugging we need a *point-in-time snapshot*:
a value that captures the complete VM state at a specific moment and never
changes afterward.

Without immutability you could write:

    state_before = sim.get_state()   # BUG: state_before is just a reference
    sim.run(more_code)               #      it reflects the new state now!
    assert state_before.stack == ()  # fails — stack may have changed

With a frozen dataclass, ``get_state()`` returns a *copy* with all mutable
lists converted to immutable tuples.  The snapshot is a true value, not a
reference.

=== WASM Locals Are Always Initialized ===

Unlike CLR local variables (which start as ``None``), WASM local variables are
always initialized to 0 when a function starts.  So the ``locals`` tuple
contains only ``int`` values, never ``None``.

=== Usage ===

    from wasm_simulator import WasmSimulator, WasmState

    sim = WasmSimulator(num_locals=4)
    result = sim.execute(program_bytes)
    state: WasmState = result.final_state

    # All fields are directly accessible
    print(state.pc)      # program counter
    print(state.halted)  # True if end instruction was executed
    print(state.stack)   # operand stack as a tuple
    print(state.locals)  # local variables as a tuple (all ints)
    print(state.cycle)   # number of instructions executed
"""

from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True)
class WasmState:
    """Immutable snapshot of WASM simulator state at a point in time.

    All mutable collections (lists) from the live simulator are converted to
    tuples so this dataclass can be frozen.  The snapshot is a true value —
    it will not change even if the simulator continues executing.

    Attributes
    ----------
    stack:
        The operand stack at the moment of the snapshot.  Values are pushed
        and popped by WASM instructions like ``i32.const``, ``i32.add``, etc.
        Stored as a tuple (bottom-to-top order).
    locals:
        Local variable slots.  WASM initializes all locals to 0 at function
        entry, so no slot is ever ``None``.  Stored as a tuple.
    pc:
        Program counter (byte offset into the bytecode).  Points to the
        *next* instruction to fetch after the last executed instruction.
    halted:
        ``True`` if execution stopped because of an ``end`` instruction.
    cycle:
        The total number of instructions executed since the last ``load()``
        or program start.

    Examples
    --------
    >>> state = WasmState(
    ...     stack=(1, 2),
    ...     locals=(0, 0, 0, 0),
    ...     pc=10,
    ...     halted=False,
    ...     cycle=2,
    ... )
    >>> state.stack
    (1, 2)
    >>> state.halted
    False
    >>> state.locals[0]
    0
    """

    stack: tuple[int, ...]
    locals: tuple[int, ...]
    pc: int
    halted: bool
    cycle: int
