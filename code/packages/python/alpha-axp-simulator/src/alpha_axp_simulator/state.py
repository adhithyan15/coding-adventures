"""AlphaState — frozen CPU snapshot for the DEC Alpha AXP 21064 simulator.

The Alpha AXP has a beautifully simple state model compared to SPARC V8:
  - No condition code register (comparisons write 0/1 to GPRs)
  - No register windows (flat 32-register file like MIPS)
  - No Y/Hi-Lo register (64-bit multiply result fits in one GPR)
  - Just: PC, nPC, 32 registers (64-bit), 64 KiB of memory

Architecture constants
──────────────────────
  NUM_REGS = 32    — r0 through r31 (r31 hardwired zero)
  MEM_SIZE = 64 KiB

ABI register aliases
────────────────────
  r26 = ra   (return address, written by BSR / JSR)
  r27 = pv   (procedure value — indirect call target)
  r29 = gp   (global pointer)
  r30 = sp   (stack pointer)
  r31 = zero (hardwired zero — reads 0, writes discarded)
"""

from __future__ import annotations

import dataclasses

# ── Architecture constants ─────────────────────────────────────────────────────

MEM_SIZE: int = 0x10000   # 64 KiB flat address space
NUM_REGS: int = 32        # r0–r31

# ABI register numbers
REG_ZERO: int = 31   # hardwired zero
REG_RA:   int = 26   # return address (link register)
REG_PV:   int = 27   # procedure value
REG_GP:   int = 29   # global pointer
REG_SP:   int = 30   # stack pointer

# HALT = call_pal 0x0000 = the all-zeros 32-bit word.
# Caution: opcode 0x00 with palcode 0 encodes as 0x00000000.  An uninitialized
# memory region (filled with zeros) will therefore halt the simulator
# immediately, which is convenient for test programs.
HALT_WORD: int = 0x0000_0000


# ── Helper: sign-extend a 64-bit register value to a Python signed int ─────────

def _as_signed64(v: int) -> int:
    """Reinterpret unsigned 64-bit value as signed (two's complement)."""
    v = v & 0xFFFF_FFFF_FFFF_FFFF
    if v >= 0x8000_0000_0000_0000:
        v -= 0x1_0000_0000_0000_0000
    return v


# ── Virtual-register to physical convenience ──────────────────────────────────

def _r(regs: tuple[int, ...], n: int) -> int:
    """Read virtual register n from a regs tuple.  r31 always returns 0."""
    if n == REG_ZERO:
        return 0
    return regs[n]


# ── AlphaState frozen dataclass ────────────────────────────────────────────────

@dataclasses.dataclass(frozen=True)
class AlphaState:
    """Immutable snapshot of the DEC Alpha AXP 21064 CPU state.

    All register values are unsigned 64-bit integers (Python ints in the range
    [0, 2^64-1]).  Signed interpretations are performed by the simulator as
    needed; this dataclass stores raw unsigned values.

    regs[31] is always 0 — the AlphaSimulator never stores a non-zero value
    there, and _r() enforces this defensively.

    memory is a 65536-element tuple of bytes (values 0–255).
    """

    pc:     int                # program counter (64-bit, wraps at MEM_SIZE)
    npc:    int                # next-PC (pc+4 in the normal pipeline)
    regs:   tuple[int, ...]    # 32 registers, each 64-bit unsigned
    memory: tuple[int, ...]    # 65536 bytes (little-endian)
    halted: bool

    # ── Register convenience properties ──────────────────────────────────────

    @property
    def r0(self)  -> int: return _r(self.regs, 0)
    @property
    def r1(self)  -> int: return _r(self.regs, 1)
    @property
    def r2(self)  -> int: return _r(self.regs, 2)
    @property
    def r3(self)  -> int: return _r(self.regs, 3)
    @property
    def r4(self)  -> int: return _r(self.regs, 4)
    @property
    def r5(self)  -> int: return _r(self.regs, 5)
    @property
    def r6(self)  -> int: return _r(self.regs, 6)
    @property
    def r7(self)  -> int: return _r(self.regs, 7)
    @property
    def r8(self)  -> int: return _r(self.regs, 8)
    @property
    def r9(self)  -> int: return _r(self.regs, 9)
    @property
    def r10(self) -> int: return _r(self.regs, 10)
    @property
    def r11(self) -> int: return _r(self.regs, 11)
    @property
    def r12(self) -> int: return _r(self.regs, 12)
    @property
    def r13(self) -> int: return _r(self.regs, 13)
    @property
    def r14(self) -> int: return _r(self.regs, 14)
    @property
    def r15(self) -> int: return _r(self.regs, 15)
    @property
    def r16(self) -> int: return _r(self.regs, 16)
    @property
    def r17(self) -> int: return _r(self.regs, 17)
    @property
    def r18(self) -> int: return _r(self.regs, 18)
    @property
    def r19(self) -> int: return _r(self.regs, 19)
    @property
    def r20(self) -> int: return _r(self.regs, 20)
    @property
    def r21(self) -> int: return _r(self.regs, 21)
    @property
    def r22(self) -> int: return _r(self.regs, 22)
    @property
    def r23(self) -> int: return _r(self.regs, 23)
    @property
    def r24(self) -> int: return _r(self.regs, 24)
    @property
    def r25(self) -> int: return _r(self.regs, 25)
    @property
    def r26(self) -> int: return _r(self.regs, 26)
    @property
    def r27(self) -> int: return _r(self.regs, 27)
    @property
    def r28(self) -> int: return _r(self.regs, 28)
    @property
    def r29(self) -> int: return _r(self.regs, 29)
    @property
    def r30(self) -> int: return _r(self.regs, 30)
    @property
    def r31(self) -> int: return 0   # always zero by definition

    # ABI aliases
    @property
    def ra(self)   -> int: return _r(self.regs, REG_RA)    # r26: return address
    @property
    def pv(self)   -> int: return _r(self.regs, REG_PV)    # r27: procedure value
    @property
    def gp(self)   -> int: return _r(self.regs, REG_GP)    # r29: global pointer
    @property
    def sp(self)   -> int: return _r(self.regs, REG_SP)    # r30: stack pointer
    @property
    def zero(self) -> int: return 0                         # r31: hardwired zero
