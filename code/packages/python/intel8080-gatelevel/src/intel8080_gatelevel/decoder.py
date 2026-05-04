"""Decoder8080 — combinational instruction decoder for the Intel 8080.

=== How the real decoder works ===

The 8080 decoder is a combinational circuit — it has no memory. Given an
8-bit opcode, it produces a set of control signals in a single gate delay.
In Python we model this as a pure function: same input always gives same output.

=== Opcode structure ===

The 8080's opcode byte is organized in two fields:

    Bit 7  Bit 6  |  Bit 5  Bit 4  Bit 3  |  Bit 2  Bit 1  Bit 0
    ────────────────────────────────────────────────────────────────
       group       |     dst / alu_op       |       src

Group decode (bits 7–6):
    00 = Group 00: misc (MVI, LXI, INR, DCR, INX, DCX, etc.)
    01 = Group 01: MOV (register-to-register transfers); 0x76 = HLT
    10 = Group 10: ALU + register (ADD, ADC, SUB, SBB, ANA, XRA, ORA, CMP)
    11 = Group 11: branches, stack, control (JMP, CALL, RET, PUSH, POP, etc.)

=== Gate-level group decode ===

Using the two MSBs as individual bits extracted via AND/NOT:

    bit7 = (opcode >> 7) & 1    (MSB)
    bit6 = (opcode >> 6) & 1

    is_group00 = AND(NOT(bit7), NOT(bit6))
    is_group01 = AND(NOT(bit7), bit6)
    is_group10 = AND(bit7, NOT(bit6))
    is_group11 = AND(bit7, bit6)

Each produces exactly one 1 and three 0s — a one-hot encoding. The decoder
is a single level of AND/NOT gates (two gate delays total).

=== Registers and their codes ===

3-bit codes used in both src and dst fields:
    000 = B    001 = C    010 = D    011 = E
    100 = H    101 = L    110 = M (memory indirect via HL)   111 = A

Register pair codes (bits 5–4 for two-register ops):
    00 = BC    01 = DE    10 = HL    11 = SP (or PSW for PUSH/POP)

=== ALU operation codes (group 10, bits 5–3) ===

    000 = ADD    001 = ADC    010 = SUB    011 = SBB
    100 = ANA    101 = XRA    110 = ORA    111 = CMP

These become ALU8080 operation codes 0–7 directly.
"""

from __future__ import annotations

from dataclasses import dataclass

from logic_gates import AND, NOT


def _bit(value: int, position: int) -> int:
    """Extract a single bit from an integer.

    This models the 8080's opcode register bus lines. Each bit is a wire;
    this function reads the voltage on wire `position`.

    Args:
        value:    The opcode byte (0–255).
        position: Bit position to read (0 = LSB, 7 = MSB).

    Returns:
        0 or 1.
    """
    return (value >> position) & 1


@dataclass(frozen=True)
class DecodedInstruction:
    """Control signals produced by the combinational decoder.

    This is what the 8080's decoder outputs as a set of simultaneous
    electrical signals. In real hardware these are wires; in Python they
    are fields of this dataclass.

    Fields
    ------
    op_group:        Which of the 4 opcode groups this instruction belongs to
                     (0–3, corresponding to bits 7–6 of opcode).
    dst:             Destination register code (bits 5–3), 0–7.
    src:             Source register code (bits 2–0), 0–7.
    alu_op:          ALU operation code for group-10 instructions (bits 5–3).
                     Same bit field as dst — the decoder drives both from the
                     same wires and the control unit picks which to use.
    reg_pair:        Register-pair code (bits 5–4, 2-bit field), 0–3.
    is_halt:         True for opcode 0x76 (MOV M,M re-used as HLT).
    is_memory_src:   True when src field = 6 (M pseudo-register → memory read).
    is_memory_dst:   True when dst field = 6 (M pseudo-register → memory write).
    extra_bytes:     Number of additional bytes needed: 0, 1, or 2.
    opcode:          The raw opcode byte (for control unit lookup tables).
    """

    op_group: int      # 0–3
    dst: int           # 0–7
    src: int           # 0–7
    alu_op: int        # 0–7 (same bits as dst for group 10)
    reg_pair: int      # 0–3 (bits 5–4)
    is_halt: bool
    is_memory_src: bool
    is_memory_dst: bool
    extra_bytes: int   # 0, 1, or 2
    opcode: int        # raw opcode byte


