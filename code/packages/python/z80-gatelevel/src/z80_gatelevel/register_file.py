"""Register file for the Z80 gate-level simulator.

=== The Z80 Register Architecture ===

The Zilog Z80 (1976) has considerably more registers than the Intel 8080:

Main register bank (active):
  A     — 8-bit accumulator
  F     — 8-bit flags register (S Z Y H X PV N C)
  B, C  — 8-bit general purpose; BC = 16-bit pair (loop counter/memory)
  D, E  — 8-bit general purpose; DE = 16-bit pair (destination pointer)
  H, L  — 8-bit general purpose; HL = 16-bit pair (general memory pointer)

Alternate register bank (shadow — swapped via EX AF,AF' and EXX):
  A', F', B', C', D', E', H', L'
  Identical in size but only ONE bank is active at any time.
  The real Z80 chip has two complete sets of 8 registers = 128 flip-flops.

Index registers:
  IX, IY — 16-bit index registers with signed 8-bit displacement

Special registers:
  SP — 16-bit stack pointer
  PC — 16-bit program counter
  I  — 8-bit interrupt vector base (for IM 2 interrupt handling)
  R  — 8-bit memory refresh counter (low 7 bits auto-increment each instruction)

Interrupt state:
  IFF1 — interrupt enable flip-flop 1 (maskable interrupt enable)
  IFF2 — interrupt enable flip-flop 2 (shadow of IFF1 for NMI save/restore)
  IM   — 2-bit interrupt mode selector (IM 0 / IM 1 / IM 2)

=== Gate cost per register ===

Each 8-bit register: 8 D flip-flops = ~16 NOR gates = ~64 transistors.
Each 16-bit register: 16 D flip-flops = ~32 NOR gates = ~128 transistors.

Main bank (8 × 8-bit):   8 × 64 = 512 transistors
Alt bank  (8 × 8-bit):   8 × 64 = 512 transistors
IX, IY   (2 × 16-bit):   2 × 128 = 256 transistors
SP, PC   (2 × 16-bit):   2 × 128 = 256 transistors
I, R     (2 × 8-bit):    2 × 64 = 128 transistors
Total:    ~1664 transistors just for registers
(The real Z80 is ~8500 transistors total — registers are ~20% of the chip)

=== Register encoding ===

The Z80 uses a 3-bit field in instructions to select 8-bit registers:
    000 = B    001 = C    010 = D    011 = E
    100 = H    101 = L    110 = (HL) pseudo   111 = A

Register pair codes (2-bit field):
    00 = BC    01 = DE    10 = HL    11 = SP

For PUSH/POP, the mapping changes:
    00 = BC    01 = DE    10 = HL    11 = AF (not SP!)

=== Implementation model ===

We use the same D flip-flop simulation as intel8080_gatelevel:
Store register state as list of ints (0 or 1). The `register()` function
from `logic_gates` models a D flip-flop array behaviorally.
"""

from __future__ import annotations

from logic_gates import register

from z80_gatelevel.bits import (
    add_16bit,
    bits_to_int,
    int_to_bits,
)

# ── 8-bit register codes (3-bit Z80 field) ────────────────────────────────────
REG_B = 0
REG_C = 1
REG_D = 2
REG_E = 3
REG_H = 4
REG_L = 5
REG_MEM = 6   # pseudo: (HL) — raises ValueError if accessed directly
REG_A = 7

# ── 16-bit register pair codes ────────────────────────────────────────────────
PAIR_BC = 0
PAIR_DE = 1
PAIR_HL = 2
PAIR_SP = 3
PAIR_IX = 4
PAIR_IY = 5


class Register8:
    """8-bit register modeled as an array of 8 D flip-flops.

    All 8 flip-flops share the same clock signal. On write(), the data
    is clocked in. On read(), the stored bits are returned.

    In the real Z80, registers are latched on the rising edge of the
    internal φ2 clock. We simulate this by running two `register()` calls:
    first with clock=0 (master absorbs data) then clock=1 (slave outputs).

    Usage:
        >>> r = Register8()
        >>> r.write(0xAB)
        >>> r.read()
        171
    """

    def __init__(self) -> None:
        """Initialize to zero (power-on state: all flip-flops reset)."""
        self._state: list[dict[str, int]] | None = None
        self._value: int = 0

    def write(self, value: int) -> None:
        """Clock a new 8-bit value into the register.

        Args:
            value: 8-bit integer (0–255). Masked to 8 bits.
        """
        value = value & 0xFF
        bits = int_to_bits(value, 8)
        _out_low, state_low = register(bits, 0, self._state, width=8)
        out_bits, self._state = register(bits, 1, state_low, width=8)
        self._value = bits_to_int(out_bits)

    def read(self) -> int:
        """Read the stored 8-bit value.

        Returns:
            8-bit integer (0–255).
        """
        return self._value

    def read_bits(self) -> list[int]:
        """Read the stored value as a list of bits (LSB first).

        Returns:
            List of 8 bits, index 0 = LSB.
        """
        return int_to_bits(self._value, 8)


