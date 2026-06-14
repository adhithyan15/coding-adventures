"""Register file for the Motorola 68000 gate-level simulator.

=== Physical structure ===

On the 68000 silicon, registers are implemented as arrays of flip-flops —
one flip-flop per bit.  This module represents each register as a list of
integer bits (0 or 1), LSB at index 0, matching the logic-gates convention.

=== Register widths ===

All data and address registers are 32 bits wide.

Data registers (D0–D7):
  - Byte ops: write to bits 7–0; bits 31–8 unchanged.
  - Word ops: write to bits 15–0; bits 31–16 unchanged.
  - Long ops: write all 32 bits.

Address registers (A0–A7):
  - No byte access (MOVEA always word or long).
  - Word write is sign-extended to 32 bits before storing.
  - Long write stores all 32 bits.

Program Counter (PC):
  - 32-bit internally; only bits 23–0 are used (24-bit bus).

Status Register (SR):
  - 16-bit register.
  - High byte (system byte): T1, T0, S, M, 0, I2, I1, I0.
  - Low byte (CCR): 0, 0, 0, X, N, Z, V, C.
  - In this simulator S is always 1 (supervisor mode).

=== Pack/unpack of SR ===

The SR is stored as individual flag bits for fast access during execution.
The pack_sr() / unpack_sr() methods convert between the bit-field
representation and the 16-bit integer that the protocol and external code use.

    SR bit layout:
      15–14: T1, T0  (trace; always 0 in this sim)
      13:    S       (supervisor; always 1)
      12:    M       (master; always 0)
      11:    0
      10–8:  I2 I1 I0  (interrupt mask)
      7–5:   0
      4:     X
      3:     N
      2:     Z
      1:     V
      0:     C
"""

from __future__ import annotations

from motorola68k_gatelevel.bits import bits_to_int, int_to_bits

_LONG_MASK = 0xFFFF_FFFF
_WORD_MASK = 0x0000_FFFF
_BYTE_MASK = 0x0000_00FF


