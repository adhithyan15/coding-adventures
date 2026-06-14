"""Register file for the Intel 8086 gate-level simulator.

=== The 8086 Register Architecture ===

The Intel 8086 (1978) has a richer register file than its 8-bit predecessors,
reflecting its role as a 16-bit processor:

General-purpose (16-bit, each with byte-accessible halves):
  AX (AH:AL)  — Accumulator.  MUL/DIV result.  Implicit in string/BCD ops.
  BX (BH:BL)  — Base.  Memory addressing base register.
  CX (CH:CL)  — Counter.  LOOP, REP prefix, shift counts.
  DX (DH:DL)  — Data.  High word of 32-bit MUL/DIV.  I/O port address.

Index / pointer (16-bit only):
  SI  — Source Index.  LODS/MOVS/CMPS source pointer (DS segment default).
  DI  — Destination Index.  STOS/MOVS/CMPS destination (ES segment).
  SP  — Stack Pointer.  Points to top of stack in SS segment.
  BP  — Base Pointer.  Stack-frame base; defaults to SS segment.

Segment registers (16-bit):
  CS  — Code Segment.  Physical instruction fetch = CS×16 + IP.
  DS  — Data Segment.  Default for most memory references.
  SS  — Stack Segment.  PUSH/POP and BP-relative accesses.
  ES  — Extra Segment.  Destination for string operations.

Instruction pointer:
  IP  — 16-bit offset within CS.

FLAGS register:
  Bit 11: OF  Bit 10: DF  Bit 9: IF  Bit 8: TF
  Bit  7: SF  Bit  6: ZF  Bit 4: AF  Bit 2: PF  Bit 0: CF
  Bit  1: always 1

=== Gate cost per register ===

Each 16-bit register: 16 D flip-flops.
  Real 8086: ~4–8 transistors per flip-flop stage = ~64–128 per register.
  13 registers (not counting FLAGS): ~832–1664 transistors.
  FLAGS (9 bits): ~72–144 transistors.
  Total register file estimate: ~900–1800 out of ~29,000 total.

=== Implementation model ===

Registers stored as 16-element bit lists (LSB-first).  This matches the
ripple_carry_adder convention in the arithmetic package.

The physical_address() method computes seg × 16 + offset using the
add_20bit() gate-level function.  The "seg × 16" step is equivalent to
left-shifting the 16-bit segment by 4 bits — implemented as bit rewiring
(prepend 4 zeros, giving a 20-bit value).
"""

from __future__ import annotations

from logic_gates import AND, OR

from intel8086_gatelevel.bits import (
    add_20bit,
    bits_to_int,
    int_to_bits,
)