class Register16:
    """16-bit register modeled as an array of 16 D flip-flops.

    Used for SP, PC, IX, IY.

    The Z80's SP is decremented before pushing (pre-decrement) and
    incremented after popping (post-increment). PC advances by 1, 2, 3, or 4
    depending on instruction length (up to 4 bytes for DDCB/FDCB instructions).

    Usage:
        >>> r = Register16()
        >>> r.write(0x1234)
        >>> r.read()
        4660
    """

    def __init__(self) -> None:
        """Initialize to zero."""
        self._state: list[dict[str, int]] | None = None
        self._value: int = 0

    def write(self, value: int) -> None:
        """Clock a new 16-bit value into the register.

        Args:
            value: 16-bit integer (0–65535). Masked.
        """
        value = value & 0xFFFF
        bits = int_to_bits(value, 16)
        _out_low, state_low = register(bits, 0, self._state, width=16)
        out_bits, self._state = register(bits, 1, state_low, width=16)
        self._value = bits_to_int(out_bits)

    def read(self) -> int:
        """Read the stored 16-bit value.

        Returns:
            16-bit integer (0–65535).
        """
        return self._value

    def inc(self, amount: int = 1) -> None:
        """Increment the register by `amount` via the 16-bit adder.

        Routes through the ripple_carry_adder gate chain.

        Args:
            amount: Amount to add (default 1).
        """
        new_val, _cout, _hc = add_16bit(self._value, amount & 0xFFFF, 0)
        self.write(new_val & 0xFFFF)

    def dec(self, amount: int = 1) -> None:
        """Decrement by `amount` via two's complement subtraction.

        Args:
            amount: Amount to subtract (default 1).
        """
        twos = (~amount + 1) & 0xFFFF
        new_val, _cout, _hc = add_16bit(self._value, twos, 0)
        self.write(new_val & 0xFFFF)


