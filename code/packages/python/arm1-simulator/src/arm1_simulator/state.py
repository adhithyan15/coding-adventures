"""Frozen snapshot of ARM1 CPU state — used by the simulator-protocol interface.

Why a separate snapshot type?
------------------------------
The ``ARM1`` class holds *live, mutable* state: a list of 27 physical
registers, a bytearray of memory, and a handful of flags that change with
every instruction.  If you want to compare state *before* and *after* a run,
or if you want to stash the state from several different runs, you need a
way to freeze everything into an immutable value.

That is exactly what ``ARM1State`` does.  It is a ``frozen=True`` dataclass,
so Python will refuse any attempts to change its fields after construction::

    state = cpu.get_state()
    state.pc = 99  # -> FrozenInstanceError

This immutability guarantee is essential for the simulator-protocol contract:
``ExecutionResult.final_state`` must be a true snapshot, not a live reference
that silently changes as the CPU keeps running.

Field-by-field guide
--------------------
registers:
    Tuple of 16 ints — the *visible* registers R0–R15 as seen by user-mode
    code in the current processor mode.  R15 is the raw combined
    PC + flags + mode word (same bits as stored in ``_regs[15]``).

    Why 16, not 27?
    The ARM1 has 27 *physical* registers (to support fast FIQ banking), but
    user-mode code only sees 16 at a time.  The ``get_state()`` method uses
    ``read_register()`` which respects mode-banking, so you always get the
    logical view.

pc:
    The 26-bit program counter — bits 25:2 of R15.  Extracted for
    convenience so callers do not have to mask R15 themselves.

mode:
    Integer processor mode: 0=USR, 1=FIQ, 2=IRQ, 3=SVC.
    Extracted from bits 1:0 of R15.

flags_n, flags_z, flags_c, flags_v:
    Individual condition flags.  Extracted from bits 31:28 of R15.
    Named with the ``flags_`` prefix to avoid shadowing the Python builtins
    ``n``, ``z``, ``c``, ``v``.

memory:
    A ``bytes`` snapshot of the simulator's RAM.  Frozen by converting
    ``bytearray → bytes``, so later writes to the simulator's memory do NOT
    affect this snapshot.

halted:
    ``True`` if the CPU was in the halted state when the snapshot was taken.

banked_fiq:
    Tuple of 7 ints — the FIQ-mode banked physical registers
    R8_fiq … R14_fiq (physical indices 16–22).

banked_irq:
    Tuple of 2 ints — the IRQ-mode banked physical registers
    R13_irq, R14_irq (physical indices 23–24).

banked_svc:
    Tuple of 2 ints — the SVC-mode banked physical registers
    R13_svc, R14_svc (physical indices 25–26).
"""

from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True)
class ARM1State:
    """Immutable snapshot of the ARM1 CPU state at a single point in time.

    Captured by ``ARM1.get_state()`` and stored in
    ``ExecutionResult[ARM1State].final_state``.

    Attributes
    ----------
    registers:
        Visible registers R0–R15 (16 ints) for the current processor mode.
        R15 holds the raw PC + flags + mode word.
    pc:
        Program counter (26-bit address), extracted from bits 25:2 of R15.
    mode:
        Processor mode: 0=USR, 1=FIQ, 2=IRQ, 3=SVC.
    flags_n:
        Negative condition flag (bit 31 of R15).
    flags_z:
        Zero condition flag (bit 30 of R15).
    flags_c:
        Carry condition flag (bit 29 of R15).
    flags_v:
        Overflow condition flag (bit 28 of R15).
    memory:
        Bytes snapshot of the simulator's full memory at capture time.
    halted:
        ``True`` if the CPU had executed a HALT (SWI 0x123456) instruction.
    banked_fiq:
        FIQ-mode banked registers R8_fiq–R14_fiq (7 ints).
    banked_irq:
        IRQ-mode banked registers R13_irq, R14_irq (2 ints).
    banked_svc:
        SVC-mode banked registers R13_svc, R14_svc (2 ints).

    Examples
    --------
    >>> from arm1_simulator import ARM1
    >>> cpu = ARM1(256)
    >>> state = cpu.get_state()
    >>> state.pc
    0
    >>> state.mode  # SVC at power-on
    3
    >>> # Frozen — any write attempt raises FrozenInstanceError:
    >>> state.pc = 99  # doctest: +ELLIPSIS
    Traceback (most recent call last):
        ...
    dataclasses.FrozenInstanceError: ...
    """

    registers: tuple[int, ...]
    pc: int
    mode: int
    flags_n: bool
    flags_z: bool
    flags_c: bool
    flags_v: bool
    memory: bytes
    halted: bool
    banked_fiq: tuple[int, ...]
    banked_irq: tuple[int, ...]
    banked_svc: tuple[int, ...]