class RegisterFile68k:
    """All CPU registers for the Motorola 68000, stored as bit arrays.

    Internal representation uses LSB-first bit lists for all registers.
    External interface (read_*/write_*) converts to/from plain integers.

    Examples:
        >>> rf = RegisterFile68k()
        >>> rf.write_dn(0, 0xDEADBEEF, 4)
        >>> hex(rf.read_dn(0, 4))
        '0xdeadbeef'
        >>> rf.write_dn(0, 0x42, 1)   # byte write: preserves upper bits
        >>> hex(rf.read_dn(0, 4))
        '0xdeadbe42'
    """

    def __init__(self) -> None:
        """Initialize all registers to zero, except A7=0x00F000 and SR=0x2700."""
        # Data registers D0–D7: 32 bits each, LSB-first
        self._d: list[list[int]] = [int_to_bits(0, 32) for _ in range(8)]
        # Address registers A0–A7: 32 bits each
        self._a: list[list[int]] = [int_to_bits(0, 32) for _ in range(8)]
        self._a[7] = int_to_bits(0x00F000, 32)  # A7 = initial stack pointer
        # Program counter
        self._pc: list[int] = int_to_bits(0x001000, 32)  # default load addr
        # Condition code flags (individual bits)
        self._flag_c: int = 0   # Carry
        self._flag_v: int = 0   # Overflow
        self._flag_z: int = 0   # Zero
        self._flag_n: int = 0   # Negative
        self._flag_x: int = 0   # Extend
        self._flag_s: int = 1   # Supervisor (always 1)
        # Interrupt mask (3 bits: I2 I1 I0); 7 = block all interrupts
        self._int_mask: int = 7

    # ── Data register access ──────────────────────────────────────────────────

    def read_dn(self, n: int, size: int) -> int:
        """Read size bytes from the low bits of data register Dn.

        Args:
            n:    Register number 0–7.
            size: 1 (byte), 2 (word), or 4 (long).

        Returns:
            Unsigned integer, masked to the appropriate width.

        Examples:
            >>> rf = RegisterFile68k()
            >>> rf.write_dn(3, 0xABCD1234, 4)
            >>> rf.read_dn(3, 1)   # low byte
            52
            >>> rf.read_dn(3, 2)   # low word
            4660
            >>> rf.read_dn(3, 4)   # full long
            2882343476
        """
        val = bits_to_int(self._d[n])
        if size == 1:
            return val & _BYTE_MASK
        if size == 2:
            return val & _WORD_MASK
        return val & _LONG_MASK

    def write_dn(self, n: int, value: int, size: int) -> None:
        """Write size bytes into Dn; upper bytes of Dn are preserved.

        Byte write: updates bits 7–0 only.
        Word write: updates bits 15–0 only.
        Long write: replaces all 32 bits.

        Args:
            n:     Register number 0–7.
            value: Value to write (masked to the appropriate width).
            size:  1, 2, or 4.

        Examples:
            >>> rf = RegisterFile68k()
            >>> rf.write_dn(0, 0xFFFFFFFF, 4)
            >>> rf.write_dn(0, 0x42, 1)   # byte write
            >>> hex(rf.read_dn(0, 4))
            '0xffffff42'
        """
        current = bits_to_int(self._d[n])
        if size == 1:
            current = (current & 0xFFFFFF00) | (value & _BYTE_MASK)
        elif size == 2:
            current = (current & 0xFFFF0000) | (value & _WORD_MASK)
        else:
            current = value & _LONG_MASK
        self._d[n] = int_to_bits(current, 32)

    # ── Address register access ───────────────────────────────────────────────

    def read_an(self, n: int) -> int:
        """Read full 32-bit value from address register An.

        Address registers always provide their full 32-bit value regardless
        of the instruction size.

        Args:
            n: Register number 0–7.

        Returns:
            32-bit unsigned integer.
        """
        return bits_to_int(self._a[n]) & _LONG_MASK

    def write_an(self, n: int, value: int) -> None:
        """Write 32-bit value to address register An.

        Word writes (e.g. MOVEA.W) sign-extend to 32 bits before storing
        — the caller is responsible for sign-extension before calling this.

        Args:
            n:     Register number 0–7.
            value: 32-bit value to store.
        """
        self._a[n] = int_to_bits(value & _LONG_MASK, 32)

    # ── Program counter ───────────────────────────────────────────────────────

    def read_pc(self) -> int:
        """Read current program counter (24-bit address space).

        Returns:
            32-bit unsigned integer; only bits 23–0 are meaningful.
        """
        return bits_to_int(self._pc) & _LONG_MASK

    def write_pc(self, value: int) -> None:
        """Write program counter.

        Args:
            value: New PC value; stored as 32-bit.
        """
        self._pc = int_to_bits(value & _LONG_MASK, 32)

    # ── Status register (SR) pack / unpack ───────────────────────────────────

    def pack_ccr(self) -> int:
        """Pack the low 5 CCR bits into a single integer.

        CCR bit layout: [X N Z V C] = bits [4 3 2 1 0].

        Returns:
            8-bit integer (0–31 meaningful).

        Examples:
            >>> rf = RegisterFile68k()
            >>> rf._flag_z = 1
            >>> rf.pack_ccr()
            4
        """
        return (
            (self._flag_x << 4)
            | (self._flag_n << 3)
            | (self._flag_z << 2)
            | (self._flag_v << 1)
            | self._flag_c
        )

    def unpack_ccr(self, ccr: int) -> None:
        """Unpack an 8-bit CCR value into individual flag bits.

        Only bits 4–0 are used; bits 7–5 are ignored.

        Args:
            ccr: Condition code register value.
        """
        self._flag_x = (ccr >> 4) & 1
        self._flag_n = (ccr >> 3) & 1
        self._flag_z = (ccr >> 2) & 1
        self._flag_v = (ccr >> 1) & 1
        self._flag_c = ccr & 1

    def pack_sr(self) -> int:
        """Pack the full 16-bit status register.

        SR layout:
          bit 15–14: T1,T0 (always 0)
          bit 13:    S = 1 (supervisor, always set)
          bit 12:    M = 0
          bit 11:    0
          bits 10–8: I2 I1 I0 (interrupt mask)
          bits 7–5:  0
          bit 4:     X
          bit 3:     N
          bit 2:     Z
          bit 1:     V
          bit 0:     C

        Returns:
            16-bit unsigned integer.

        Examples:
            >>> rf = RegisterFile68k()
            >>> sr = rf.pack_sr()
            >>> sr & 0x2000  # supervisor bit
            8192
            >>> (sr >> 8) & 0x7  # interrupt mask = 7
            7
        """
        return (
            (1 << 13)                    # S bit always 1
            | (self._int_mask << 8)
            | self.pack_ccr()
        )

    def unpack_sr(self, sr: int) -> None:
        """Unpack a 16-bit status register value.

        Note: The S bit (bit 13) is forced to 1 regardless of the input
        value — this simulator always runs in supervisor mode.

        Args:
            sr: 16-bit status register value.
        """
        self._int_mask = (sr >> 8) & 0x7
        self._flag_s = 1  # always supervisor
        self.unpack_ccr(sr & 0x1F)

    def reset(self) -> None:
        """Reset all registers to power-on defaults.

        D0–D7 → 0.  A0–A6 → 0.  A7 → 0x00F000.
        PC → 0x001000.  SR → 0x2700 (supervisor, IMask=7, all CCR=0).
        """
        for i in range(8):
            self._d[i] = int_to_bits(0, 32)
            self._a[i] = int_to_bits(0, 32)
        self._a[7] = int_to_bits(0x00F000, 32)
        self._pc = int_to_bits(0x001000, 32)
        self._flag_c = 0
        self._flag_v = 0
        self._flag_z = 0
        self._flag_n = 0
        self._flag_x = 0
        self._flag_s = 1
        self._int_mask = 7
