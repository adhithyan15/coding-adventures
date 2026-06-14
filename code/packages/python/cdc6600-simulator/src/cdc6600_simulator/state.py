"""
CDC 6600 (1964) State Dataclass
================================

The CDC 6600 has three distinct register types — a forward-looking design
that anticipated the "load/store ISA" philosophy decades before RISC:

  X0–X7   60-bit operand registers   (floating-point / integer data)
  A0–A7   18-bit address registers   (memory addresses)
  B0–B7   18-bit index registers     (loop counters / increments, B0 == 0)

Memory is an array of 60-bit words; all arithmetic fits in Python's
arbitrary-precision integers, masked to the appropriate width.
"""

from __future__ import annotations

from dataclasses import dataclass

# ── Bit-width constants ────────────────────────────────────────────────────────

MASK60: int = (1 << 60) - 1   # 60-bit unsigned mask: 0xFFF_FFFF_FFFF_FFFF
MASK18: int = (1 << 18) - 1   # 18-bit unsigned mask: 0x3_FFFF
SIGN60: int = 1 << 59          # sign bit of a 60-bit quantity
SIGN18: int = 1 << 17          # sign bit of an 18-bit quantity

MEMORY_WORDS: int = 4096       # 4 096 × 60-bit words (4 096 × 60 bits = ~30 KB)
NREGS: int = 8                 # 8 registers in each bank (X, A, B)


def sext60(v: int) -> int:
    """Sign-extend a 60-bit masked value to a Python signed int."""
    v = v & MASK60
    if v & SIGN60:
        return v - (1 << 60)
    return v


def sext18(v: int) -> int:
    """Sign-extend an 18-bit masked value to a Python signed int."""
    v = v & MASK18
    if v & SIGN18:
        return v - (1 << 18)
    return v


# ── Immutable state snapshot ───────────────────────────────────────────────────


@dataclass(frozen=True)
class CDC6600State:
    """
    Complete, immutable snapshot of the CDC 6600 simulator at one moment in time.

    Fields
    ------
    p       Parcel address — word_index × 4 + parcel_index (0–3).
            Analogous to the Program Counter on other architectures.
    x       Eight 60-bit operand registers X0–X7.
    a       Eight 18-bit address registers A0–A7.
    b       Eight 18-bit index/increment registers B0–B7.
            B0 is always 0; attempts to write it are silently discarded.
    memory  4096 sixty-bit words stored as a tuple of Python ints.
    halted  True once a HALT (zero parcel) has been executed.
    """

    p: int                    # parcel address
    x: tuple[int, ...]        # 8 × 60-bit
    a: tuple[int, ...]        # 8 × 18-bit
    b: tuple[int, ...]        # 8 × 18-bit (b[0] always 0)
    memory: tuple[int, ...]   # 4096 × 60-bit words
    halted: bool

    # ── Convenience properties for individual registers ────────────────────────

    @property
    def x0(self) -> int: return self.x[0]

    @property
    def x1(self) -> int: return self.x[1]

    @property
    def x2(self) -> int: return self.x[2]

    @property
    def x3(self) -> int: return self.x[3]

    @property
    def x4(self) -> int: return self.x[4]

    @property
    def x5(self) -> int: return self.x[5]

    @property
    def x6(self) -> int: return self.x[6]

    @property
    def x7(self) -> int: return self.x[7]

    @property
    def a0(self) -> int: return self.a[0]

    @property
    def a1(self) -> int: return self.a[1]

    @property
    def a2(self) -> int: return self.a[2]

    @property
    def a3(self) -> int: return self.a[3]

    @property
    def a4(self) -> int: return self.a[4]

    @property
    def a5(self) -> int: return self.a[5]

    @property
    def a6(self) -> int: return self.a[6]

    @property
    def a7(self) -> int: return self.a[7]

    @property
    def b0(self) -> int: return 0  # always zero

    @property
    def b1(self) -> int: return self.b[1]

    @property
    def b2(self) -> int: return self.b[2]

    @property
    def b3(self) -> int: return self.b[3]

    @property
    def b4(self) -> int: return self.b[4]

    @property
    def b5(self) -> int: return self.b[5]

    @property
    def b6(self) -> int: return self.b[6]

    @property
    def b7(self) -> int: return self.b[7]


def make_initial_state() -> CDC6600State:
    """Return the power-on / reset state: all registers and memory zeroed, P=0."""
    return CDC6600State(
        p=0,
        x=tuple(0 for _ in range(NREGS)),
        a=tuple(0 for _ in range(NREGS)),
        b=tuple(0 for _ in range(NREGS)),
        memory=tuple(0 for _ in range(MEMORY_WORDS)),
        halted=False,
    )
