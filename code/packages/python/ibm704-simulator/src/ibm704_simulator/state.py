"""IBM704State — an immutable snapshot of the IBM 704 mainframe's state.

The live ``IBM704Simulator`` holds *mutable* state — its accumulator,
registers, memory, and program counter all change with every instruction. But
for testing and debugging we need a *point-in-time snapshot*: a value that
captures the complete machine state at a specific moment and never changes
afterward, even if the simulator continues executing.

Without immutability you would write::

    state_before = sim.get_state()       # BUG: just a reference, not a copy
    sim.execute(more_code)               # mutates "state_before" too!
    assert state_before.accumulator_magnitude == 5  # may fail unexpectedly

With a frozen dataclass, ``get_state()`` returns a *copy* with all mutable
collections converted to immutable equivalents (lists → tuples). The snapshot
is a true value, not a reference. Attempting to mutate it raises
``dataclasses.FrozenInstanceError``.

Why so many separate fields?
----------------------------
The 704's accumulator is **38 bits**, not 36 — it has the standard sign +
35-bit magnitude *plus* two overflow-detection bits (Q and P). Modeling these
as separate booleans keeps the math obvious in the simulator and makes it
trivial to assert ``state.overflow_trigger is True`` in tests.

The MQ register is exposed twice: once as the raw 36-bit ``mq`` (sign in
bit 35) for callers that want bit-level fidelity, and once as the convenience
pair ``mq_sign`` / ``mq_magnitude`` for callers that prefer not to mask
manually.

Memory layout
-------------
Memory is 32,768 words. In the live simulator that is a Python ``list[int]``;
in the snapshot it is a ``tuple[int, ...]`` so the dataclass can be frozen.
Each entry is a plain int in ``[0, 2**36)``. To read word at address Y::

    word = state.memory[Y]
    sign = (word >> 35) & 1
    magnitude = word & ((1 << 35) - 1)

For convenience, ``ibm704_simulator.word`` has helpers (``word_sign``,
``word_magnitude``, ``word_to_signed_int``) that do this for you.
"""

from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True)
class IBM704State:
    """Immutable snapshot of IBM 704 CPU state at a point in time.

    All mutable collections from the live simulator are converted to tuples
    here so this dataclass can be frozen. The snapshot is a true value — it
    will not change even if the simulator continues executing.

    Attributes
    ----------
    accumulator_sign:
        AC sign bit. ``True`` if the accumulator value is negative.
    accumulator_p:
        AC bit P (overflow indicator). Set when an arithmetic operation
        carried into the P position. If P is true after an arithmetic
        instruction, the ``overflow_trigger`` is also set.
    accumulator_q:
        AC bit Q (overflow indicator). Set when an arithmetic operation
        carried into the Q position.
    accumulator_magnitude:
        AC bits 1-35 — the 35-bit magnitude. Range: ``[0, 2**35 - 1]``.
    mq:
        The full 36-bit MQ register as a raw integer (sign in bit 35).
    mq_sign:
        ``True`` if MQ is negative (convenience field; equivalent to
        ``bool(mq & (1 << 35))``).
    mq_magnitude:
        The 35-bit MQ magnitude (convenience field).
    index_a, index_b, index_c:
        The three index registers, 15 bits each. Tag bits 1, 2, and 4
        select these (in the order A, B, C).
    pc:
        Program counter, 15 bits. Address of the *next* instruction to
        fetch.
    halted:
        ``True`` if the machine is halted (HTR or HPR was executed, or an
        unrecoverable error such as divide-check on DVH).
    overflow_trigger:
        Set to ``True`` whenever an arithmetic operation overflowed (P bit
        set). Cleared by ``TOV`` (transfer on overflow) when that
        instruction takes its branch. Programs poll this trigger to detect
        and recover from numeric overflow.
    divide_check_trigger:
        Set to ``True`` if a divide instruction's quotient does not fit in
        the MQ register. ``DVH`` halts on a divide-check; ``DVP`` proceeds
        and lets the program test the trigger.
    memory:
        All 32,768 words of core memory, each a 36-bit integer. Indexed
        ``memory[address]`` for ``0 <= address < 32768``.

    Examples
    --------
    >>> state = IBM704State(
    ...     accumulator_sign=False,
    ...     accumulator_p=False,
    ...     accumulator_q=False,
    ...     accumulator_magnitude=42,
    ...     mq=0,
    ...     mq_sign=False,
    ...     mq_magnitude=0,
    ...     index_a=0,
    ...     index_b=0,
    ...     index_c=0,
    ...     pc=10,
    ...     halted=True,
    ...     overflow_trigger=False,
    ...     divide_check_trigger=False,
    ...     memory=tuple([0] * 32768),
    ... )
    >>> state.accumulator_magnitude
    42
    >>> state.pc
    10
    >>> state.halted
    True
    >>> state.memory[0]
    0
    """

    accumulator_sign: bool
    accumulator_p: bool
    accumulator_q: bool
    accumulator_magnitude: int
    mq: int
    mq_sign: bool
    mq_magnitude: int
    index_a: int
    index_b: int
    index_c: int
    pc: int
    halted: bool
    overflow_trigger: bool
    divide_check_trigger: bool
    memory: tuple[int, ...]
