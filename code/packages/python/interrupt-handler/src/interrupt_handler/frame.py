"""Interrupt Frame — saved CPU context for interrupt entry/exit.

When an interrupt fires, the CPU must save everything needed to resume
the interrupted code later. This is the interrupt frame (or trap frame).

Layout on the kernel stack (136 bytes total):
    PC (return address)         4 bytes
    MStatus register            4 bytes
    MCause register             4 bytes
    x1  (ra)                    4 bytes
    x2  (sp)                    4 bytes
    ...
    x31 (t6)                    4 bytes
    Total: 3 + 31 = 34 words = 136 bytes

Why save ALL 32 registers? The ISR is arbitrary code -- it might use any
register. Saving everything is safe and simple.
"""

from __future__ import annotations

from dataclasses import dataclass, field


@dataclass
class InterruptFrame:
    """Holds all CPU state needed to resume after an interrupt.

    Attributes:
        pc: Saved program counter (where to resume).
        registers: All 32 RISC-V general-purpose registers (x0-x31).
        mstatus: Machine status register.
        mcause: What caused the interrupt (interrupt number).
    """

    pc: int = 0
    registers: list[int] = field(default_factory=lambda: [0] * 32)
    mstatus: int = 0
    mcause: int = 0


def save_context(
    registers: list[int], pc: int, mstatus: int, mcause: int
) -> InterruptFrame:
    """Create an InterruptFrame from the current CPU state.

    Called at the beginning of interrupt handling, before the ISR runs.

    Args:
        registers: All 32 general-purpose registers (x0-x31).
        pc: The program counter (next instruction after interrupt).
        mstatus: The machine status register.
        mcause: The interrupt number that triggered the save.

    Returns:
        A complete InterruptFrame for later restoration.
    """
    return InterruptFrame(
        pc=pc,
        registers=list(registers),  # defensive copy
        mstatus=mstatus,
        mcause=mcause,
    )


def restore_context(
    frame: InterruptFrame,
) -> tuple[list[int], int, int]:
    """Extract CPU state from an InterruptFrame.

    Called after the ISR completes, to resume the interrupted code.

    Returns:
        Tuple of (registers, pc, mstatus).
    """
    return list(frame.registers), frame.pc, frame.mstatus
