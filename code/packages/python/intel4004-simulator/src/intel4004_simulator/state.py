"""Intel4004State — an immutable snapshot of the Intel 4004 CPU state.

=== Why an Immutable Snapshot? ===

The Intel 4004 simulator object holds *mutable* state — its registers, RAM,
and program counter change with every instruction.  But for testing and
debugging, we need a *point-in-time snapshot*: a value that captures the
complete CPU state at a specific moment and never changes afterward.

Without immutability, you could write:

    state_before = sim.get_state()   # BUG: state_before is just a reference
    sim.run(more_code)               #      it reflects the new state now!
    assert state_before.accumulator == 5  # fails — accumulator has changed

With a frozen dataclass, ``get_state()`` returns a *copy* with all mutable
lists converted to immutable tuples.  The snapshot is a true value, not a
reference.

=== RAM Layout ===

The 4004 has a two-level RAM structure:

    4 banks × 4 registers × 16 main nibbles  (+ 4 status nibbles per register)

In the live simulator this is stored as nested Python lists:
``list[list[list[int]]]``.  Lists are mutable, so they cannot be part of a
frozen dataclass.  We convert them to nested tuples when creating the snapshot:

    ram: tuple[tuple[tuple[int, ...], ...], ...]

To read a value from a snapshot:

    value = state.ram[bank_index][register_index][character_index]

=== Hardware Stack ===

The 4004 has a 3-level hardware stack for subroutine return addresses (12-bit
addresses each).  Live state is a 3-element list; the snapshot stores it as a
tuple of 3 integers.

=== Usage ===

    from intel4004_simulator import Intel4004Simulator, Intel4004State

    sim = Intel4004Simulator()
    result = sim.execute(program_bytes)
    state: Intel4004State = result.final_state

    # All fields are directly accessible
    print(state.accumulator)    # 4-bit accumulator (0–15)
    print(state.registers[0])   # R0 value
    print(state.carry)          # True/False
    print(state.pc)             # program counter (0–4095)
    print(state.halted)         # True if HALT was executed

    # RAM: bank 0, register 0, character 0
    print(state.ram[0][0][0])

    # Frozen — this raises FrozenInstanceError:
    # state.accumulator = 99
"""

from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True)
class Intel4004State:
    """Immutable snapshot of Intel 4004 CPU state at a point in time.

    All mutable collections (lists) from the live simulator are converted to
    tuples so this dataclass can be frozen.  The snapshot is a true value —
    it will not change even if the simulator continues executing.

    Attributes
    ----------
    accumulator:
        The 4-bit accumulator (0–15).  Most arithmetic in the 4004 passes
        through this register.
    registers:
        All 16 × 4-bit general-purpose registers (R0–R15) as a tuple.
        ``registers[n]`` is the value of register Rn (0–15).
    carry:
        The carry/borrow flag.  Set by ADD/SUB and bit-rotation instructions.
        Note: in SUB/SBM the carry is *inverted* — ``True`` means no borrow.
    pc:
        Program counter (0–4095).  Points to the *next* instruction to fetch
        after the last executed instruction.
    halted:
        ``True`` if the CPU halted (HLT instruction was executed).
    ram:
        4 banks × 4 registers × 16 main nibbles.
        Indexed as ``ram[bank][register][character]``.
        Each nibble is 0–15.
    hw_stack:
        The 3 hardware return-address slots (12-bit addresses, 0–4095).
        The 4004 stack is circular — pushing a 4th address overwrites the
        oldest.  Stored as a tuple of exactly 3 integers.
    stack_pointer:
        Current top-of-stack index (0–2).  Points to where the *next* push
        will write (i.e., the next free slot, not the last pushed value).

    Examples
    --------
    >>> from dataclasses import FrozenInstanceError
    >>> state = Intel4004State(
    ...     accumulator=5,
    ...     registers=tuple([0] * 16),
    ...     carry=False,
    ...     pc=10,
    ...     halted=True,
    ...     ram=tuple(tuple(tuple(0 for _ in range(16)) for _ in range(4)) for _ in range(4)),
    ...     hw_stack=(0, 0, 0),
    ...     stack_pointer=0,
    ... )
    >>> state.accumulator
    5
    >>> state.pc
    10
    >>> state.halted
    True
    >>> state.registers[0]
    0
    >>> state.ram[0][0][0]
    0
    >>> state.hw_stack
    (0, 0, 0)
    """

    accumulator: int
    registers: tuple[int, ...]
    carry: bool
    pc: int
    halted: bool
    ram: tuple[tuple[tuple[int, ...], ...], ...]
    hw_stack: tuple[int, ...]
    stack_pointer: int
