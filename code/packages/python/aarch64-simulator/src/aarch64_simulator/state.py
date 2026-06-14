"""
AArch64 (2011) State Dataclass
================================

AArch64 is the 64-bit execution state introduced by ARMv8-A in 2011.  It
powers every Apple Silicon chip (M1 / M2 / M3 / M4), AWS Graviton, and every
modern smartphone.

Register set at a glance
------------------------
  X0–X7     64-bit   Argument / result registers
  X8        64-bit   Indirect result / syscall number
  X9–X15    64-bit   Caller-saved temporaries
  X16–X17   64-bit   Intra-procedure-call scratch (IP0/IP1)
  X18       64-bit   Platform register
  X19–X28   64-bit   Callee-saved registers
  X29       64-bit   Frame pointer (FP)
  X30       64-bit   Link register (LR) — written by BL/BLR; read by RET
  XZR       64-bit   Zero register — always reads 0; writes silently discarded
             (register index 31 in most instruction encodings)
  SP        64-bit   Stack pointer
  PC        64-bit   Program counter (not directly accessible as GPR)

  W0–W30    32-bit   Low-word views of X0–X30 — writes zero-extend to X-width

Condition flags (NZCV)
----------------------
  N   Negative   MSB of result was 1 (bit 63 for 64-bit, bit 31 for 32-bit)
  Z   Zero       Result was zero
  C   Carry      Unsigned carry-out (or borrow-complement for subtract)
  V   Overflow   Signed overflow

Only S-suffix instructions (ADDS, SUBS, ANDS, BICS) and compare instructions
(CMP, CMN, TST) update NZCV.  The flags are stored as a 4-bit nibble:
  bit 3 = N, bit 2 = Z, bit 1 = C, bit 0 = V
"""

from __future__ import annotations

from dataclasses import dataclass

# ── Architecture constants ─────────────────────────────────────────────────────

MASK32: int = 0xFFFF_FFFF                   # 32-bit unsigned mask
MASK64: int = 0xFFFF_FFFF_FFFF_FFFF        # 64-bit unsigned mask
SIGN32: int = 0x8000_0000                   # sign bit of a 32-bit quantity
SIGN64: int = 0x8000_0000_0000_0000         # sign bit of a 64-bit quantity
MEM_SIZE: int = 65_536                      # 64 KiB flat byte-addressed memory
NUM_GPRS: int = 32                          # X0–X30, XZR (index 31 = always 0)
XZR: int = 31                               # XZR register index

# NZCV flag bit weights within the 4-bit nzcv nibble.
NZCV_N: int = 0b1000    # Negative
NZCV_Z: int = 0b0100    # Zero
NZCV_C: int = 0b0010    # Carry
NZCV_V: int = 0b0001    # Overflow


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
class AArch64State:
    """
    Complete, immutable snapshot of the AArch64 simulator at one moment.

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
    memory  65 536 bytes stored as a tuple of Python ints (each 0–255).
    halted  True once the simulator fetches the all-zero HALT word (0x00000000,
            which is UDF #0 — permanently undefined in AArch64; used here as a
            simulation sentinel).
    """

    pc: int                         # program counter
    gpr: tuple[int, ...]            # 32 × 64-bit registers (X0–X30, XZR)
    sp: int                         # stack pointer
    nzcv: int                       # condition flags (4-bit)
    memory: tuple[int, ...]         # 65 536 bytes
    halted: bool

    # ── Convenience properties for named registers ────────────────────────────

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


def make_initial_state() -> AArch64State:
    """Return the power-on / reset state: all registers and memory zeroed, PC=0."""
    return AArch64State(
        pc=0,
        gpr=tuple(0 for _ in range(NUM_GPRS)),
        sp=0,
        nzcv=0,
        memory=tuple(0 for _ in range(MEM_SIZE)),
        halted=False,
    )
