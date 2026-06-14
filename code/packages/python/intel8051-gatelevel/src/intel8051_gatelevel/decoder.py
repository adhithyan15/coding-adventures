"""decoder.py — Gate-tree instruction decoder for the Intel 8051.

On real hardware, instruction decode is performed by a combinational logic
network that takes the 8 opcode bits and generates control signals.  The
decoder is typically a two-level AND-OR network (sum of minterms).

This module models that decode using AND/OR/NOT gates from logic_gates.
Every classification decision is made through explicit gate function calls
rather than Python `if opcode == ...` or `match` statements.

=============================================================================
8051 opcode structure
=============================================================================

The 8051 opcode byte encodes the instruction family in its upper bits:

    Bits [7:5] — major group (3 bits → 8 major groups)
    Bits [4:0] — sub-operation within the group

For register-indexed families (Rn = R0..R7), bits[7:3] are fixed and
bits[2:0] select the register.  Family detection must check only bits[7:3].

For indirect-register families (@Ri = @R0 or @R1), bits[7:1] are fixed
and bit[0] selects R0 or R1.

=============================================================================
Gate-tree decode
=============================================================================

For each opcode bit, we extract the bit value with:
    bit_N = (opcode >> N) & 1

Then apply AND/OR/NOT to build classification logic:
    is_add_family = AND(NOT(bit7), AND(bit6, NOT(bit5)))

This mirrors how a PLA (Programmable Logic Array) works in hardware:
the AND plane computes product terms (minterms), and the OR plane
combines them into output signals.

Key principle for "family" checks (Rn variants):
    Check ONLY the bits that are fixed for the family.
    Do NOT check the bits that encode the register number.
    Example: MOV A,Rn covers 0xE8-0xEF; bits[7:3]=11101.
    Condition: AND(b7, AND(b6, AND(b5, AND(nb4, b3))))
    (bits 2,1,0 are NOT checked — they select R0..R7)
"""

from __future__ import annotations

from logic_gates import AND, NOT


def _bits(opcode: int) -> list[int]:
    """Extract the 8 bits of an opcode byte (LSB-first).

    Returns list[int] where index 0 is bit 0 (LSB), index 7 is bit 7 (MSB).
    This matches the gate-level convention used throughout the simulator.
    """
    return [(opcode >> i) & 1 for i in range(8)]


