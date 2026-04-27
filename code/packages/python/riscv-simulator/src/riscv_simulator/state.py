"""Frozen snapshot of RISC-V simulator state — used by the simulator-protocol.

Why a separate snapshot type?
------------------------------
``RiscVSimulator`` holds *live, mutable* state through its embedded ``CPU``
object: a ``RegisterFile`` of 32 mutable 32-bit registers, a ``Memory``
bytearray, a mutable PC integer, and a mutable halted flag.

If you want to compare state before and after a run, or stash the result of
several different programs, you need a way to freeze everything into an
immutable value.  ``RiscVState`` is that frozen value.

It is a ``frozen=True`` dataclass, so Python refuses any write after
construction::

    state = sim.get_state()
    state.pc = 99  # -> FrozenInstanceError

This immutability guarantee is essential for the simulator-protocol contract:
``ExecutionResult.final_state`` must be a true snapshot — not a live reference
that silently changes as execution continues.

Field-by-field guide
--------------------
registers:
    Tuple of 32 unsigned 32-bit ints — x0 … x31.  x0 is always 0 (the
    RISC-V zero-register invariant).  Captured via ``RegisterFile.read(i)``
    for i in 0..31.

pc:
    The 32-bit program counter value at the moment of capture.

csr_mstatus:
    Machine Status Register (CSR 0x300).  Controls interrupt enables.
    The MIE bit (bit 3) determines whether machine-mode interrupts fire.

csr_mtvec:
    Machine Trap Vector (CSR 0x305).  Holds the address of the trap handler.
    When an ecall/exception occurs (and mtvec != 0), execution jumps here.

csr_mscratch:
    Machine Scratch Register (CSR 0x340).  General-purpose scratch storage
    for the trap handler — often used to save/restore registers.

csr_mepc:
    Machine Exception PC (CSR 0x341).  Saves the PC of the instruction that
    caused the trap, so ``mret`` can restore it.

csr_mcause:
    Machine Cause Register (CSR 0x342).  Records the reason for the most
    recent trap — e.g., 11 = ecall from M-mode.

memory:
    A ``bytes`` snapshot of the simulator's full RAM.  Frozen by converting
    ``bytearray → bytes`` so later writes do NOT affect the snapshot.

halted:
    ``True`` if the simulator reached a HALT condition (``ecall`` with
    mtvec = 0, or ``ecall`` with mtvec set and no further instructions).
"""

from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True)
class RiscVState:
    """Immutable snapshot of the RISC-V RV32I simulator state.

    Captured by ``RiscVSimulator.get_state()`` and stored in
    ``ExecutionResult[RiscVState].final_state``.

    Attributes
    ----------
    registers:
        32-element tuple of unsigned 32-bit ints — x0 … x31.
        x0 is always 0 (hardwired zero register).
    pc:
        Program counter value at capture time.
    csr_mstatus:
        Machine Status Register (CSR 0x300).
    csr_mtvec:
        Machine Trap Vector (CSR 0x305).
    csr_mscratch:
        Machine Scratch Register (CSR 0x340).
    csr_mepc:
        Machine Exception PC (CSR 0x341).
    csr_mcause:
        Machine Cause Register (CSR 0x342).
    memory:
        Bytes snapshot of the simulator's full RAM.
    halted:
        ``True`` if the CPU was in the halted state when captured.

    Examples
    --------
    >>> from riscv_simulator import RiscVSimulator
    >>> sim = RiscVSimulator(256)
    >>> state = sim.get_state()
    >>> state.pc
    0
    >>> len(state.registers)
    32
    >>> # Frozen — any write attempt raises FrozenInstanceError:
    >>> state.pc = 99  # doctest: +ELLIPSIS
    Traceback (most recent call last):
        ...
    dataclasses.FrozenInstanceError: ...
    """

    registers: tuple[int, ...]
    pc: int
    csr_mstatus: int
    csr_mtvec: int
    csr_mscratch: int
    csr_mepc: int
    csr_mcause: int
    memory: bytes
    halted: bool
