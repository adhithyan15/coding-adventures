"""Z80State — immutable snapshot of the Zilog Z80 CPU.

The Z80 has considerably more registers than the Intel 8080 it supersedes:

Main register bank
------------------
A   — accumulator (8-bit)
F   — flags register (8-bit, packed; see f_byte())
B, C — general purpose; BC is a 16-bit pair used as loop counter
D, E — general purpose; DE is used as a destination pointer
H, L — general purpose; HL is used as a general memory pointer

Alternate register bank  (unique to Z80)
-----------------------------------------
A', F', B', C', D', E', H', L'
These shadow the main bank. Swapped in/out via EX AF,AF' and EXX.
Only one bank is live at a time; the other is stored but invisible to
normal instructions.

Special registers
-----------------
IX, IY — 16-bit index registers with signed 8-bit displacement addressing
SP     — 16-bit stack pointer
PC     — 16-bit program counter
I      — 8-bit interrupt vector base (used in IM 2)
R      — 8-bit memory refresh counter (low 7 bits auto-increment)

Flags (F register)
------------------
Bit 7  S   Sign       — bit 7 of result
Bit 6  Z   Zero       — result == 0
Bit 5  Y   (undocumented, copy of result bit 5)
Bit 4  H   Half-carry — carry from bit 3 to 4 (ADD) / borrow (SUB)
Bit 3  X   (undocumented, copy of result bit 3)
Bit 2  P/V Parity (logical ops) / Overflow (arithmetic ops)
Bit 1  N   Add/Subtract — 1 after SUB/SBC/DEC/CP, 0 after ADD/ADC/INC
Bit 0  C   Carry

Interrupt state
---------------
IFF1 — interrupt enable flip-flop (maskable interrupts enabled when True)
IFF2 — shadow of IFF1, saved/restored during NMI
IM   — interrupt mode (0, 1, or 2)
"""

from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True)
class Z80State:
    """Immutable snapshot of the complete Z80 CPU state.

    All integer register fields are in 0–255 (8-bit) or 0–65535 (16-bit).
    The memory field is a 65536-element tuple of byte values (0–255).

    Use f_byte() to get the packed F register value.
    """

    # ── Main register bank ───────────────────────────────────────────────────
    a: int   # Accumulator
    b: int
    c: int
    d: int
    e: int
    h: int
    l: int  # noqa: E741  (L register; ambiguous name is intentional)

    # ── Alternate register bank ──────────────────────────────────────────────
    # a_prime and f_prime are stored as raw values.
    # The other alternate registers mirror the naming convention.
    a_prime: int   # A'
    f_prime: int   # F' (packed byte)
    b_prime: int   # B'
    c_prime: int   # C'
    d_prime: int   # D'
    e_prime: int   # E'
    h_prime: int   # H'
    l_prime: int   # L'

    # ── Index / special registers ────────────────────────────────────────────
    ix: int    # Index register X (16-bit)
    iy: int    # Index register Y (16-bit)
    sp: int    # Stack pointer (16-bit)
    pc: int    # Program counter (16-bit)
    i:  int    # Interrupt vector base (8-bit)
    r:  int    # Memory refresh counter (8-bit; only low 7 bits increment)

    # ── Flags (main bank, unpacked for readability) ──────────────────────────
    flag_s:  bool   # Sign
    flag_z:  bool   # Zero
    flag_h:  bool   # Half-carry
    flag_pv: bool   # Parity / Overflow
    flag_n:  bool   # Add/Subtract
    flag_c:  bool   # Carry

    # ── Interrupt state ──────────────────────────────────────────────────────
    iff1: bool   # Maskable interrupt enable flip-flop
    iff2: bool   # Shadow of IFF1 (preserved across NMI)
    im:   int    # Interrupt mode: 0, 1, or 2

    # ── Simulator state ──────────────────────────────────────────────────────
    halted: bool
    memory: tuple[int, ...]   # 65536 bytes

    # ── Derived helpers ──────────────────────────────────────────────────────

    def f_byte(self) -> int:
        """Pack the main flags into the Z80 F register byte.

        Bit layout::

            7  6  5  4  3  2  1  0
            S  Z  0  H  0  PV N  C

        Bits 5 and 3 (Y and X) are undocumented; we set them to 0 here
        for simplicity. Real Z80 sets them to bits 5 and 3 of the last
        arithmetic result — this detail is rarely tested.
        """
        return (
            (int(self.flag_s)  << 7)
            | (int(self.flag_z)  << 6)
            | (int(self.flag_h)  << 4)
            | (int(self.flag_pv) << 2)
            | (int(self.flag_n)  << 1)
            | int(self.flag_c)
        )

    @property
    def bc(self) -> int:
        """16-bit BC register pair."""
        return (self.b << 8) | self.c

    @property
    def de(self) -> int:
        """16-bit DE register pair."""
        return (self.d << 8) | self.e

    @property
    def hl(self) -> int:
        """16-bit HL register pair."""
        return (self.h << 8) | self.l