def decode_opcode(opcode: int) -> str:
    """Decode an 8051 opcode byte to a mnemonic string using gate trees.

    Strategy: check the most-specific patterns (fully-specified 8-bit matches)
    before less-specific family patterns (5-bit top-bits-only checks).  This
    prevents a specific opcode like 0xE5 (MOV A,dir) from being swallowed by
    the 0xE8-0xEF family pattern.

    Args:
        opcode: Instruction opcode byte (0-255).

    Returns:
        Mnemonic string such as "ADD A,Rn", "MOV A,#imm", "DJNZ Rn", etc.
        Returns "UNKNOWN" for unrecognized opcodes.
    """
    b = _bits(opcode)

    # Bit signal names (hardware convention: b7=MSB, b0=LSB)
    b7, b6, b5, b4, b3, b2, b1, b0 = b[7], b[6], b[5], b[4], b[3], b[2], b[1], b[0]

    # Inverted signals — NOT gate on each bit
    nb7 = NOT(b7)
    nb6 = NOT(b6)
    nb5 = NOT(b5)
    nb4 = NOT(b4)
    nb3 = NOT(b3)
    nb2 = NOT(b2)
    nb1 = NOT(b1)
    nb0 = NOT(b0)

    # =========================================================================
    # Fully-specified 8-bit patterns (checked first to avoid false family match)
    # =========================================================================

    # NOP: 0x00 = 00000000
    if AND(nb7, AND(nb6, AND(nb5, AND(nb4, AND(nb3, AND(nb2, AND(nb1, nb0))))))):
        return "NOP"

    # HALT: 0xA5 = 10100101
    if AND(b7, AND(nb6, AND(b5, AND(nb4, AND(nb3, AND(b2, AND(nb1, b0))))))):
        return "HALT"

    # RR A: 0x03 = 00000011
    if AND(nb7, AND(nb6, AND(nb5, AND(nb4, AND(nb3, AND(nb2, AND(b1, b0))))))):
        return "RR A"

    # INC A: 0x04 = 00000100
    if AND(nb7, AND(nb6, AND(nb5, AND(nb4, AND(nb3, AND(b2, AND(nb1, nb0))))))):
        return "INC A"

    # INC dir: 0x05 = 00000101
    if AND(nb7, AND(nb6, AND(nb5, AND(nb4, AND(nb3, AND(b2, AND(nb1, b0))))))):
        return "INC dir"

    # INC @R0: 0x06 = 00000110
    if AND(nb7, AND(nb6, AND(nb5, AND(nb4, AND(nb3, AND(b2, AND(b1, nb0))))))):
        return "INC @R0"

    # INC @R1: 0x07 = 00000111
    if AND(nb7, AND(nb6, AND(nb5, AND(nb4, AND(nb3, AND(b2, AND(b1, b0))))))):
        return "INC @R1"

    # JBC bit,rel: 0x10 = 00010000
    if AND(nb7, AND(nb6, AND(nb5, AND(b4, AND(nb3, AND(nb2, AND(nb1, nb0))))))):
        return "JBC"

    # LCALL: 0x12 = 00010010
    if AND(nb7, AND(nb6, AND(nb5, AND(b4, AND(nb3, AND(nb2, AND(b1, nb0))))))):
        return "LCALL"

    # RRC A: 0x13 = 00010011
    if AND(nb7, AND(nb6, AND(nb5, AND(b4, AND(nb3, AND(nb2, AND(b1, b0))))))):
        return "RRC A"

    # DEC A: 0x14 = 00010100
    if AND(nb7, AND(nb6, AND(nb5, AND(b4, AND(nb3, AND(b2, AND(nb1, nb0))))))):
        return "DEC A"

    # DEC dir: 0x15 = 00010101
    if AND(nb7, AND(nb6, AND(nb5, AND(b4, AND(nb3, AND(b2, AND(nb1, b0))))))):
        return "DEC dir"

    # DEC @R0: 0x16 = 00010110
    if AND(nb7, AND(nb6, AND(nb5, AND(b4, AND(nb3, AND(b2, AND(b1, nb0))))))):
        return "DEC @R0"

    # DEC @R1: 0x17 = 00010111
    if AND(nb7, AND(nb6, AND(nb5, AND(b4, AND(nb3, AND(b2, AND(b1, b0))))))):
        return "DEC @R1"

    # JB bit,rel: 0x20 = 00100000
    if AND(nb7, AND(nb6, AND(b5, AND(nb4, AND(nb3, AND(nb2, AND(nb1, nb0))))))):
        return "JB"

    # RET: 0x22 = 00100010
    if AND(nb7, AND(nb6, AND(b5, AND(nb4, AND(nb3, AND(nb2, AND(b1, nb0))))))):
        return "RET"

    # RL A: 0x23 = 00100011
    if AND(nb7, AND(nb6, AND(b5, AND(nb4, AND(nb3, AND(nb2, AND(b1, b0))))))):
        return "RL A"

    # ADD A,#imm: 0x24 = 00100100
    if AND(nb7, AND(nb6, AND(b5, AND(nb4, AND(nb3, AND(b2, AND(nb1, nb0))))))):
        return "ADD A,#imm"

    # ADD A,dir: 0x25 = 00100101
    if AND(nb7, AND(nb6, AND(b5, AND(nb4, AND(nb3, AND(b2, AND(nb1, b0))))))):
        return "ADD A,dir"

    # ADD A,@R0: 0x26 = 00100110
    if AND(nb7, AND(nb6, AND(b5, AND(nb4, AND(nb3, AND(b2, AND(b1, nb0))))))):
        return "ADD A,@R0"

    # ADD A,@R1: 0x27 = 00100111
    if AND(nb7, AND(nb6, AND(b5, AND(nb4, AND(nb3, AND(b2, AND(b1, b0))))))):
        return "ADD A,@R1"

    # JNB bit,rel: 0x30 = 00110000
    if AND(nb7, AND(nb6, AND(b5, AND(b4, AND(nb3, AND(nb2, AND(nb1, nb0))))))):
        return "JNB"

    # RETI: 0x32 = 00110010
    if AND(nb7, AND(nb6, AND(b5, AND(b4, AND(nb3, AND(nb2, AND(b1, nb0))))))):
        return "RETI"

    # RLC A: 0x33 = 00110011
    if AND(nb7, AND(nb6, AND(b5, AND(b4, AND(nb3, AND(nb2, AND(b1, b0))))))):
        return "RLC A"

    # ADDC A,#imm: 0x34 = 00110100
    if AND(nb7, AND(nb6, AND(b5, AND(b4, AND(nb3, AND(b2, AND(nb1, nb0))))))):
        return "ADDC A,#imm"

    # ADDC A,dir: 0x35 = 00110101
    if AND(nb7, AND(nb6, AND(b5, AND(b4, AND(nb3, AND(b2, AND(nb1, b0))))))):
        return "ADDC A,dir"

    # ADDC A,@R0: 0x36 = 00110110
    if AND(nb7, AND(nb6, AND(b5, AND(b4, AND(nb3, AND(b2, AND(b1, nb0))))))):
        return "ADDC A,@R0"

    # ADDC A,@R1: 0x37 = 00110111
    if AND(nb7, AND(nb6, AND(b5, AND(b4, AND(nb3, AND(b2, AND(b1, b0))))))):
        return "ADDC A,@R1"

    # JC rel: 0x40 = 01000000
    if AND(nb7, AND(b6, AND(nb5, AND(nb4, AND(nb3, AND(nb2, AND(nb1, nb0))))))):
        return "JC"

    # ORL A,#imm: 0x44 = 01000100
    if AND(nb7, AND(b6, AND(nb5, AND(nb4, AND(nb3, AND(b2, AND(nb1, nb0))))))):
        return "ORL A,#imm"

    # ORL A,dir: 0x45 = 01000101
    if AND(nb7, AND(b6, AND(nb5, AND(nb4, AND(nb3, AND(b2, AND(nb1, b0))))))):
        return "ORL A,dir"

    # ORL A,@R0: 0x46 = 01000110
    if AND(nb7, AND(b6, AND(nb5, AND(nb4, AND(nb3, AND(b2, AND(b1, nb0))))))):
        return "ORL A,@R0"

    # ORL A,@R1: 0x47 = 01000111
    if AND(nb7, AND(b6, AND(nb5, AND(nb4, AND(nb3, AND(b2, AND(b1, b0))))))):
        return "ORL A,@R1"

    # ORL dir,A: 0x42 = 01000010
    if AND(nb7, AND(b6, AND(nb5, AND(nb4, AND(nb3, AND(nb2, AND(b1, nb0))))))):
        return "ORL dir,A"

    # ORL dir,#imm: 0x43 = 01000011
    if AND(nb7, AND(b6, AND(nb5, AND(nb4, AND(nb3, AND(nb2, AND(b1, b0))))))):
        return "ORL dir,#imm"

    # JNC rel: 0x50 = 01010000
    if AND(nb7, AND(b6, AND(nb5, AND(b4, AND(nb3, AND(nb2, AND(nb1, nb0))))))):
        return "JNC"

    # ANL A,#imm: 0x54 = 01010100
    if AND(nb7, AND(b6, AND(nb5, AND(b4, AND(nb3, AND(b2, AND(nb1, nb0))))))):
        return "ANL A,#imm"

    # ANL A,dir: 0x55 = 01010101
    if AND(nb7, AND(b6, AND(nb5, AND(b4, AND(nb3, AND(b2, AND(nb1, b0))))))):
        return "ANL A,dir"

    # ANL A,@R0: 0x56 = 01010110
    if AND(nb7, AND(b6, AND(nb5, AND(b4, AND(nb3, AND(b2, AND(b1, nb0))))))):
        return "ANL A,@R0"

    # ANL A,@R1: 0x57 = 01010111
    if AND(nb7, AND(b6, AND(nb5, AND(b4, AND(nb3, AND(b2, AND(b1, b0))))))):
        return "ANL A,@R1"

    # ANL dir,A: 0x52 = 01010010
    if AND(nb7, AND(b6, AND(nb5, AND(b4, AND(nb3, AND(nb2, AND(b1, nb0))))))):
        return "ANL dir,A"

    # ANL dir,#imm: 0x53 = 01010011
    if AND(nb7, AND(b6, AND(nb5, AND(b4, AND(nb3, AND(nb2, AND(b1, b0))))))):
        return "ANL dir,#imm"

    # JZ rel: 0x60 = 01100000
    if AND(nb7, AND(b6, AND(b5, AND(nb4, AND(nb3, AND(nb2, AND(nb1, nb0))))))):
        return "JZ"

    # XRL A,#imm: 0x64 = 01100100
    if AND(nb7, AND(b6, AND(b5, AND(nb4, AND(nb3, AND(b2, AND(nb1, nb0))))))):
        return "XRL A,#imm"

    # XRL A,dir: 0x65 = 01100101
    if AND(nb7, AND(b6, AND(b5, AND(nb4, AND(nb3, AND(b2, AND(nb1, b0))))))):
        return "XRL A,dir"

    # XRL A,@R0: 0x66 = 01100110
    if AND(nb7, AND(b6, AND(b5, AND(nb4, AND(nb3, AND(b2, AND(b1, nb0))))))):
        return "XRL A,@R0"

    # XRL A,@R1: 0x67 = 01100111
    if AND(nb7, AND(b6, AND(b5, AND(nb4, AND(nb3, AND(b2, AND(b1, b0))))))):
        return "XRL A,@R1"

    # XRL dir,A: 0x62 = 01100010
    if AND(nb7, AND(b6, AND(b5, AND(nb4, AND(nb3, AND(nb2, AND(b1, nb0))))))):
        return "XRL dir,A"

    # XRL dir,#imm: 0x63 = 01100011
    if AND(nb7, AND(b6, AND(b5, AND(nb4, AND(nb3, AND(nb2, AND(b1, b0))))))):
        return "XRL dir,#imm"

    # JNZ rel: 0x70 = 01110000
    if AND(nb7, AND(b6, AND(b5, AND(b4, AND(nb3, AND(nb2, AND(nb1, nb0))))))):
        return "JNZ"

    # MOV A,#imm: 0x74 = 01110100
    if AND(nb7, AND(b6, AND(b5, AND(b4, AND(nb3, AND(b2, AND(nb1, nb0))))))):
        return "MOV A,#imm"

    # MOV dir,A: 0xF5 — handled in 0xF0-0xFF block below
    # MOV dir,#imm: 0x75 = 01110101
    if AND(nb7, AND(b6, AND(b5, AND(b4, AND(nb3, AND(b2, AND(nb1, b0))))))):
        return "MOV dir,#imm"

    # MOV @R0,#imm: 0x76 = 01110110
    if AND(nb7, AND(b6, AND(b5, AND(b4, AND(nb3, AND(b2, AND(b1, nb0))))))):
        return "MOV @R0,#imm"

    # MOV @R1,#imm: 0x77 = 01110111
    if AND(nb7, AND(b6, AND(b5, AND(b4, AND(nb3, AND(b2, AND(b1, b0))))))):
        return "MOV @R1,#imm"

    # SJMP: 0x80 = 10000000
    if AND(b7, AND(nb6, AND(nb5, AND(nb4, AND(nb3, AND(nb2, AND(nb1, nb0))))))):
        return "SJMP"

    # MOVC A,@A+PC: 0x83 = 10000011
    if AND(b7, AND(nb6, AND(nb5, AND(nb4, AND(nb3, AND(nb2, AND(b1, b0))))))):
        return "MOVC A,@A+PC"

    # DIV AB: 0x84 = 10000100
    if AND(b7, AND(nb6, AND(nb5, AND(nb4, AND(nb3, AND(b2, AND(nb1, nb0))))))):
        return "DIV AB"

    # MOV dir,dir: 0x85 = 10000101
    if AND(b7, AND(nb6, AND(nb5, AND(nb4, AND(nb3, AND(b2, AND(nb1, b0))))))):
        return "MOV dir,dir"

    # MOV dir,@R0: 0x86 = 10000110
    if AND(b7, AND(nb6, AND(nb5, AND(nb4, AND(nb3, AND(b2, AND(b1, nb0))))))):
        return "MOV dir,@R0"

    # MOV dir,@R1: 0x87 = 10000111
    if AND(b7, AND(nb6, AND(nb5, AND(nb4, AND(nb3, AND(b2, AND(b1, b0))))))):
        return "MOV dir,@R1"

    # MOV DPTR,#imm16: 0x90 = 10010000
    if AND(b7, AND(nb6, AND(nb5, AND(b4, AND(nb3, AND(nb2, AND(nb1, nb0))))))):
        return "MOV DPTR,#imm16"

    # MOV bit,C: 0x92 = 10010010
    if AND(b7, AND(nb6, AND(nb5, AND(b4, AND(nb3, AND(nb2, AND(b1, nb0))))))):
        return "MOV bit,C"

    # MOVC A,@A+DPTR: 0x93 = 10010011
    if AND(b7, AND(nb6, AND(nb5, AND(b4, AND(nb3, AND(nb2, AND(b1, b0))))))):
        return "MOVC A,@A+DPTR"

    # SUBB A,#imm: 0x94 = 10010100
    if AND(b7, AND(nb6, AND(nb5, AND(b4, AND(nb3, AND(b2, AND(nb1, nb0))))))):
        return "SUBB A,#imm"

    # SUBB A,dir: 0x95 = 10010101
    if AND(b7, AND(nb6, AND(nb5, AND(b4, AND(nb3, AND(b2, AND(nb1, b0))))))):
        return "SUBB A,dir"

    # SUBB A,@R0: 0x96 = 10010110
    if AND(b7, AND(nb6, AND(nb5, AND(b4, AND(nb3, AND(b2, AND(b1, nb0))))))):
        return "SUBB A,@R0"

    # SUBB A,@R1: 0x97 = 10010111
    if AND(b7, AND(nb6, AND(nb5, AND(b4, AND(nb3, AND(b2, AND(b1, b0))))))):
        return "SUBB A,@R1"

    # ORL C,/bit: 0xA0 = 10100000
    if AND(b7, AND(nb6, AND(b5, AND(nb4, AND(nb3, AND(nb2, AND(nb1, nb0))))))):
        return "ORL C,/bit"

    # MOV C,bit: 0xA2 = 10100010
    if AND(b7, AND(nb6, AND(b5, AND(nb4, AND(nb3, AND(nb2, AND(b1, nb0))))))):
        return "MOV C,bit"

    # INC DPTR: 0xA3 = 10100011
    if AND(b7, AND(nb6, AND(b5, AND(nb4, AND(nb3, AND(nb2, AND(b1, b0))))))):
        return "INC DPTR"

    # MUL AB: 0xA4 = 10100100
    if AND(b7, AND(nb6, AND(b5, AND(nb4, AND(nb3, AND(b2, AND(nb1, nb0))))))):
        return "MUL AB"

    # HALT is 0xA5, already handled above

    # MOV @R0,dir: 0xA6 = 10100110
    if AND(b7, AND(nb6, AND(b5, AND(nb4, AND(nb3, AND(b2, AND(b1, nb0))))))):
        return "MOV @R0,dir"

    # MOV @R1,dir: 0xA7 = 10100111
    if AND(b7, AND(nb6, AND(b5, AND(nb4, AND(nb3, AND(b2, AND(b1, b0))))))):
        return "MOV @R1,dir"

    # ANL C,/bit: 0xB0 = 10110000
    if AND(b7, AND(nb6, AND(b5, AND(b4, AND(nb3, AND(nb2, AND(nb1, nb0))))))):
        return "ANL C,/bit"

    # CPL bit: 0xB2 = 10110010
    if AND(b7, AND(nb6, AND(b5, AND(b4, AND(nb3, AND(nb2, AND(b1, nb0))))))):
        return "CPL bit"

    # CPL C: 0xB3 = 10110011
    if AND(b7, AND(nb6, AND(b5, AND(b4, AND(nb3, AND(nb2, AND(b1, b0))))))):
        return "CPL C"

    # CJNE A,#imm: 0xB4 = 10110100
    if AND(b7, AND(nb6, AND(b5, AND(b4, AND(nb3, AND(b2, AND(nb1, nb0))))))):
        return "CJNE A,#imm"

    # CJNE A,dir: 0xB5 = 10110101
    if AND(b7, AND(nb6, AND(b5, AND(b4, AND(nb3, AND(b2, AND(nb1, b0))))))):
        return "CJNE A,dir"

    # CJNE @R0,#imm: 0xB6 = 10110110
    if AND(b7, AND(nb6, AND(b5, AND(b4, AND(nb3, AND(b2, AND(b1, nb0))))))):
        return "CJNE @R0,#imm"

    # CJNE @R1,#imm: 0xB7 = 10110111
    if AND(b7, AND(nb6, AND(b5, AND(b4, AND(nb3, AND(b2, AND(b1, b0))))))):
        return "CJNE @R1,#imm"

    # PUSH dir: 0xC0 = 11000000
    if AND(b7, AND(b6, AND(nb5, AND(nb4, AND(nb3, AND(nb2, AND(nb1, nb0))))))):
        return "PUSH"

    # CLR bit: 0xC2 = 11000010
    if AND(b7, AND(b6, AND(nb5, AND(nb4, AND(nb3, AND(nb2, AND(b1, nb0))))))):
        return "CLR bit"

    # CLR C: 0xC3 = 11000011
    if AND(b7, AND(b6, AND(nb5, AND(nb4, AND(nb3, AND(nb2, AND(b1, b0))))))):
        return "CLR C"

    # SWAP A: 0xC4 = 11000100
    if AND(b7, AND(b6, AND(nb5, AND(nb4, AND(nb3, AND(b2, AND(nb1, nb0))))))):
        return "SWAP A"

    # XCH A,dir: 0xC5 = 11000101
    if AND(b7, AND(b6, AND(nb5, AND(nb4, AND(nb3, AND(b2, AND(nb1, b0))))))):
        return "XCH A,dir"

    # XCH A,@R0: 0xC6 = 11000110
    if AND(b7, AND(b6, AND(nb5, AND(nb4, AND(nb3, AND(b2, AND(b1, nb0))))))):
        return "XCH A,@R0"

    # XCH A,@R1: 0xC7 = 11000111
    if AND(b7, AND(b6, AND(nb5, AND(nb4, AND(nb3, AND(b2, AND(b1, b0))))))):
        return "XCH A,@R1"

    # DA A: 0xD4 = 11010100
    if AND(b7, AND(b6, AND(nb5, AND(b4, AND(nb3, AND(b2, AND(nb1, nb0))))))):
        return "DA A"

    # DJNZ dir: 0xD5 = 11010101
    if AND(b7, AND(b6, AND(nb5, AND(b4, AND(nb3, AND(b2, AND(nb1, b0))))))):
        return "DJNZ dir"

    # XCHD A,@R0: 0xD6 = 11010110
    if AND(b7, AND(b6, AND(nb5, AND(b4, AND(nb3, AND(b2, AND(b1, nb0))))))):
        return "XCHD A,@R0"

    # XCHD A,@R1: 0xD7 = 11010111
    if AND(b7, AND(b6, AND(nb5, AND(b4, AND(nb3, AND(b2, AND(b1, b0))))))):
        return "XCHD A,@R1"

    # POP dir: 0xD0 = 11010000
    if AND(b7, AND(b6, AND(nb5, AND(b4, AND(nb3, AND(nb2, AND(nb1, nb0))))))):
        return "POP"

    # SETB bit: 0xD2 = 11010010
    if AND(b7, AND(b6, AND(nb5, AND(b4, AND(nb3, AND(nb2, AND(b1, nb0))))))):
        return "SETB bit"

    # SETB C: 0xD3 = 11010011
    if AND(b7, AND(b6, AND(nb5, AND(b4, AND(nb3, AND(nb2, AND(b1, b0))))))):
        return "SETB C"

    # CLR A: 0xE4 = 11100100
    if AND(b7, AND(b6, AND(b5, AND(nb4, AND(nb3, AND(b2, AND(nb1, nb0))))))):
        return "CLR A"

    # MOV A,dir: 0xE5 = 11100101
    if AND(b7, AND(b6, AND(b5, AND(nb4, AND(nb3, AND(b2, AND(nb1, b0))))))):
        return "MOV A,dir"

    # MOV A,@R0: 0xE6 = 11100110
    if AND(b7, AND(b6, AND(b5, AND(nb4, AND(nb3, AND(b2, AND(b1, nb0))))))):
        return "MOV A,@R0"

    # MOV A,@R1: 0xE7 = 11100111
    if AND(b7, AND(b6, AND(b5, AND(nb4, AND(nb3, AND(b2, AND(b1, b0))))))):
        return "MOV A,@R1"

    # MOVX A,@DPTR: 0xE0 = 11100000
    if AND(b7, AND(b6, AND(b5, AND(nb4, AND(nb3, AND(nb2, AND(nb1, nb0))))))):
        return "MOVX A,@DPTR"

    # MOVX A,@R0: 0xE2 = 11100010
    if AND(b7, AND(b6, AND(b5, AND(nb4, AND(nb3, AND(nb2, AND(b1, nb0))))))):
        return "MOVX A,@R0"

    # MOVX A,@R1: 0xE3 = 11100011
    if AND(b7, AND(b6, AND(b5, AND(nb4, AND(nb3, AND(nb2, AND(b1, b0))))))):
        return "MOVX A,@R1"

    # MOVX @DPTR,A: 0xF0 = 11110000
    if AND(b7, AND(b6, AND(b5, AND(b4, AND(nb3, AND(nb2, AND(nb1, nb0))))))):
        return "MOVX @DPTR,A"

    # MOVX @R0,A: 0xF2 = 11110010
    if AND(b7, AND(b6, AND(b5, AND(b4, AND(nb3, AND(nb2, AND(b1, nb0))))))):
        return "MOVX @R0,A"

    # MOVX @R1,A: 0xF3 = 11110011
    if AND(b7, AND(b6, AND(b5, AND(b4, AND(nb3, AND(nb2, AND(b1, b0))))))):
        return "MOVX @R1,A"

    # CPL A: 0xF4 = 11110100
    if AND(b7, AND(b6, AND(b5, AND(b4, AND(nb3, AND(b2, AND(nb1, nb0))))))):
        return "CPL A"

    # MOV dir,A: 0xF5 = 11110101
    if AND(b7, AND(b6, AND(b5, AND(b4, AND(nb3, AND(b2, AND(nb1, b0))))))):
        return "MOV dir,A"

    # MOV @R0,A: 0xF6 = 11110110
    if AND(b7, AND(b6, AND(b5, AND(b4, AND(nb3, AND(b2, AND(b1, nb0))))))):
        return "MOV @R0,A"

    # MOV @R1,A: 0xF7 = 11110111
    if AND(b7, AND(b6, AND(b5, AND(b4, AND(nb3, AND(b2, AND(b1, b0))))))):
        return "MOV @R1,A"

    # ORL C,bit: 0x72 = 01110010
    if AND(nb7, AND(b6, AND(b5, AND(b4, AND(nb3, AND(nb2, AND(b1, nb0))))))):
        return "ORL C,bit"

    # JMP @A+DPTR: 0x73 = 01110011
    if AND(nb7, AND(b6, AND(b5, AND(b4, AND(nb3, AND(nb2, AND(b1, b0))))))):
        return "JMP @A+DPTR"

    # ANL C,bit: 0x82 = 10000010
    if AND(b7, AND(nb6, AND(nb5, AND(nb4, AND(nb3, AND(nb2, AND(b1, nb0))))))):
        return "ANL C,bit"

    # LJMP: 0x02 = 00000010
    if AND(nb7, AND(nb6, AND(nb5, AND(nb4, AND(nb3, AND(nb2, AND(b1, nb0))))))):
        return "LJMP"

    # =========================================================================
    # Family patterns — check ONLY bits[7:3] (or bits[7:1] for @Ri)
    # These must come AFTER all fully-specified patterns to avoid false matches
    # =========================================================================

    # INC Rn family: 0x08-0x0F — bits[7:3] = 00001
    # Condition: b7=0 b6=0 b5=0 b4=0 b3=1 (bits 2,1,0 select Rn)
    if AND(nb7, AND(nb6, AND(nb5, AND(nb4, b3)))):
        return "INC Rn"

    # DEC Rn family: 0x18-0x1F — bits[7:3] = 00011
    if AND(nb7, AND(nb6, AND(nb5, AND(b4, b3)))):
        return "DEC Rn"

    # ADD A,Rn family: 0x28-0x2F — bits[7:3] = 00101
    if AND(nb7, AND(nb6, AND(b5, AND(nb4, b3)))):
        return "ADD A,Rn"

    # ADDC A,Rn family: 0x38-0x3F — bits[7:3] = 00111
    if AND(nb7, AND(nb6, AND(b5, AND(b4, b3)))):
        return "ADDC A,Rn"

    # ORL A,Rn family: 0x48-0x4F — bits[7:3] = 01001
    if AND(nb7, AND(b6, AND(nb5, AND(nb4, b3)))):
        return "ORL A,Rn"

    # ANL A,Rn family: 0x58-0x5F — bits[7:3] = 01011
    if AND(nb7, AND(b6, AND(nb5, AND(b4, b3)))):
        return "ANL A,Rn"

    # XRL A,Rn family: 0x68-0x6F — bits[7:3] = 01101
    if AND(nb7, AND(b6, AND(b5, AND(nb4, b3)))):
        return "XRL A,Rn"

    # MOV Rn,#imm family: 0x78-0x7F — bits[7:3] = 01111
    if AND(nb7, AND(b6, AND(b5, AND(b4, b3)))):
        return "MOV Rn,#imm"

    # MOV dir,Rn family: 0x88-0x8F — bits[7:3] = 10001
    if AND(b7, AND(nb6, AND(nb5, AND(nb4, b3)))):
        return "MOV dir,Rn"

    # SUBB A,Rn family: 0x98-0x9F — bits[7:3] = 10011
    if AND(b7, AND(nb6, AND(nb5, AND(b4, b3)))):
        return "SUBB A,Rn"

    # MOV Rn,dir family: 0xA8-0xAF — bits[7:3] = 10101
    if AND(b7, AND(nb6, AND(b5, AND(nb4, b3)))):
        return "MOV Rn,dir"

    # CJNE Rn,#imm family: 0xB8-0xBF — bits[7:3] = 10111
    if AND(b7, AND(nb6, AND(b5, AND(b4, b3)))):
        return "CJNE Rn,#imm"

    # XCH A,Rn family: 0xC8-0xCF — bits[7:3] = 11001
    if AND(b7, AND(b6, AND(nb5, AND(nb4, b3)))):
        return "XCH A,Rn"

    # DJNZ Rn family: 0xD8-0xDF — bits[7:3] = 11011
    if AND(b7, AND(b6, AND(nb5, AND(b4, b3)))):
        return "DJNZ Rn"

    # MOV A,Rn family: 0xE8-0xEF — bits[7:3] = 11101
    if AND(b7, AND(b6, AND(b5, AND(nb4, b3)))):
        return "MOV A,Rn"

    # MOV Rn,A family: 0xF8-0xFF — bits[7:3] = 11111
    if AND(b7, AND(b6, AND(b5, AND(b4, b3)))):
        return "MOV Rn,A"

    # =========================================================================
    # AJMP and ACALL: bits[4:0] patterns (lowest 5 bits), upper 3 bits = page
    # Must come after all fully-specified patterns since they use partial match
    # =========================================================================

    # AJMP: bits[4:0] = 00001 (b4=0 b3=0 b2=0 b1=0 b0=1)
    if AND(nb4, AND(nb3, AND(nb2, AND(nb1, b0)))):
        return "AJMP"

    # ACALL: bits[4:0] = 10001 (b4=1 b3=0 b2=0 b1=0 b0=1)
    if AND(b4, AND(nb3, AND(nb2, AND(nb1, b0)))):
        return "ACALL"

    return "UNKNOWN"
