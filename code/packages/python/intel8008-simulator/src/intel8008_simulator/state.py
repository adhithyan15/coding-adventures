"""Intel 8008 state snapshot — immutable CPU state for the simulator-protocol.

=== Why a Separate State Module? ===

The ``Intel8008Simulator`` class holds *mutable* internal state — registers,
flags, stack, and memory all change as instructions execute.  But the
``Simulator[StateT]`` protocol requires ``get_state()`` to return an
*immutable* snapshot that won't change even if the simulator keeps running.

This module defines two frozen dataclasses:

    ``Intel8008Flags``  — immutable copy of the four condition flags
    ``Intel8008State``  — immutable snapshot of the full CPU state

"Frozen" means Python will raise ``FrozenInstanceError`` if you try to
change any field after construction.  This is safe to pass around, store in
lists, or diff between two points in execution.

=== Intel 8008 Register Map (quick reference) ===

    A  — accumulator (all ALU operations write here)
    B  — general purpose
    C  — general purpose
    D  — general purpose
    E  — general purpose
    H  — high byte of the HL address pair (used for indirect memory access)
    L  — low byte of the HL address pair
    PC — program counter (lives in stack[0] of the push-down stack)

=== The Push-Down Stack ===

Unlike most architectures, the 8008 has NO stack pointer.  Instead it has
eight 14-bit registers arranged as a push-down stack.  Entry 0 is ALWAYS
the current PC.  Entries 1–7 are saved return addresses (LIFO).  CALL
rotates entries down; RET rotates them up.  The ``stack`` field here
captures all eight entries as an immutable tuple.
"""

from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True)
class Intel8008Flags:
    """Immutable snapshot of the Intel 8008 condition flags.

    These four flags are set by ALU operations and tested by conditional
    jumps, calls, and returns.

    Attributes
    ----------
    carry:
        CY — set if the last addition overflowed 8 bits, or if the last
        subtraction required a borrow (CY=1 after SUB means ``a < b``).
        AND/OR/XOR always clear CY.  INR/DCR do NOT touch CY.
    zero:
        Z — set when the 8-bit result was exactly 0x00.
    sign:
        S — set when bit 7 of the result was 1 (the result is negative in
        two's complement: value in range 0x80–0xFF).
    parity:
        P — set when the result has *even* parity (an even count of 1-bits).
        P=0 means odd parity.  This matches the 8008 and 8080 convention.

    Examples
    --------
    >>> flags = Intel8008Flags(carry=False, zero=True, sign=False, parity=True)
    >>> flags.zero
    True
    >>> flags.carry = True  # doctest: +ELLIPSIS
    Traceback (most recent call last):
        ...
    dataclasses.FrozenInstanceError: ...
    """

    carry: bool
    zero: bool
    sign: bool
    parity: bool


@dataclass(frozen=True)
class Intel8008State:
    """Immutable snapshot of Intel 8008 CPU state at a single point in time.

    All fields are value types or immutable containers (tuple, bytes) so
    the snapshot is guaranteed not to change after construction, even if
    the simulator continues executing.

    Attributes
    ----------
    a:
        Accumulator register (8-bit, 0–255).  The implicit target of all
        ALU operations.
    b, c, d, e, h, l:
        General-purpose registers (8-bit each).  H:L form a 14-bit memory
        address pair used for indirect memory access via the M pseudo-register.
    pc:
        Program counter (14-bit, range 0–16383).  This is a copy of
        ``stack[0]`` — on the real chip the PC lives at the top of the
        push-down stack.
    flags:
        Immutable snapshot of CY/Z/S/P condition flags at this moment.
    stack:
        All eight push-down stack entries as a tuple (index 0 = PC,
        indices 1–7 = saved return addresses in LIFO order).
    stack_depth:
        Number of *saved* return addresses currently live (0–7).  Does
        not count entry 0 (the PC).  Equals the current nesting depth.
    memory:
        Full 16,384-byte address space as an immutable ``bytes`` object.
        Copying the entire memory makes the snapshot truly independent of
        future writes.  (16 KiB is small enough to copy cheaply.)
    halted:
        True if the processor has executed a HLT instruction and stopped.

    Examples
    --------
    >>> from intel8008_simulator.state import Intel8008Flags, Intel8008State
    >>> flags = Intel8008Flags(carry=False, zero=True, sign=False, parity=True)
    >>> state = Intel8008State(
    ...     a=0, b=0, c=0, d=0, e=0, h=0, l=0,
    ...     pc=3,
    ...     flags=flags,
    ...     stack=tuple([3, 0, 0, 0, 0, 0, 0, 0]),
    ...     stack_depth=0,
    ...     memory=bytes(16384),
    ...     halted=True,
    ... )
    >>> state.pc
    3
    >>> state.halted
    True
    """

    a: int            # accumulator (8-bit, 0–255)
    b: int            # register B (8-bit)
    c: int            # register C (8-bit)
    d: int            # register D (8-bit)
    e: int            # register E (8-bit)
    h: int            # register H — high byte of HL address pair (8-bit)
    l: int            # register L — low byte of HL address pair (8-bit)  # noqa: E741
    pc: int           # program counter (14-bit, 0–16383) — copy of stack[0]
    flags: Intel8008Flags
    stack: tuple[int, ...]   # 8-level push-down stack as immutable tuple
    stack_depth: int         # number of saved return addresses (0–7)
    memory: bytes            # full 16 KiB memory snapshot (immutable)
    halted: bool             # True after HLT is executed