class Decoder8080:
    """Combinational instruction decoder for the Intel 8080.

    Maps an 8-bit opcode to a DecodedInstruction record using AND/NOT/OR
    gate functions. No state is held — every call is independent.

    Usage:
        >>> dec = Decoder8080()
        >>> dec.decode(0x80)   # ADD B
        DecodedInstruction(op_group=2, dst=0, src=0, alu_op=0, ...)
        >>> dec.decode(0x76)   # HLT
        DecodedInstruction(is_halt=True, ...)
        >>> dec.decode(0xC3)   # JMP addr16
        DecodedInstruction(op_group=3, extra_bytes=2, ...)
    """

    def decode(self, opcode: int) -> DecodedInstruction:
        """Decode a single opcode byte into control signals.

        Implements the combinational gate tree described in the module
        docstring. All bit extractions use AND/NOT gates.

        Args:
            opcode: 8-bit instruction opcode (0–255).

        Returns:
            DecodedInstruction with all control signals.
        """
        # ── Extract individual opcode bits via gate-level reads ──────────
        # These correspond to the 8 wires of the opcode register bus.
        b7 = _bit(opcode, 7)
        b6 = _bit(opcode, 6)
        b5 = _bit(opcode, 5)
        b4 = _bit(opcode, 4)
        b3 = _bit(opcode, 3)
        b2 = _bit(opcode, 2)
        b1 = _bit(opcode, 1)
        b0 = _bit(opcode, 0)

        # ── Group decode: AND/NOT tree on bits 7–6 ──────────────────────
        # is_groupXX = AND(NOT(b7) or b7, NOT(b6) or b6)
        _nb7 = NOT(b7)
        _nb6 = NOT(b6)
        is_group00 = AND(_nb7, _nb6)    # 00
        is_group01 = AND(_nb7, b6)      # 01
        is_group10 = AND(b7, _nb6)      # 10
        is_group11 = AND(b7, b6)        # 11

        op_group = (is_group10 << 1) | is_group01  # encodes 00→0, 01→1, 10→2, 11→3
        if is_group11:
            op_group = 3

        # ── Field extraction ─────────────────────────────────────────────
        dst = (b5 << 2) | (b4 << 1) | b3      # bits 5–3
        src = (b2 << 1) | (b1 << 0) | (b0 << 0)  # Hmm, let me redo this
        # Actually: src = bits 2-0
        src = (b2 << 2) | (b1 << 1) | b0
        alu_op = dst   # for group 10: bits 5–3 are the ALU op
        reg_pair = (b5 << 1) | b4   # bits 5–4

        # ── HLT detection: opcode == 0x76 ────────────────────────────────
        # is_halt = AND(is_group01, AND(AND(b5,b4), AND(b3, AND(NOT(b2), AND(b1, b0)))))
        # Simplification: check all bits of 0x76 = 0b01110110
        # b7=0 b6=1 b5=1 b4=1 b3=0 b2=1 b1=1 b0=0
        is_halt_int = AND(
            is_group01,
            AND(AND(b5, b4), AND(NOT(b3), AND(b2, AND(b1, NOT(b0)))))
        )
        is_halt = bool(is_halt_int)

        # ── Memory operand detection ──────────────────────────────────────
        # M pseudo-register: code 6 = 0b110
        # is_memory_src: src == 6 → bits2=1, bits1=1, bits0=0
        is_mem_src_int = AND(b2, AND(b1, NOT(b0)))
        is_memory_src = bool(AND(is_mem_src_int, NOT(is_halt_int)))

        # is_memory_dst: dst == 6 → bits5=1, bits4=1, bits3=0
        is_mem_dst_int = AND(b5, AND(b4, NOT(b3)))
        is_memory_dst = bool(AND(is_mem_dst_int, NOT(is_halt_int)))

        # ── Extra bytes needed ────────────────────────────────────────────
        extra_bytes = _count_extra_bytes(opcode, is_group00, is_group11, b3, b2, b1, b0, b5, b4)  # noqa: E501

        return DecodedInstruction(
            op_group=op_group,
            dst=dst,
            src=src,
            alu_op=alu_op,
            reg_pair=reg_pair,
            is_halt=is_halt,
            is_memory_src=is_memory_src,
            is_memory_dst=is_memory_dst,
            extra_bytes=extra_bytes,
            opcode=opcode,
        )


