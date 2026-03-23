"""Instruction decoder — combinational logic that maps opcodes to control signals.

=== How instruction decoding works in hardware ===

The decoder takes an 8-bit instruction byte and produces control signals
that tell the rest of the CPU what to do. In the real 4004, this was a
combinational logic network — a forest of AND, OR, and NOT gates that
pattern-match the opcode bits.

For example, to detect LDM (0xD_):
    is_ldm = AND(bit7, bit6, NOT(bit5), bit4)  → bits 7654 = 1101

The decoder doesn't use sequential logic — it's purely combinational.
Given the same input bits, it always produces the same output signals.

=== Control signals ===

The decoder outputs tell the control unit what to do:
    - write_acc:    Write a value to the accumulator
    - write_reg:    Write a value to the register file
    - write_carry:  Update the carry flag
    - alu_add:      Route through ALU add
    - alu_sub:      Route through ALU subtract
    - is_jump:      This is a jump instruction
    - is_call:      This is JMS (push return address)
    - is_return:    This is BBL (pop and return)
    - is_two_byte:  Instruction is 2 bytes
    - uses_ram:     Instruction accesses RAM
    - reg_index:    Which register (lower nibble)
    - pair_index:   Which register pair
    - immediate:    Immediate value from instruction
"""

from __future__ import annotations

from dataclasses import dataclass

from logic_gates import AND, NOT, OR


@dataclass
class DecodedInstruction:
    """Control signals produced by the instruction decoder.

    Every field represents a wire carrying a 0 or 1 signal, or a
    multi-bit value extracted from the instruction.
    """

    # Original instruction bytes
    raw: int
    raw2: int | None

    # Upper and lower nibbles
    upper: int  # bits [7:4]
    lower: int  # bits [3:0]

    # Instruction family detection (from gate logic)
    is_nop: int
    is_hlt: int
    is_ldm: int
    is_ld: int
    is_xch: int
    is_inc: int
    is_add: int
    is_sub: int
    is_jun: int
    is_jcn: int
    is_isz: int
    is_jms: int
    is_bbl: int
    is_fim: int
    is_src: int
    is_fin: int
    is_jin: int
    is_io: int       # 0xE_ range
    is_accum: int    # 0xF_ range

    # Two-byte flag
    is_two_byte: int

    # Operand extraction
    reg_index: int   # lower nibble (register index)
    pair_index: int  # lower nibble >> 1 (pair index)
    immediate: int   # lower nibble (immediate value)
    condition: int   # lower nibble (JCN condition code)

    # For 2-byte instructions
    addr12: int      # 12-bit address (JUN/JMS)
    addr8: int       # 8-bit address/data (JCN/ISZ/FIM)


