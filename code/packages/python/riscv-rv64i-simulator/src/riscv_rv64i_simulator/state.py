"""
RV64I architectural constants and immutable state snapshot.

RISC-V has 32 general-purpose 64-bit integer registers (x0–x31) plus a
program counter.  Register x0 is hardwired to zero — reads always return 0
and writes are silently discarded.

ABI register aliases (used in assembly; the simulator only tracks numbers):

  zero = x0   ra = x1   sp = x2   gp = x3   tp = x4
  t0–t2 = x5–x7          s0/fp = x8     s1 = x9
  a0–a1 = x10–x11        a2–a7 = x12–x17
  s2–s11 = x18–x27       t3–t6 = x28–x31
"""

from __future__ import annotations

from dataclasses import dataclass

# ── Constants ──────────────────────────────────────────────────────────────────

NUM_REGS: int = 32       # x0–x31
MEM_SIZE: int = 65_536   # 64 KiB flat address space

MASK64: int = 0xFFFF_FFFF_FFFF_FFFF   # 64-bit mask
MASK32: int = 0xFFFF_FFFF              # 32-bit mask
MASK16: int = 0xFFFF                   # 16-bit mask
MASK8:  int = 0xFF                     # 8-bit mask

# ABI register numbers
ZERO: int = 0    # hardwired zero
RA:   int = 1    # return address
SP:   int = 2    # stack pointer

# Sign bit positions
SIGN64: int = 1 << 63
SIGN32: int = 1 << 31

# ── Sign-extension helpers ─────────────────────────────────────────────────────


def sext(value: int, bits: int) -> int:
    """
    Sign-extend a `bits`-wide value to Python's arbitrary-precision integer.

    If the MSB (bit `bits-1`) is set, the value is negative; we extend it by
    flipping all higher bits.

    Example:
      sext(0xFF, 8)  →  -1   (8-bit 0xFF = -1 in two's complement)
      sext(0x7F, 8)  → 127   (positive, no extension)
    """
    if value & (1 << (bits - 1)):
        value -= (1 << bits)
    return value


def sext12(v: int) -> int:
    """Sign-extend a 12-bit immediate to Python int."""
    return sext(v & 0xFFF, 12)


def sext13(v: int) -> int:
    """Sign-extend a 13-bit B-type immediate."""
    return sext(v & 0x1FFF, 13)


def sext21(v: int) -> int:
    """Sign-extend a 21-bit J-type immediate."""
    return sext(v & 0x1FFFFF, 21)


def sext32_to_64(v: int) -> int:
    """
    Sign-extend a 32-bit value to a Python 64-bit two's-complement integer.

    Used for RV64I "word" instructions (ADDW, SLLW, etc.) where the 32-bit
    result must be sign-extended into the full 64-bit register.

    Example:
      sext32_to_64(0xFFFF_FFFE)  →  -2   (0xFFFF_FFFF_FFFF_FFFE)
      sext32_to_64(0x0000_0001)  →   1
    """
    return sext(v & MASK32, 32)


def to_signed64(v: int) -> int:
    """Interpret a 64-bit unsigned value as a signed Python integer."""
    v &= MASK64
    if v & SIGN64:
        v -= (1 << 64)
    return v


def to_signed32(v: int) -> int:
    """Interpret the lower 32 bits of v as a signed Python integer."""
    v &= MASK32
    if v & SIGN32:
        v -= (1 << 32)
    return v


# ── Immutable state snapshot ───────────────────────────────────────────────────


@dataclass(frozen=True)
class RV64IState:
    """
    Frozen snapshot of the RV64I simulator state, returned by get_state()
    and execute().

    Attributes
    ----------
    pc       : Program counter (byte address).
    gpr      : Tuple of 32 × 64-bit general-purpose register values.
               Index 0 is always 0 (the zero register).
    memory   : Tuple of MEM_SIZE bytes (the full address space).
    halted   : True after the halt sentinel (0x00000000) has been fetched.
    """

    pc:     int
    gpr:    tuple[int, ...]    # length 32
    memory: tuple[int, ...]    # length MEM_SIZE
    halted: bool

    # ── Convenience register properties ──────────────────────────────────────

    @property
    def zero(self) -> int: return 0          # always 0

    @property
    def ra(self) -> int: return self.gpr[1]  # return address

    @property
    def sp(self) -> int: return self.gpr[2]  # stack pointer

    @property
    def gp(self) -> int: return self.gpr[3]  # global pointer

    @property
    def tp(self) -> int: return self.gpr[4]  # thread pointer

    # Temporaries t0–t2
    @property
    def t0(self) -> int: return self.gpr[5]

    @property
    def t1(self) -> int: return self.gpr[6]

    @property
    def t2(self) -> int: return self.gpr[7]

    # Saved s0/fp, s1
    @property
    def s0(self) -> int: return self.gpr[8]

    @property
    def fp(self) -> int: return self.gpr[8]   # alias for s0

    @property
    def s1(self) -> int: return self.gpr[9]

    # Arguments / return values a0–a7
    @property
    def a0(self) -> int: return self.gpr[10]

    @property
    def a1(self) -> int: return self.gpr[11]

    @property
    def a2(self) -> int: return self.gpr[12]

    @property
    def a3(self) -> int: return self.gpr[13]

    @property
    def a4(self) -> int: return self.gpr[14]

    @property
    def a5(self) -> int: return self.gpr[15]

    @property
    def a6(self) -> int: return self.gpr[16]

    @property
    def a7(self) -> int: return self.gpr[17]

    # Saved s2–s11
    @property
    def s2(self) -> int: return self.gpr[18]

    @property
    def s3(self) -> int: return self.gpr[19]

    @property
    def s4(self) -> int: return self.gpr[20]

    @property
    def s5(self) -> int: return self.gpr[21]

    @property
    def s6(self) -> int: return self.gpr[22]

    @property
    def s7(self) -> int: return self.gpr[23]

    @property
    def s8(self) -> int: return self.gpr[24]

    @property
    def s9(self) -> int: return self.gpr[25]

    @property
    def s10(self) -> int: return self.gpr[26]

    @property
    def s11(self) -> int: return self.gpr[27]

    # Temporaries t3–t6
    @property
    def t3(self) -> int: return self.gpr[28]

    @property
    def t4(self) -> int: return self.gpr[29]

    @property
    def t5(self) -> int: return self.gpr[30]

    @property
    def t6(self) -> int: return self.gpr[31]
