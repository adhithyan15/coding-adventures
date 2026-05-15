"""
ARMv7-A / Thumb-2 (2004) State Dataclass
==========================================

ARMv7-A is the 32-bit application profile architecture that powered the smartphone
revolution.  It introduced Thumb-2 — a variable-width encoding that mixes 16-bit
and 32-bit instruction halfwords in the same instruction stream.  Every iPhone up
to the 5s, every early Android phone, and the Raspberry Pi 1 and 2 run ARMv7-A.

Register set at a glance
-------------------------
  R0–R3    32-bit   Argument / result registers (caller-saved)
  R4–R11   32-bit   Variable registers (callee-saved)
  R12      32-bit   IP — intra-procedure scratch (caller-saved)
  R13      32-bit   SP — stack pointer
  R14      32-bit   LR — link register (written by BL/BLX)
  R15      32-bit   PC — program counter (reads return pc+4 in Thumb)

CPSR flags
----------
  N  bit 31  Negative flag
  Z  bit 30  Zero flag
  C  bit 29  Carry flag
  V  bit 28  Overflow flag
  T  bit  5  Thumb state (1 = Thumb mode)
"""

from __future__ import annotations

from dataclasses import dataclass

# ── Architecture constants ─────────────────────────────────────────────────────

MASK32: int = 0xFFFF_FFFF          # 32-bit unsigned mask
MASK16: int = 0xFFFF               # 16-bit unsigned mask
MASK8: int = 0xFF                  # 8-bit unsigned mask
SIGN32: int = 0x8000_0000          # sign bit of a 32-bit quantity
MEM_SIZE: int = 65_536             # 64 KiB flat byte-addressed memory
NUM_REGS: int = 16                 # R0–R15

# Register indices
SP: int = 13   # Stack pointer
LR: int = 14   # Link register
PC: int = 15   # Program counter

# CPSR flag bit positions
CPSR_N: int = 31   # Negative
CPSR_Z: int = 30   # Zero
CPSR_C: int = 29   # Carry
CPSR_V: int = 28   # Overflow
CPSR_T: int = 5    # Thumb state

# Condition codes — 4-bit values used in branch encodings
COND_EQ: int = 0b0000   # Equal:           Z=1
COND_NE: int = 0b0001   # Not equal:       Z=0
COND_CS: int = 0b0010   # Carry set:       C=1
COND_CC: int = 0b0011   # Carry clear:     C=0
COND_MI: int = 0b0100   # Minus:           N=1
COND_PL: int = 0b0101   # Plus:            N=0
COND_VS: int = 0b0110   # Overflow:        V=1
COND_VC: int = 0b0111   # No overflow:     V=0
COND_HI: int = 0b1000   # Unsigned higher: C=1 AND Z=0
COND_LS: int = 0b1001   # Unsigned ≤:      C=0 OR Z=1
COND_GE: int = 0b1010   # Signed ≥:        N=V
COND_LT: int = 0b1011   # Signed <:        N≠V
COND_GT: int = 0b1100   # Signed >:        Z=0 AND N=V
COND_LE: int = 0b1101   # Signed ≤:        Z=1 OR N≠V
COND_AL: int = 0b1110   # Always


# ── Sign-extension helpers ─────────────────────────────────────────────────────


def sext(v: int, bits: int) -> int:
    """Sign-extend an integer that occupies `bits` low bits to a Python signed int."""
    v = v & ((1 << bits) - 1)
    if v >> (bits - 1):
        return v - (1 << bits)
    return v


def sext8(v: int) -> int:
    """Sign-extend an 8-bit value."""
    return sext(v, 8)


def sext11(v: int) -> int:
    """Sign-extend an 11-bit value (Thumb unconditional branch offset)."""
    return sext(v, 11)


def sext24(v: int) -> int:
    """Sign-extend a 24-bit value (Thumb BL offset)."""
    return sext(v, 24)


# ── Immutable state snapshot ───────────────────────────────────────────────────


@dataclass(frozen=True)
class ARMv7AState:
    """
    Complete, immutable snapshot of the ARMv7-A simulator at one moment.

    Fields
    ------
    pc      Program counter — address of the *next* instruction to fetch.
            Initialized to 0 on reset.
    gpr     16 × 32-bit general-purpose registers (R0–R15).
            R13 = SP, R14 = LR, R15 = PC (but pc field is authoritative).
    cpsr    Current Program Status Register (32-bit):
              bit 31 = N, bit 30 = Z, bit 29 = C, bit 28 = V, bit 5 = T.
    memory  65 536 bytes stored as a tuple of Python ints (each 0–255).
    halted  True once the simulator fetches the all-zero halfword 0x0000,
            used as the simulation halt sentinel.
    """

    pc: int                   # program counter
    gpr: tuple[int, ...]      # 16 × 32-bit registers R0–R15
    cpsr: int                 # Current Program Status Register
    memory: tuple[int, ...]   # 65 536 bytes
    halted: bool

    # ── Convenience properties for named registers ────────────────────────────

    @property
    def r0(self) -> int: return self.gpr[0]
    @property
    def r1(self) -> int: return self.gpr[1]
    @property
    def r2(self) -> int: return self.gpr[2]
    @property
    def r3(self) -> int: return self.gpr[3]
    @property
    def r4(self) -> int: return self.gpr[4]
    @property
    def r5(self) -> int: return self.gpr[5]
    @property
    def r6(self) -> int: return self.gpr[6]
    @property
    def r7(self) -> int: return self.gpr[7]
    @property
    def r8(self) -> int: return self.gpr[8]
    @property
    def r9(self) -> int: return self.gpr[9]
    @property
    def r10(self) -> int: return self.gpr[10]
    @property
    def r11(self) -> int: return self.gpr[11]
    @property
    def r12(self) -> int: return self.gpr[12]   # IP

    @property
    def sp(self) -> int: return self.gpr[13]    # R13

    @property
    def lr(self) -> int: return self.gpr[14]    # R14

    # ── Condition flag properties ─────────────────────────────────────────────

    @property
    def n(self) -> bool:
        """Negative flag (bit 31 of CPSR)."""
        return bool((self.cpsr >> 31) & 1)

    @property
    def z(self) -> bool:
        """Zero flag (bit 30 of CPSR)."""
        return bool((self.cpsr >> 30) & 1)

    @property
    def c(self) -> bool:
        """Carry flag (bit 29 of CPSR)."""
        return bool((self.cpsr >> 29) & 1)

    @property
    def v(self) -> bool:
        """Overflow flag (bit 28 of CPSR)."""
        return bool((self.cpsr >> 28) & 1)

    @property
    def thumb(self) -> bool:
        """Thumb state bit (bit 5 of CPSR)."""
        return bool((self.cpsr >> 5) & 1)