class RegisterFile8086:
    """Complete Intel 8086 register file.

    All 16-bit registers are stored as 16-element bit lists (LSB first).
    FLAG bits are stored as individual integers (0 or 1).

    The physical_address() method routes segment addressing through the
    gate-level add_20bit() function.

    Usage::

        >>> rf = RegisterFile8086()
        >>> rf.write16("ax", 0x1234)
        >>> rf.read16("ax")
        4660
        >>> rf.read8_low("ax")   # AL
        52
        >>> rf.read8_high("ax")  # AH
        18
    """

    def __init__(self) -> None:
        """Initialize all registers to zero (power-on state)."""
        # General-purpose registers (16-bit bit arrays, LSB first)
        self._ax: list[int] = [0] * 16
        self._bx: list[int] = [0] * 16
        self._cx: list[int] = [0] * 16
        self._dx: list[int] = [0] * 16
        # Index / pointer registers
        self._si: list[int] = [0] * 16
        self._di: list[int] = [0] * 16
        self._sp: list[int] = [0] * 16
        self._bp: list[int] = [0] * 16
        # Segment registers
        self._cs: list[int] = [0] * 16
        self._ds: list[int] = [0] * 16
        self._ss: list[int] = [0] * 16
        self._es: list[int] = [0] * 16
        # Instruction pointer
        self._ip: list[int] = [0] * 16

        # FLAGS — individual flip-flops
        self._flag_cf: int = 0   # carry
        self._flag_pf: int = 0   # parity
        self._flag_af: int = 0   # auxiliary carry
        self._flag_zf: int = 0   # zero
        self._flag_sf: int = 0   # sign
        self._flag_tf: int = 0   # trap
        self._flag_if: int = 0   # interrupt enable
        self._flag_df: int = 0   # direction
        self._flag_of: int = 0   # overflow

    # ── Register name → bit array mapping ────────────────────────────────────

    _REG16_MAP = {
        "ax": "_ax", "bx": "_bx", "cx": "_cx", "dx": "_dx",
        "si": "_si", "di": "_di", "sp": "_sp", "bp": "_bp",
        "cs": "_cs", "ds": "_ds", "ss": "_ss", "es": "_es",
        "ip": "_ip",
    }

    # The 8086 byte registers map to halves of the general-purpose regs:
    #   AL/BL/CL/DL = low byte (bits 0–7)
    #   AH/BH/CH/DH = high byte (bits 8–15)
    _BYTE_REG_MAP = {
        "al": ("ax", "low"), "ah": ("ax", "high"),
        "bl": ("bx", "low"), "bh": ("bx", "high"),
        "cl": ("cx", "low"), "ch": ("cx", "high"),
        "dl": ("dx", "low"), "dh": ("dx", "high"),
    }

    def _get_bits(self, reg: str) -> list[int]:
        """Return reference to the bit list for a 16-bit register."""
        attr = self._REG16_MAP[reg]
        return getattr(self, attr)

    def _set_bits(self, reg: str, bits: list[int]) -> None:
        """Store a 16-bit bit list into a register."""
        attr = self._REG16_MAP[reg]
        setattr(self, attr, list(bits))

    # ── 16-bit read / write ───────────────────────────────────────────────────

    def read16(self, reg: str) -> int:
        """Read a 16-bit register value.

        Args:
            reg: Register name (lowercase): "ax", "bx", "cx", "dx",
                 "si", "di", "sp", "bp", "cs", "ds", "ss", "es", "ip".

        Returns:
            Unsigned 16-bit integer (0–65535).

        Examples:
            >>> rf = RegisterFile8086(); rf.write16("bx", 0xABCD); rf.read16("bx")
            43981
        """
        return bits_to_int(self._get_bits(reg))

    def write16(self, reg: str, value: int) -> None:
        """Write a 16-bit value into a register.

        Args:
            reg:   Register name (lowercase).
            value: 16-bit unsigned value (masked to 0–65535).

        Examples:
            >>> rf = RegisterFile8086(); rf.write16("sp", 0xFFFE); rf.read16("sp")
            65534
        """
        value = value & 0xFFFF
        self._set_bits(reg, int_to_bits(value, 16))

    # ── 8-bit high/low byte access ────────────────────────────────────────────

    def read8_low(self, reg: str) -> int:
        """Read the low byte of a general-purpose register (AL/BL/CL/DL).

        Args:
            reg: One of "ax", "bx", "cx", "dx".

        Returns:
            Low 8 bits as unsigned integer (0–255).

        Examples:
            >>> rf = RegisterFile8086(); rf.write16("ax", 0x1234); rf.read8_low("ax")
            52
        """
        bits = self._get_bits(reg)
        return bits_to_int(bits[:8])

    def read8_high(self, reg: str) -> int:
        """Read the high byte of a general-purpose register (AH/BH/CH/DH).

        Args:
            reg: One of "ax", "bx", "cx", "dx".

        Returns:
            High 8 bits as unsigned integer (0–255).

        Examples:
            >>> rf = RegisterFile8086(); rf.write16("ax", 0x1234); rf.read8_high("ax")
            18
        """
        bits = self._get_bits(reg)
        return bits_to_int(bits[8:])

    def write8_low(self, reg: str, value: int) -> None:
        """Write the low byte of a general-purpose register.

        The high byte is preserved.

        Args:
            reg:   One of "ax", "bx", "cx", "dx".
            value: 8-bit value (masked to 0–255).

        Examples:
            >>> rf = RegisterFile8086()
            >>> rf.write16("ax", 0x1200); rf.write8_low("ax", 0x56)
            >>> rf.read16("ax")
            4694
        """
        value = value & 0xFF
        new_bits = int_to_bits(value, 8)
        current = self._get_bits(reg)
        self._set_bits(reg, new_bits + current[8:])

    def write8_high(self, reg: str, value: int) -> None:
        """Write the high byte of a general-purpose register.

        The low byte is preserved.

        Args:
            reg:   One of "ax", "bx", "cx", "dx".
            value: 8-bit value (masked to 0–255).

        Examples:
            >>> rf = RegisterFile8086()
            >>> rf.write16("ax", 0x0034); rf.write8_high("ax", 0x12)
            >>> rf.read16("ax")
            4660
        """
        value = value & 0xFF
        new_bits = int_to_bits(value, 8)
        current = self._get_bits(reg)
        self._set_bits(reg, current[:8] + new_bits)

    # ── FLAGS pack / unpack ───────────────────────────────────────────────────

    def pack_flags(self) -> int:
        """Pack all flag flip-flops into the 16-bit FLAGS register value.

        Layout:
            bit  0: CF    bit  1: 1 (always)    bit  2: PF
            bit  4: AF    bit  6: ZF             bit  7: SF
            bit  8: TF    bit  9: IF             bit 10: DF
            bit 11: OF

        Returns:
            16-bit FLAGS value.

        Examples:
            >>> rf = RegisterFile8086()
            >>> rf._flag_zf = 1
            >>> hex(rf.pack_flags())
            '0x42'
        """
        return (
            OR(self._flag_cf, 0) << 0   # bit 0: CF
            | (1 << 1)                   # bit 1: always 1
            | OR(self._flag_pf, 0) << 2  # bit 2: PF
            | OR(self._flag_af, 0) << 4  # bit 4: AF
            | OR(self._flag_zf, 0) << 6  # bit 6: ZF
            | OR(self._flag_sf, 0) << 7  # bit 7: SF
            | OR(self._flag_tf, 0) << 8  # bit 8: TF
            | OR(self._flag_if, 0) << 9  # bit 9: IF
            | OR(self._flag_df, 0) << 10 # bit 10: DF
            | OR(self._flag_of, 0) << 11 # bit 11: OF
        )

    def unpack_flags(self, flags: int) -> None:
        """Unpack a 16-bit FLAGS word into individual flag flip-flops.

        Used by POPF and IRET to restore processor status from the stack.

        Args:
            flags: 16-bit FLAGS value.

        Examples:
            >>> rf = RegisterFile8086()
            >>> rf.unpack_flags(0x0043)   # CF=1, PF=1, ZF=1, bit1=1
            >>> rf._flag_cf, rf._flag_pf, rf._flag_zf
            (1, 1, 1)
        """
        self._flag_cf = AND((flags >> 0) & 1, 1)
        self._flag_pf = AND((flags >> 2) & 1, 1)
        self._flag_af = AND((flags >> 4) & 1, 1)
        self._flag_zf = AND((flags >> 6) & 1, 1)
        self._flag_sf = AND((flags >> 7) & 1, 1)
        self._flag_tf = AND((flags >> 8) & 1, 1)
        self._flag_if = AND((flags >> 9) & 1, 1)
        self._flag_df = AND((flags >> 10) & 1, 1)
        self._flag_of = AND((flags >> 11) & 1, 1)

    # ── Physical address computation ──────────────────────────────────────────

    def physical_address(self, seg: str, offset: int) -> int:
        """Compute 20-bit physical address: seg_reg × 16 + offset.

        The "× 16" is a 4-bit left shift — hardware rewiring.
        The addition uses add_20bit() through the ripple-carry gate chain.

        Args:
            seg:    Segment register name: "cs", "ds", "ss", "es".
            offset: 16-bit offset (0–65535).

        Returns:
            20-bit physical address (0–0xFFFFF).

        Examples:
            >>> rf = RegisterFile8086()
            >>> rf.write16("cs", 0x1000); rf.physical_address("cs", 0x0100)
            65792
        """
        seg_val = self.read16(seg)
        # seg × 16: shift 16-bit segment value left 4 bits → 20-bit value
        seg_shifted = seg_val << 4  # Wire A[0..15] to output[4..19]; output[0..3] = 0
        result, _ = add_20bit(seg_shifted, offset & 0xFFFF)
        return result & 0xFFFFF