def decode(raw: int, raw2: int | None = None) -> DecodedInstruction:
    """Decode an instruction byte into control signals using gates.

    In real hardware, this is a combinational circuit — no clock needed.
    The input bits propagate through AND/OR/NOT gate trees to produce
    the output control signals.

    Args:
        raw: First instruction byte (0x00–0xFF).
        raw2: Second byte for 2-byte instructions, or None.

    Returns:
        DecodedInstruction with all control signals set.
    """
    # Extract individual bits using AND gates (masking)
    b7 = (raw >> 7) & 1
    b6 = (raw >> 6) & 1
    b5 = (raw >> 5) & 1
    b4 = (raw >> 4) & 1
    b3 = (raw >> 3) & 1
    b2 = (raw >> 2) & 1
    b1 = (raw >> 1) & 1
    b0 = raw & 1

    upper = (raw >> 4) & 0xF
    lower = raw & 0xF

    # --- Instruction family detection ---
    # Each family is detected by AND-ing the upper nibble bits.
    # Using NOT for inverted bits.

    # NOP = 0x00: all bits zero
    is_nop = AND(
        AND(NOT(b7), NOT(b6)),
        AND(AND(NOT(b5), NOT(b4)), AND(NOT(b3), NOT(b2))),
    )
    is_nop = AND(is_nop, AND(NOT(b1), NOT(b0)))

    # HLT = 0x01: only b0 is 1
    is_hlt = AND(
        AND(NOT(b7), NOT(b6)),
        AND(AND(NOT(b5), NOT(b4)), AND(NOT(b3), NOT(b2))),
    )
    is_hlt = AND(is_hlt, AND(NOT(b1), b0))

    # Upper nibble patterns (using gate logic):
    # 0x1_ = 0001 : JCN
    is_jcn_family = AND(AND(NOT(b7), NOT(b6)), AND(NOT(b5), b4))

    # 0x2_ = 0010 : FIM (even b0) or SRC (odd b0)
    is_2x = AND(AND(NOT(b7), NOT(b6)), AND(b5, NOT(b4)))
    is_fim = AND(is_2x, NOT(b0))
    is_src = AND(is_2x, b0)

    # 0x3_ = 0011 : FIN (even b0) or JIN (odd b0)
    is_3x = AND(AND(NOT(b7), NOT(b6)), AND(b5, b4))
    is_fin = AND(is_3x, NOT(b0))
    is_jin = AND(is_3x, b0)

    # 0x4_ = 0100 : JUN
    is_jun_family = AND(AND(NOT(b7), b6), AND(NOT(b5), NOT(b4)))

    # 0x5_ = 0101 : JMS
    is_jms_family = AND(AND(NOT(b7), b6), AND(NOT(b5), b4))

    # 0x6_ = 0110 : INC
    is_inc_family = AND(AND(NOT(b7), b6), AND(b5, NOT(b4)))

    # 0x7_ = 0111 : ISZ
    is_isz_family = AND(AND(NOT(b7), b6), AND(b5, b4))

    # 0x8_ = 1000 : ADD
    is_add_family = AND(AND(b7, NOT(b6)), AND(NOT(b5), NOT(b4)))

    # 0x9_ = 1001 : SUB
    is_sub_family = AND(AND(b7, NOT(b6)), AND(NOT(b5), b4))

    # 0xA_ = 1010 : LD
    is_ld_family = AND(AND(b7, NOT(b6)), AND(b5, NOT(b4)))

    # 0xB_ = 1011 : XCH
    is_xch_family = AND(AND(b7, NOT(b6)), AND(b5, b4))

    # 0xC_ = 1100 : BBL
    is_bbl_family = AND(AND(b7, b6), AND(NOT(b5), NOT(b4)))

    # 0xD_ = 1101 : LDM
    is_ldm_family = AND(AND(b7, b6), AND(NOT(b5), b4))

    # 0xE_ = 1110 : I/O operations
    is_io_family = AND(AND(b7, b6), AND(b5, NOT(b4)))

    # 0xF_ = 1111 : accumulator operations
    is_accum_family = AND(AND(b7, b6), AND(b5, b4))

    # Two-byte detection
    is_two_byte = OR(
        OR(is_jcn_family, is_jun_family),
        OR(OR(is_jms_family, is_isz_family), is_fim),
    )

    # Operand extraction
    reg_index = lower
    pair_index = lower >> 1
    immediate = lower
    condition = lower

    # 12-bit address for JUN/JMS
    second = raw2 if raw2 is not None else 0
    addr12 = (lower << 8) | second
    addr8 = second

    return DecodedInstruction(
        raw=raw,
        raw2=raw2,
        upper=upper,
        lower=lower,
        is_nop=is_nop,
        is_hlt=is_hlt,
        is_ldm=is_ldm_family,
        is_ld=is_ld_family,
        is_xch=is_xch_family,
        is_inc=is_inc_family,
        is_add=is_add_family,
        is_sub=is_sub_family,
        is_jun=is_jun_family,
        is_jcn=is_jcn_family,
        is_isz=is_isz_family,
        is_jms=is_jms_family,
        is_bbl=is_bbl_family,
        is_fim=is_fim,
        is_src=is_src,
        is_fin=is_fin,
        is_jin=is_jin,
        is_io=is_io_family,
        is_accum=is_accum_family,
        is_two_byte=is_two_byte,
        reg_index=reg_index,
        pair_index=pair_index,
        immediate=immediate,
        condition=condition,
        addr12=addr12,
        addr8=addr8,
    )
