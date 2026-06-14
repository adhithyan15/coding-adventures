"""Register file for the MOS 6502 gate-level simulator.

=== The 6502 Register Architecture ===

The MOS 6502 (1975) has a famously tiny register file compared to its
contemporaries.  This was a deliberate design choice: Motorola, in the
MOS team's previous employer, was building register-rich chips like the
6800.  The 6502 bet on zero-page memory as "cheap registers" instead.

Active registers:
  A   — 8-bit accumulator (arithmetic / logical results always go here)
  X   — 8-bit index register (address offsets, loop counters)
  Y   — 8-bit index register (address offsets)
  S   — 8-bit stack pointer (effective address = 0x0100 + S)
  PC  — 16-bit program counter

Processor status (P register — not a "data" register, but part of state):
  7 active flag bits: N V - B D I Z C  (bit 5 is always 1, no flip-flop)

=== Gate cost per register ===

Each 8-bit register: 8 D flip-flops.
  Real NMOS: ~2 transistors per flip-flop stage × 6 stages = ~12–16 per bit
  Total for 8-bit register: ~96–128 transistors

Register summary for 6502:
  A, X, Y, S (4 × 8-bit):  ~384–512 transistors
  PC          (1 × 16-bit): ~192–256 transistors
  Flags       (7 bits):     ~84–112 transistors
  Total:                    ~660–880 transistors (out of ~3,510 total)

=== Implementation model ===

We use the same D flip-flop simulation as z80_gatelevel:
Store register state as list of ints (0 or 1).  The `register()` function
from `logic_gates` models a D flip-flop array behaviorally.

Flags are stored as individual int bits (0 or 1), each modelling a single
flip-flop.  This matches the real 6502 where each flag has its own
flip-flop in the processor status register.

=== P register packing ===

The P byte is assembled from individual flag flip-flops whenever the
processor needs to push P or return it via instructions like PHP, BRK.
The bit layout:

    Bit 7  N  (flag_n flip-flop)
    Bit 6  V  (flag_v flip-flop)
    Bit 5  -  1 (always — this bit has no physical flip-flop in NMOS 6502)
    Bit 4  B  (flag_b — "software" indicator, set during push for BRK/PHP)
    Bit 3  D  (flag_d flip-flop)
    Bit 2  I  (flag_i flip-flop)
    Bit 1  Z  (flag_z flip-flop)
    Bit 0  C  (flag_c flip-flop)
"""

from __future__ import annotations

from logic_gates import AND, OR, register

from mos6502_gatelevel.bits import (
    add_16bit,
    bits_to_int,
    int_to_bits,
)


