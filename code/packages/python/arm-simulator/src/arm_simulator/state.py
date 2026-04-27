"""ARM CPU state snapshot — immutable CPU state for the simulator-protocol.

=== Why a Separate State Module? ===

The ``ARMSimulator`` class wraps a ``CPU`` object whose internal state is
*mutable* — registers, PC, and halted flag all change during execution.
The ``Simulator[StateT]`` protocol requires ``get_state()`` to return an
*immutable* snapshot so that callers can safely store, compare, and diff
states without worrying about them changing later.

This module provides ``ARMState``: a frozen dataclass that captures the
complete ARM CPU state at a single point in time.

=== ARM Architecture Quick Reference ===

ARM (Acorn RISC Machine, 1985) has 16 general-purpose 32-bit registers:

    R0–R3   : function arguments and return values
    R4–R11  : callee-saved general purpose
    R12     : IP — intra-procedure scratch
    R13     : SP — stack pointer
    R14     : LR — link register (return address after CALL/BL)
    R15     : PC — program counter (yes, it's a visible register!)

In our simplified simulator:
  - ``registers`` holds R0–R15 (16 × 32-bit values) as a tuple.
  - ``pc`` is ``registers[15]`` — a convenience alias.
  - Instructions are 4 bytes wide (word-aligned).
  - The halt sentinel is ``0xFFFFFFFF`` (a custom encoding).

=== Condition Flags ===

ARM stores four condition flags (N, Z, C, V) in the CPSR (Current Program
Status Register).  Our simplified simulator does not implement the full
CPSR but records the four flags from the most recent comparison:

    N  (Negative): set when the result's MSB is 1 (negative in two's comp).
    Z  (Zero):     set when the result is exactly 0.
    C  (Carry):    set when an addition/shift produces a carry-out.
    V  (Overflow): set when signed arithmetic overflows.

In our MVP simulator the instruction set does not update flags (S-bit = 0
on all data-processing instructions), so the flags tuple will be all-False
unless future instructions set the S-bit.
"""

from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True)
class ARMState:
    """Immutable snapshot of ARM CPU state at a single point in time.

    All fields are value types or immutable containers so the snapshot
    cannot change after construction, even if the simulator keeps running.

    Attributes
    ----------
    registers:
        Tuple of 16 × 32-bit register values (R0–R15).  Index 15 (R15) is
        the program counter — reading ``registers[15]`` gives the PC.
        Immutable tuple ensures the snapshot cannot be mutated in-place.
    pc:
        Program counter (32-bit).  A convenience alias for ``registers[15]``.
        Byte-addressed; each instruction advances the PC by 4.
    flags:
        Condition flags (N, Z, C, V) as a 4-tuple of booleans in that order:
          - flags[0] = N (Negative)
          - flags[1] = Z (Zero)
          - flags[2] = C (Carry)
          - flags[3] = V (oVerflow)
        In the current MVP simulator these are always ``(False, False, False,
        False)`` because no instruction sets the S-bit.  Reserved for future
        use when conditional instructions are implemented.
    memory:
        Snapshot of the full memory as immutable ``bytes``.  The size
        matches the ``memory_size`` passed to ``ARMSimulator.__init__``
        (default 65,536 bytes = 64 KiB).  Copying memory makes the snapshot
        independent of future writes.
    halted:
        True after the simulator executes the HLT sentinel (0xFFFFFFFF).

    Examples
    --------
    >>> from arm_simulator.state import ARMState
    >>> state = ARMState(
    ...     registers=tuple([0] * 16),
    ...     pc=0,
    ...     flags=(False, False, False, False),
    ...     memory=bytes(65536),
    ...     halted=False,
    ... )
    >>> state.pc
    0
    >>> state.registers[0]
    0
    >>> state.halted
    False
    >>> state.registers = tuple([1] * 16)  # doctest: +ELLIPSIS
    Traceback (most recent call last):
        ...
    dataclasses.FrozenInstanceError: ...
    """

    registers: tuple[int, ...]    # 16 × 32-bit registers R0–R15 (as tuple)
    pc: int                       # program counter — copy of registers[15]
    flags: tuple[bool, ...]       # (N, Z, C, V) condition flags as 4-tuple
    memory: bytes                 # full memory snapshot (immutable)
    halted: bool                  # True after HLT (0xFFFFFFFF) executed