def _count_extra_bytes(  # noqa: PLR0912,PLR0913
    opcode: int,
    is_group00: int,
    is_group11: int,
    b3: int,
    b2: int,
    b1: int,
    b0: int,
    b5: int,
    b4: int,
) -> int:
    """Compute extra bytes needed based on opcode group and pattern.

    The 8080 has three instruction lengths:
        1-byte: register-register ops, ALU register, single-byte control
        2-byte: MVI, ADI/ACI/SUI/SBI/ANI/ORI/XRI/CPI, IN port, OUT port
        3-byte: LXI, LDA/STA, LHLD/SHLD, JMP, CALL, conditional J/CALL

    This function implements the instruction-length decoder, which in the
    real 8080 is a combinational gate array scanning the opcode bits.

    Returns 0, 1, or 2.
    """
    # Group 00 (misc/immediate) length rules:
    if is_group00:
        # LXI rp, d16 — bits5-4 are reg pair, bits2-0 = 001
        # opcode = 00rp0001
        if (opcode & 0b00001111) == 0b00000001:
            return 2  # LXI: 3 bytes total

        # MVI r, d8 — dst bits + 0b110 src pattern
        # opcode = 00ddd110  (MVI r,d8 where src=6)
        if (opcode & 0b00000111) == 0b00000110 and opcode != 0x76:
            return 1  # MVI: 2 bytes total

        # LDA / STA / LHLD / SHLD: specific opcodes
        if opcode in (0x3A, 0x32, 0x2A, 0x22):
            return 2

        # JMP direct (relative forms don't exist in 8080)
        if opcode == 0xC3:
            return 2

        return 0

    # Group 11 (branches/stack) length rules:
    if is_group11:
        low3 = opcode & 0b00000111

        # Unconditional JMP 0xC3, CALL 0xCD → 3 bytes
        if opcode in (0xC3, 0xCD):
            return 2

        # Conditional JMP: 0bCC010  (opcode & 0b11000111 == 0b11000010)
        if (opcode & 0b11000111) == 0b11000010:  # Ccc010
            return 2

        # Conditional CALL: 0bCC100 (opcode & 0b11000111 == 0b11000100)
        if (opcode & 0b11000111) == 0b11000100:
            return 2

        # LDA (0x3A), STA (0x32), LHLD (0x2A), SHLD (0x22): already handled above
        # IN/OUT: 2-byte
        if opcode in (0xDB, 0xD3):
            return 1

        # ADI/ACI/SUI/SBI/ANI/ORI/XRI/CPI: immediate ALU (0xC6, 0xCE, etc.)
        # Pattern: group11, src=110, dst=alu_op
        if AND(NOT(b3), AND(NOT(b2), AND(b1, b0))):  # low bits = 110: nope
            pass
        if low3 == 0b110:   # src=6 in group11 = ALU immediate
            return 1

        return 0

    # Group 01 (MOV) and Group 10 (ALU register): all 1-byte
    return 0