class RegisterFile:
    """Z80 register file: main bank + alternate bank + index registers.

    Contains all Z80 registers stored in flip-flop arrays.

    Main bank:    A, B, C, D, E, H, L, F (the F register stores flags)
    Alternate:    A', B', C', D', E', H', L', F'
    Index:        IX, IY (16-bit)

    Usage:
        >>> rf = RegisterFile()
        >>> rf.write8(REG_A, 42)
        >>> rf.read8(REG_A)
        42
        >>> rf.write16_pair(PAIR_BC, 0x1234)
        >>> rf.read16_pair(PAIR_BC)
        0x1234
        >>> rf.exchange_af()   # EX AF, AF'
        >>> rf.exchange_bank() # EXX (swap BC/DE/HL with B'C'/D'E'/H'L')
    """

    def __init__(self) -> None:
        """Initialize all registers to zero."""
        # Main bank: A and general-purpose registers
        # Index 7 = A, 0-5 = B,C,D,E,H,L; index 6 unused (pseudo-reg)
        self._regs: list[Register8] = [Register8() for _ in range(8)]

        # Alternate bank: A', B', C', D', E', H', L' (stored as int)
        # We use Register8 objects for the alternate bank too
        self._alt: list[Register8] = [Register8() for _ in range(8)]

        # F (flags register) — stored as a separate Register8
        # bit layout: S Z Y H X PV N C
        self._f = Register8()
        self._f_prime = Register8()

        # Index and special 16-bit registers
        self._ix = Register16()
        self._iy = Register16()

    # ── 8-bit register access ─────────────────────────────────────────────────

    def read8(self, reg_id: int) -> int:
        """Read an 8-bit register value by 3-bit code.

        Args:
            reg_id: Register code (REG_A=7, REG_B=0, ..., REG_L=5).
                    REG_MEM (6) is not valid — raises ValueError.

        Returns:
            8-bit integer (0–255).
        """
        if reg_id == REG_MEM:
            msg = "REG_MEM (6) is a pseudo-register — use memory access"
            raise ValueError(msg)
        return self._regs[reg_id].read()

    def write8(self, reg_id: int, value: int) -> None:
        """Write an 8-bit value to register by 3-bit code.

        Args:
            reg_id: Register code (0–7, not 6).
            value:  8-bit integer (0–255). Masked.
        """
        if reg_id == REG_MEM:
            msg = "REG_MEM (6) is a pseudo-register — use memory write"
            raise ValueError(msg)
        self._regs[reg_id].write(value & 0xFF)

    # ── 16-bit register pair access ───────────────────────────────────────────

    def read16_pair(
        self, pair_id: int, sp: Register16 | None = None, pc: Register16 | None = None
    ) -> int:
        """Read a 16-bit register pair value.

        Args:
            pair_id: PAIR_BC=0, PAIR_DE=1, PAIR_HL=2, PAIR_SP=3,
                     PAIR_IX=4, PAIR_IY=5.
            sp:      SP Register16 (required for PAIR_SP=3).
            pc:      PC Register16 (not needed here, for symmetry).

        Returns:
            16-bit integer (0–65535).
        """
        match pair_id:
            case 0:  # BC
                return (self._regs[REG_B].read() << 8) | self._regs[REG_C].read()
            case 1:  # DE
                return (self._regs[REG_D].read() << 8) | self._regs[REG_E].read()
            case 2:  # HL
                return (self._regs[REG_H].read() << 8) | self._regs[REG_L].read()
            case 3:  # SP
                if sp is None:
                    msg = "SP Register16 required for PAIR_SP"
                    raise ValueError(msg)
                return sp.read()
            case 4:  # IX
                return self._ix.read()
            case 5:  # IY
                return self._iy.read()
            case _:
                msg = f"Invalid pair_id: {pair_id}"
                raise ValueError(msg)

    def write16_pair(
        self,
        pair_id: int,
        value: int,
        sp: Register16 | None = None,
    ) -> None:
        """Write a 16-bit value to a register pair.

        High byte → first register, low byte → second register.

        Args:
            pair_id: PAIR_BC/DE/HL/SP/IX/IY.
            value:   16-bit integer (0–65535). Masked.
            sp:      SP Register16 (required for PAIR_SP).
        """
        value = value & 0xFFFF
        hi = (value >> 8) & 0xFF
        lo = value & 0xFF

        match pair_id:
            case 0:
                self._regs[REG_B].write(hi)
                self._regs[REG_C].write(lo)
            case 1:
                self._regs[REG_D].write(hi)
                self._regs[REG_E].write(lo)
            case 2:
                self._regs[REG_H].write(hi)
                self._regs[REG_L].write(lo)
            case 3:
                if sp is None:
                    msg = "SP Register16 required for PAIR_SP"
                    raise ValueError(msg)
                sp.write(value)
            case 4:
                self._ix.write(value)
            case 5:
                self._iy.write(value)
            case _:
                msg = f"Invalid pair_id: {pair_id}"
                raise ValueError(msg)

    # ── Flags access ──────────────────────────────────────────────────────────

    def read_flags(self) -> dict[str, int]:
        """Read all flags from the F register as a dict.

        Returns:
            Dict with keys 's', 'z', 'h', 'pv', 'n', 'c' (each 0 or 1).
        """
        f_byte = self._f.read()
        return {
            's':  (f_byte >> 7) & 1,
            'z':  (f_byte >> 6) & 1,
            'h':  (f_byte >> 4) & 1,
            'pv': (f_byte >> 2) & 1,
            'n':  (f_byte >> 1) & 1,
            'c':  f_byte & 1,
        }

    def write_flags(self, s: int, z: int, h: int, pv: int, n: int, c: int) -> None:
        """Write all flags to the F register.

        Args:
            s, z, h, pv, n, c: Each 0 or 1.
        """
        f_byte = pack_f(s, z, h, pv, n, c)
        self._f.write(f_byte)

    def pack_f(self) -> int:
        """Return the packed F register byte."""
        return self._f.read()

    def unpack_f_into_flags(self, byte: int) -> None:
        """Unpack an F register byte into the flag flip-flops.

        Args:
            byte: F register byte (0–255).
        """
        self._f.write(byte & 0xFF)

    # ── Bank exchange operations ───────────────────────────────────────────────

    def exchange_af(self) -> None:
        """EX AF, AF' — swap main A,F with alternate A',F'.

        This is a single Z80 instruction (opcode 0x08). It exchanges the
        content of the flip-flop arrays for A and F with their alternates.

        In hardware, this is done by toggling a bank-select flip-flop that
        routes all bus connections to either the main or alternate bank.
        In our simulation, we read both values and write them cross.
        """
        a_main = self._regs[REG_A].read()
        a_alt = self._alt[REG_A].read()
        f_main = self._f.read()
        f_alt = self._f_prime.read()

        self._regs[REG_A].write(a_alt)
        self._alt[REG_A].write(a_main)
        self._f.write(f_alt)
        self._f_prime.write(f_main)

    def exchange_bank(self) -> None:
        """EXX — swap BC, DE, HL with B'C', D'E', H'L'.

        Z80 opcode 0xD9. Toggles the general-purpose register bank.
        All three pairs (BC, DE, HL) are exchanged simultaneously with
        their alternates. AF is NOT affected.
        """
        for reg_id in (REG_B, REG_C, REG_D, REG_E, REG_H, REG_L):
            main_val = self._regs[reg_id].read()
            alt_val = self._alt[reg_id].read()
            self._regs[reg_id].write(alt_val)
            self._alt[reg_id].write(main_val)

    # ── Alternate register access (for get_state) ─────────────────────────────

    def read_alt8(self, reg_id: int) -> int:
        """Read an alternate register by code.

        Only valid for B'(0), C'(1), D'(2), E'(3), H'(4), L'(5), A'(7).
        """
        return self._alt[reg_id].read()

    def write_alt8(self, reg_id: int, value: int) -> None:
        """Write an alternate register by code."""
        self._alt[reg_id].write(value & 0xFF)

    def read_f_prime(self) -> int:
        """Read the alternate F' register byte."""
        return self._f_prime.read()

    def write_f_prime(self, value: int) -> None:
        """Write the alternate F' register byte."""
        self._f_prime.write(value & 0xFF)

    def read_ix(self) -> int:
        """Read IX."""
        return self._ix.read()

    def write_ix(self, value: int) -> None:
        """Write IX."""
        self._ix.write(value & 0xFFFF)

    def read_iy(self) -> int:
        """Read IY."""
        return self._iy.read()

    def write_iy(self, value: int) -> None:
        """Write IY."""
        self._iy.write(value & 0xFFFF)


