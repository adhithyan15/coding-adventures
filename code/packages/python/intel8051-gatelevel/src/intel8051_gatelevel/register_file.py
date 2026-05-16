"""register_file.py — Gate-level register file for the Intel 8051.

Simulates the 8051's internal RAM (including SFRs) as a 2D array of bit
values, with the program counter (PC) stored as a separate 16-bit bit array.

Why store as bit arrays?
-------------------------
On real silicon, each register is a row of D flip-flops — one per bit.
The flip-flop's Q output is the stored bit; writing clocks a new value in.
We represent this as list[list[int]]: 256 bytes × 8 bits/byte.

PC is stored separately as list[int] (16 bits) because on the 8051 the PC
is NOT memory-mapped — it lives in dedicated flip-flops separate from IRAM.

Bit-addressable area
--------------------
The 8051 has a unique feature: individual bits in IRAM[0x20..0x2F] and in
certain SFRs can be read, set, cleared, or tested by single instructions.
This maps 128 bit addresses (0x00..0x7F) to the lower RAM area, and another
128 (0x80..0xFF) to bit-addressable SFRs.

Bit address decoding:
  0x00-0x7F → IRAM byte = 0x20 + (bit_addr >> 3),  bit position = bit_addr & 7
  0x80-0xFF → IRAM byte = (bit_addr & 0xF8),         bit position = bit_addr & 7

"""

from __future__ import annotations

from .bits import add_16bit, bits_to_int, int_to_bits


class RegisterFile8051:
    """256-byte IRAM + 16-bit PC, stored as bit arrays (flip-flop simulation).

    The internal RAM (IRAM) holds:
        0x00-0x1F: 4 register banks (R0-R7 per bank), selected via PSW.RS1:RS0
        0x20-0x2F: bit-addressable area (128 individual bits)
        0x30-0x7F: general scratchpad RAM
        0x80-0xFF: Special Function Registers (SFRs)

    All SFRs (including ACC at 0xE0, B at 0xF0, PSW at 0xD0, SP at 0x81,
    DPL at 0x82, DPH at 0x83) live in the same 256-byte array, matching
    the real 8051's unified addressing scheme.
    """

    IRAM_SIZE = 256

    def __init__(self) -> None:
        # IRAM: 256 rows × 8 columns, each element is 0 or 1 (flip-flop state)
        # Initialized to all-zero at power-on (undefined on real hardware,
        # zeroed here for determinism)
        self._iram: list[list[int]] = [[0] * 8 for _ in range(self.IRAM_SIZE)]

        # PC: 16 flip-flops, stored LSB-first (bit 0 = address bit 0)
        self._pc: list[int] = [0] * 16

    # ── IRAM read / write ─────────────────────────────────────────────────────

    def read_iram8(self, addr: int) -> int:
        """Read one byte from IRAM at the given address.

        The flip-flop array is converted back to an integer using the
        bits_to_int bridge function.

        Args:
            addr: Byte address, 0-255.

        Returns:
            Integer value of the byte (0-255).
        """
        addr &= 0xFF
        return bits_to_int(self._iram[addr])

    def write_iram8(self, addr: int, value: int) -> None:
        """Write one byte to IRAM at the given address.

        The integer is split into individual bits (int_to_bits) and each
        bit is stored in the corresponding flip-flop cell.

        Args:
            addr:  Byte address, 0-255.
            value: Value to store (0-255; higher bits are masked off).
        """
        addr &= 0xFF
        self._iram[addr] = int_to_bits(value & 0xFF, 8)

    # ── PC read / write / increment ───────────────────────────────────────────

    def read_pc(self) -> int:
        """Read the 16-bit program counter as an integer."""
        return bits_to_int(self._pc)

    def write_pc(self, value: int) -> None:
        """Write a 16-bit value to the program counter.

        Args:
            value: New PC value, 0-65535.
        """
        self._pc = int_to_bits(value & 0xFFFF, 16)

    def increment_pc(self, by: int = 1) -> None:
        """Increment the program counter by `by` using a gate-level 16-bit adder.

        This routes through add_16bit which calls ripple_carry_adder (16
        full adders in series).  The result wraps at 65535 → 0.

        Args:
            by: Amount to add to PC (typically 1, 2, or 3 depending on
                instruction length).
        """
        current = bits_to_int(self._pc)
        new_pc, _ = add_16bit(current, by, 0)
        self._pc = int_to_bits(new_pc & 0xFFFF, 16)

    # ── Bit-addressable read / write ─────────────────────────────────────────

    def _resolve_bit_addr(self, bit_addr: int) -> tuple[int, int]:
        """Resolve a bit address to (byte_addr, bit_position).

        The 8051 bit address space maps to two regions:
          - 0x00-0x7F: bit-addressable lower RAM (bytes 0x20-0x2F)
          - 0x80-0xFF: bit-addressable SFRs (bytes at multiples of 8 in SFR space)

        Args:
            bit_addr: 0-255 bit address.

        Returns:
            (byte_addr, bit_pos) where byte_addr indexes _iram and
            bit_pos is 0-7 (0 = LSB).
        """
        bit_addr &= 0xFF
        if bit_addr < 0x80:
            # Lower RAM bit area: byte 0x20 + (bit_addr >> 3), bit = bit_addr & 7
            byte_addr = 0x20 + (bit_addr >> 3)
            bit_pos = bit_addr & 0x7
        else:
            # SFR bit area: byte = bit_addr & 0xF8, bit = bit_addr & 7
            # e.g., PSW bits 0xD0-0xD7 map to byte 0xD0, positions 0-7
            byte_addr = bit_addr & 0xF8
            bit_pos = bit_addr & 0x7
        return byte_addr, bit_pos

    def read_bit(self, bit_addr: int) -> int:
        """Read one bit from the bit-addressable space.

        Args:
            bit_addr: Bit address (0-255).

        Returns:
            0 or 1.
        """
        byte_addr, bit_pos = self._resolve_bit_addr(bit_addr)
        return self._iram[byte_addr][bit_pos]

    def write_bit(self, bit_addr: int, value: int) -> None:
        """Write one bit to the bit-addressable space.

        Each bit is stored in a dedicated flip-flop cell (_iram[byte][bit_pos]).
        Only that single cell is updated; the other 7 bits in the byte are
        untouched.

        Args:
            bit_addr: Bit address (0-255).
            value:    New bit value, 0 or 1.
        """
        byte_addr, bit_pos = self._resolve_bit_addr(bit_addr)
        self._iram[byte_addr][bit_pos] = value & 1

    # ── Bulk operations (for initialization/snapshot) ─────────────────────────

    def load_iram(self, data: bytes | bytearray) -> None:
        """Load a bytes-like object into IRAM.

        Used during reset to install SFR initial values.

        Args:
            data: Up to 256 bytes; extra bytes are ignored.
        """
        for i, byte in enumerate(data[:self.IRAM_SIZE]):
            self._iram[i] = int_to_bits(byte & 0xFF, 8)

    def dump_iram(self) -> bytearray:
        """Return the current IRAM contents as a bytearray."""
        return bytearray(bits_to_int(row) for row in self._iram)
