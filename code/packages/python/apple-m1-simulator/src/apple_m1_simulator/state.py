"""
Apple M1 (2020) State Dataclass
=================================

The Apple M1 is Apple's first ARM-based SoC for Mac, released November 2020.
It implements ARMv8.4-A — the same AArch64 integer ISA plus full NEON/AdvSIMD
floating-point and vector support.

Register set at a glance
------------------------
Integer (GPR):
  X0–X7     64-bit   Argument / result registers
  X8        64-bit   Indirect result / syscall number
  X9–X15    64-bit   Caller-saved temporaries
  X16–X17   64-bit   Intra-procedure-call scratch (IP0/IP1)
  X18       64-bit   Platform register
  X19–X28   64-bit   Callee-saved registers
  X29       64-bit   Frame pointer (FP)
  X30       64-bit   Link register (LR) — written by BL/BLR; read by RET
  XZR       64-bit   Zero register — always reads 0; writes silently discarded
  SP        64-bit   Stack pointer (separate from GPR file)
  PC        64-bit   Program counter (not accessible as a GPR)

  W0–W30    32-bit   Low-word views of X0–X30 — writes zero-extend to X-width

NEON/FP:
  V0–V31    128-bit  NEON vector / FP registers (stored as Python int, 0..2^128-1)
  D0–D31    64-bit   Lower 64 bits of V0–V31 (IEEE 754 double-precision)
  S0–S31    32-bit   Lower 32 bits of V0–V31 (IEEE 754 single-precision)

  Write semantics:
    D-register write: zero-extends to 128 bits (upper 64 bits become 0)
    S-register write: zero-extends to 128 bits (upper 96 bits become 0)

Condition flags (NZCV)
----------------------
  N   Negative   MSB of result was 1 (bit 63 for 64-bit, bit 31 for 32-bit)
  Z   Zero       Result was zero
  C   Carry      Unsigned carry-out (or borrow-complement for subtract)
  V   Overflow   Signed overflow

  For FP compare (FCMP):
    Equal:     N=0, Z=1, C=1, V=0  (NZCV = 0b0110)
    Less:      N=1, Z=0, C=0, V=0  (NZCV = 0b1000)
    Greater:   N=0, Z=0, C=1, V=0  (NZCV = 0b0010)
    Unordered: N=0, Z=0, C=1, V=1  (NZCV = 0b0011)
"""

from __future__ import annotations

import struct
from dataclasses import dataclass

# ── Architecture constants ─────────────────────────────────────────────────────

MASK32: int = 0xFFFF_FFFF                       # 32-bit unsigned mask
MASK64: int = 0xFFFF_FFFF_FFFF_FFFF            # 64-bit unsigned mask
MASK128: int = (1 << 128) - 1                  # 128-bit unsigned mask
SIGN32: int = 0x8000_0000                       # sign bit of a 32-bit quantity
SIGN64: int = 0x8000_0000_0000_0000             # sign bit of a 64-bit quantity
MEM_SIZE: int = 65_536                          # 64 KiB flat byte-addressed memory
NUM_GPRS: int = 32                              # X0–X30, XZR (index 31 = always 0)
NUM_VREGS: int = 32                             # V0–V31
XZR: int = 31                                  # XZR register index

# NZCV flag bit weights within the 4-bit nzcv nibble.
NZCV_N: int = 0b1000    # Negative
NZCV_Z: int = 0b0100    # Zero
NZCV_C: int = 0b0010    # Carry
NZCV_V: int = 0b0001    # Overflow

# NZCV values for FP compare results (see AArch64 spec, C5.2.7 FCMP)
NZCV_FP_EQ: int = 0b0110   # Equal:     Z=1, C=1
NZCV_FP_LT: int = 0b1000   # Less than: N=1
NZCV_FP_GT: int = 0b0010   # Greater:   C=1
NZCV_FP_UN: int = 0b0011   # Unordered (NaN): C=1, V=1


# ── IEEE 754 bit-level helpers ─────────────────────────────────────────────────

def f64_from_bits(bits: int) -> float:
    """Convert a 64-bit integer bit-pattern to an IEEE 754 double."""
    return struct.unpack(">d", struct.pack(">Q", bits & MASK64))[0]


def f64_to_bits(f: float) -> int:
    """Convert an IEEE 754 double to a 64-bit integer bit-pattern."""
    return struct.unpack(">Q", struct.pack(">d", f))[0]


def f32_from_bits(bits: int) -> float:
    """Convert a 32-bit integer bit-pattern to an IEEE 754 single."""
    return struct.unpack(">f", struct.pack(">I", bits & MASK32))[0]