def pack_f(s: int, z: int, h: int, pv: int, n: int, c: int) -> int:
    """Pack Z80 flag bits into the F register byte.

    F register bit layout::

        7  6  5  4  3  2  1  0
        S  Z  0  H  0  PV N  C

    Bits 5 and 3 (Y and X) are undocumented "copy of result" bits.
    We set them to 0 here for simplicity.

    Args:
        s, z, h, pv, n, c: Each 0 or 1.

    Returns:
        8-bit F register byte.

    Examples:
        >>> pack_f(1, 0, 0, 0, 0, 1)  # S=1, C=1
        0x81
        >>> pack_f(0, 1, 0, 0, 0, 0)  # Z=1
        0x40
    """
    return (
        ((s & 1) << 7)
        | ((z & 1) << 6)
        | ((h & 1) << 4)
        | ((pv & 1) << 2)
        | ((n & 1) << 1)
        | (c & 1)
    )


def unpack_f(byte: int) -> tuple[int, int, int, int, int, int]:
    """Unpack an F register byte into individual flag bits.

    Args:
        byte: F register byte (0–255).

    Returns:
        Tuple (s, z, h, pv, n, c) — each 0 or 1.

    Examples:
        >>> unpack_f(0x81)  # S=1, C=1
        (1, 0, 0, 0, 0, 1)
        >>> unpack_f(0xFF)  # all flags set
        (1, 1, 1, 1, 1, 1)
    """
    s  = (byte >> 7) & 1
    z  = (byte >> 6) & 1
    h  = (byte >> 4) & 1
    pv = (byte >> 2) & 1
    n  = (byte >> 1) & 1
    c  = byte & 1
    return s, z, h, pv, n, c