class Register8:
    """8-bit register modeled as an array of 8 D flip-flops.

    All 8 flip-flops share the same clock signal.  On write(), the data
    is clocked in.  On read(), the stored bits are returned.

    In the real 6502, registers are latched on the rising edge of the
    internal phi2 clock.  We simulate this by running two register()
    calls: first with clock=0 (master absorbs data), then clock=1
    (slave outputs).

    Usage::

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
            value: 8-bit integer (0–255).  Masked to 8 bits.
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

    Used for the program counter (PC).

    The 6502's PC advances by 1 for most instructions, by 2 for
    two-byte instructions, etc.  The inc() method routes through the
    16-bit ripple-carry adder gate chain rather than Python addition.

    Usage::

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
            value: 16-bit integer (0–65535).  Masked.
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
        """Increment the register by ``amount`` via the 16-bit adder.

        Routes through the ripple_carry_adder gate chain.  Used for
        PC advancement during instruction fetch.

        Args:
            amount: Amount to add (default 1).  Masked to 16 bits.
        """
        new_val, _cout = add_16bit(self._value, amount & 0xFFFF, 0)
        self.write(new_val & 0xFFFF)


class FlagRegister:
    """The 6502 processor status register (P) — 7 individual flag flip-flops.

    Each flag is a separate D flip-flop.  Bit 5 (always 1) has no
    physical flip-flop; it is hardwired to Vcc.

    Flags:
        n — Negative   (bit 7 of P)
        v — Overflow   (bit 6)
        b — Break      (bit 4, only set in pushed copy)
        d — Decimal    (bit 3)
        i — Interrupt  (bit 2)
        z — Zero       (bit 1)
        c — Carry      (bit 0)

    Usage::

        >>> f = FlagRegister()
        >>> f.set_n(1); f.set_c(1)
        >>> f.pack()
        0b10100001  # N=1, bit5=1, C=1
    """

    def __init__(self) -> None:
        """Power-on state: I=1, all others 0.  Bit 5 always 1."""
        self._n: int = 0
        self._v: int = 0
        self._b: int = 0
        self._d: int = 0
        self._i: int = 1   # I=1 at power-on (interrupt disable)
        self._z: int = 0
        self._c: int = 0

    # ── Individual flag setters (each clocks one flip-flop) ──────────────────

    def set_n(self, value: int) -> None:
        """Set the Negative flag (bit 7 of result → N flip-flop)."""
        self._n = AND(OR(value, 0), 1)   # Sanitize to 0/1 via gate

    def set_v(self, value: int) -> None:
        """Set the Overflow flag."""
        self._v = AND(OR(value, 0), 1)

    def set_b(self, value: int) -> None:
        """Set the Break flag (only meaningful in pushed P copies)."""
        self._b = AND(OR(value, 0), 1)

    def set_d(self, value: int) -> None:
        """Set the Decimal mode flag."""
        self._d = AND(OR(value, 0), 1)

    def set_i(self, value: int) -> None:
        """Set the Interrupt disable flag."""
        self._i = AND(OR(value, 0), 1)

    def set_z(self, value: int) -> None:
        """Set the Zero flag."""
        self._z = AND(OR(value, 0), 1)

    def set_c(self, value: int) -> None:
        """Set the Carry flag."""
        self._c = AND(OR(value, 0), 1)

    # ── Individual flag getters ───────────────────────────────────────────────

    def get_n(self) -> int:
        """Read the Negative flag (0 or 1)."""
        return self._n

    def get_v(self) -> int:
        """Read the Overflow flag (0 or 1)."""
        return self._v

    def get_b(self) -> int:
        """Read the Break flag (0 or 1)."""
        return self._b

    def get_d(self) -> int:
        """Read the Decimal mode flag (0 or 1)."""
        return self._d

    def get_i(self) -> int:
        """Read the Interrupt disable flag (0 or 1)."""
        return self._i

    def get_z(self) -> int:
        """Read the Zero flag (0 or 1)."""
        return self._z

    def get_c(self) -> int:
        """Read the Carry flag (0 or 1)."""
        return self._c

    # ── Pack / unpack ─────────────────────────────────────────────────────────

    def pack(self, with_b: int | None = None) -> int:
        """Pack all flags into the P status byte.

        Bit 5 is hardwired to 1 (no flip-flop on real 6502 silicon).

        Args:
            with_b: Override the B bit in the packed result.  When
                    PHP or BRK push P, B=1 is forced.  When IRQ/NMI
                    push P, B=0 is forced.  Pass None to use current
                    self._b value (default).

        Returns:
            8-bit P byte:  N V 1 B D I Z C

        Truth table::

            Bit 7  N  self._n
            Bit 6  V  self._v
            Bit 5  1  (always)
            Bit 4  B  with_b or self._b
            Bit 3  D  self._d
            Bit 2  I  self._i
            Bit 1  Z  self._z
            Bit 0  C  self._c
        """
        b_bit = self._b if with_b is None else (with_b & 1)
        return (
            (self._n << 7)
            | (self._v << 6)
            | 0x20              # bit 5 always 1
            | (b_bit << 4)
            | (self._d << 3)
            | (self._i << 2)
            | (self._z << 1)
            | self._c
        )

    def unpack(self, p: int) -> None:
        """Unpack a P byte into individual flag flip-flops.

        Used by PLP and RTI to restore processor status from stack.
        Bit 5 is ignored (it has no flip-flop to set).

        Args:
            p: 8-bit P status byte.
        """
        self._n = (p >> 7) & 1
        self._v = (p >> 6) & 1
        self._b = (p >> 4) & 1
        self._d = (p >> 3) & 1
        self._i = (p >> 2) & 1
        self._z = (p >> 1) & 1
        self._c = p & 1


class RegisterFile6502:
    """Complete 6502 register file: A, X, Y, S, PC, and flags.

    All registers are modeled as D flip-flop arrays.  This class
    provides a unified interface to all CPU state.

    Usage::

        >>> rf = RegisterFile6502()
        >>> rf.a.write(0x42)
        >>> rf.a.read()
        66
        >>> rf.pc.write(0x0200)
        >>> rf.flags.set_n(1)
        >>> rf.flags.pack()
        0xA4   # N=1, bit5=1, I=1 (default)
    """

    def __init__(self) -> None:
        """Initialize all registers to power-on state."""
        self.a = Register8()     # Accumulator
        self.x = Register8()     # Index X
        self.y = Register8()     # Index Y
        self.s = Register8()     # Stack pointer (power-on: 0xFD)
        self.pc = Register16()   # Program counter
        self.flags = FlagRegister()

        # Power-on state: S = 0xFD
        self.s.write(0xFD)

    def reset(self) -> None:
        """Reset all registers to power-on state.

        A=X=Y=0, S=0xFD, PC=0x0000, flags: I=1 rest 0.
        """
        self.a.write(0)
        self.x.write(0)
        self.y.write(0)
        self.s.write(0xFD)
        self.pc.write(0)
        self.flags._n = 0
        self.flags._v = 0
        self.flags._b = 0
        self.flags._d = 0
        self.flags._i = 1   # I=1: interrupts disabled at power-on
        self.flags._z = 0
        self.flags._c = 0