def f32_to_bits(f: float) -> int:
    """Convert an IEEE 754 single to a 32-bit integer bit-pattern."""
    return struct.unpack(">I", struct.pack(">f", f))[0]


# ── Sign-extension helpers ─────────────────────────────────────────────────────


def sext(v: int, bits: int) -> int:
    """Sign-extend an integer that occupies `bits` low bits to a Python signed int."""
    v = v & ((1 << bits) - 1)
    if v >> (bits - 1):
        return v - (1 << bits)
    return v


def sext12(v: int) -> int:
    """Sign-extend a 12-bit immediate."""
    return sext(v, 12)


def sext16(v: int) -> int:
    """Sign-extend a 16-bit value."""
    return sext(v, 16)


def sext19(v: int) -> int:
    """Sign-extend a 19-bit value (conditional / CBZ branch offset field)."""
    return sext(v, 19)


def sext26(v: int) -> int:
    """Sign-extend a 26-bit value (unconditional branch offset field)."""
    return sext(v, 26)


def sext32(v: int) -> int:
    """Sign-extend a 32-bit value to a Python signed integer."""
    return sext(v, 32)


# ── Immutable state snapshot ───────────────────────────────────────────────────


@dataclass(frozen=True)
class AppleM1State:
    """
    Complete, immutable snapshot of the Apple M1 simulator at one moment.

    Fields
    ------
    pc      Program counter — address of the *next* instruction to fetch (post-
            increment semantics; 0 on reset).
    gpr     32 × 64-bit general-purpose registers.
              Indices 0–30  → X0–X30 (real general-purpose registers).
              Index 31      → XZR (always 0; writes are silently discarded).
    sp      Stack pointer (64-bit).  Separate from the GPR file so it cannot be
            clobbered by an XZR write.
    nzcv    Condition flags, 4-bit nibble: N=bit3, Z=bit2, C=bit1, V=bit0.
    vreg    32 × 128-bit NEON/FP registers (V0–V31).  Each is a Python int in the
            range 0..2^128-1.  Lower 64 bits = D register view; lower 32 = S view.
    memory  65 536 bytes stored as a tuple of Python ints (each 0–255).
    halted  True once the simulator fetches the all-zero HALT word (0x00000000).
    """

    pc: int                         # program counter
    gpr: tuple[int, ...]            # 32 × 64-bit registers (X0–X30, XZR)
    sp: int                         # stack pointer
    nzcv: int                       # condition flags (4-bit)
    vreg: tuple[int, ...]           # 32 × 128-bit NEON/FP registers (V0–V31)
    memory: tuple[int, ...]         # 65 536 bytes
    halted: bool

    # ── GPR convenience properties ────────────────────────────────────────────

    @property
    def x0(self) -> int: return self.gpr[0]
    @property
    def x1(self) -> int: return self.gpr[1]
    @property
    def x2(self) -> int: return self.gpr[2]
    @property
    def x3(self) -> int: return self.gpr[3]
    @property
    def x4(self) -> int: return self.gpr[4]
    @property
    def x5(self) -> int: return self.gpr[5]
    @property
    def x6(self) -> int: return self.gpr[6]
    @property
    def x7(self) -> int: return self.gpr[7]
    @property
    def x8(self) -> int: return self.gpr[8]
    @property
    def x9(self) -> int: return self.gpr[9]
    @property
    def x10(self) -> int: return self.gpr[10]
    @property
    def x11(self) -> int: return self.gpr[11]
    @property
    def x12(self) -> int: return self.gpr[12]
    @property
    def x13(self) -> int: return self.gpr[13]
    @property
    def x14(self) -> int: return self.gpr[14]
    @property
    def x15(self) -> int: return self.gpr[15]
    @property
    def x16(self) -> int: return self.gpr[16]
    @property
    def x17(self) -> int: return self.gpr[17]
    @property
    def x18(self) -> int: return self.gpr[18]
    @property
    def x19(self) -> int: return self.gpr[19]
    @property
    def x20(self) -> int: return self.gpr[20]
    @property
    def x21(self) -> int: return self.gpr[21]
    @property
    def x22(self) -> int: return self.gpr[22]
    @property
    def x23(self) -> int: return self.gpr[23]
    @property
    def x24(self) -> int: return self.gpr[24]
    @property
    def x25(self) -> int: return self.gpr[25]
    @property
    def x26(self) -> int: return self.gpr[26]
    @property
    def x27(self) -> int: return self.gpr[27]
    @property
    def x28(self) -> int: return self.gpr[28]
    @property
    def x29(self) -> int: return self.gpr[29]
    @property
    def x30(self) -> int: return self.gpr[30]   # Link register (LR)

    # W-register views (32-bit lower halves)
    @property
    def w0(self) -> int: return self.gpr[0] & MASK32
    @property
    def w1(self) -> int: return self.gpr[1] & MASK32
    @property
    def w2(self) -> int: return self.gpr[2] & MASK32
    @property
    def w3(self) -> int: return self.gpr[3] & MASK32
    @property
    def w4(self) -> int: return self.gpr[4] & MASK32
    @property
    def w5(self) -> int: return self.gpr[5] & MASK32

    # ── Condition flag properties ─────────────────────────────────────────────

    @property
    def n(self) -> bool:
        """Negative flag."""
        return bool((self.nzcv >> 3) & 1)

    @property
    def z(self) -> bool:
        """Zero flag."""
        return bool((self.nzcv >> 2) & 1)

    @property
    def c(self) -> bool:
        """Carry flag."""
        return bool((self.nzcv >> 1) & 1)

    @property
    def v(self) -> bool:
        """Overflow flag."""
        return bool(self.nzcv & 1)

    # ── NEON/FP register properties ───────────────────────────────────────────
    # Raw 128-bit integer view of each V register.

    @property
    def v0(self) -> int: return self.vreg[0]
    @property
    def v1(self) -> int: return self.vreg[1]
    @property
    def v2(self) -> int: return self.vreg[2]
    @property
    def v3(self) -> int: return self.vreg[3]
    @property
    def v4(self) -> int: return self.vreg[4]
    @property
    def v5(self) -> int: return self.vreg[5]
    @property
    def v6(self) -> int: return self.vreg[6]
    @property
    def v7(self) -> int: return self.vreg[7]

    # D-register (lower 64-bit integer) views.
    @property
    def d0_bits(self) -> int: return self.vreg[0] & MASK64
    @property
    def d1_bits(self) -> int: return self.vreg[1] & MASK64
    @property
    def d2_bits(self) -> int: return self.vreg[2] & MASK64
    @property
    def d3_bits(self) -> int: return self.vreg[3] & MASK64
    @property
    def d4_bits(self) -> int: return self.vreg[4] & MASK64
    @property
    def d5_bits(self) -> int: return self.vreg[5] & MASK64
    @property
    def d6_bits(self) -> int: return self.vreg[6] & MASK64
    @property
    def d7_bits(self) -> int: return self.vreg[7] & MASK64

    # D-register (lower 64-bit) float views (IEEE 754 double).
    @property
    def d0(self) -> float: return f64_from_bits(self.vreg[0])
    @property
    def d1(self) -> float: return f64_from_bits(self.vreg[1])
    @property
    def d2(self) -> float: return f64_from_bits(self.vreg[2])
    @property
    def d3(self) -> float: return f64_from_bits(self.vreg[3])
    @property
    def d4(self) -> float: return f64_from_bits(self.vreg[4])
    @property
    def d5(self) -> float: return f64_from_bits(self.vreg[5])
    @property
    def d6(self) -> float: return f64_from_bits(self.vreg[6])
    @property
    def d7(self) -> float: return f64_from_bits(self.vreg[7])

    # S-register (lower 32-bit) float views (IEEE 754 single).
    @property
    def s0(self) -> float: return f32_from_bits(self.vreg[0])
    @property
    def s1(self) -> float: return f32_from_bits(self.vreg[1])
    @property
    def s2(self) -> float: return f32_from_bits(self.vreg[2])
    @property
    def s3(self) -> float: return f32_from_bits(self.vreg[3])
    @property
    def s4(self) -> float: return f32_from_bits(self.vreg[4])
    @property
    def s5(self) -> float: return f32_from_bits(self.vreg[5])
    @property
    def s6(self) -> float: return f32_from_bits(self.vreg[6])
    @property
    def s7(self) -> float: return f32_from_bits(self.vreg[7])


def make_initial_state() -> AppleM1State:
    """Return the power-on / reset state: all registers and memory zeroed, PC=0."""
    return AppleM1State(
        pc=0,
        gpr=tuple(0 for _ in range(NUM_GPRS)),
        sp=0,
        nzcv=0,
        vreg=tuple(0 for _ in range(NUM_VREGS)),
        memory=tuple(0 for _ in range(MEM_SIZE)),
        halted=False,
    )
