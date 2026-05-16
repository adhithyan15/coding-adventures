"""decoder.py — MIPS R2000 instruction decoder using gate-level field extraction.

MIPS R2000 instruction formats
────────────────────────────────

Three fixed-width 32-bit formats:

R-type (register):
  ┌────────┬────┬────┬────┬───────┬───────┐
  │ op (6) │rs(5)│rt(5)│rd(5)│shamt(5)│funct(6)│
  └────────┴────┴────┴────┴───────┴───────┘
  Bits: 31..26  25..21  20..16  15..11  10..6  5..0
  Always op=0; funct identifies the instruction.

I-type (immediate):
  ┌────────┬────┬────┬─────────────────┐
  │ op (6) │rs(5)│rt(5)│   imm16 (16)  │
  └────────┴────┴────┴─────────────────┘

J-type (jump):
  ┌────────┬───────────────────────────┐
  │ op (6) │        target26 (26)      │
  └────────┴───────────────────────────┘

Gate-level field extraction
────────────────────────────
Real hardware uses AND gates to mask out bit fields, and shift registers
(or wire re-ordering) to right-justify each field.  We model this by
extracting bit sub-lists from the word's bit representation, then
converting back to integers.

All extraction is done on the LSB-first bit list representation of the
instruction word.

Sign extension of imm16
────────────────────────
For I-type instructions, imm16 is sign-extended to 32 bits.  In hardware,
this is done by a sign-extension circuit: bit 15 of the immediate is
replicated into bits 16..31.  We implement this by checking bit 15 (which
is at index 15 in the LSB-first list) and filling upper bits accordingly.

Mnemonic lookup
───────────────
The mnemonic is a human-readable name (e.g., "ADD", "BEQ", "J") looked up
from a dispatch table keyed by (format, op, funct/rt).  Unknown opcodes
return mnemonic "UNKNOWN".
"""

from __future__ import annotations

from .bits import bits_to_int, int_to_bits

# ── Mnemonic lookup tables ─────────────────────────────────────────────────────

# R-type: keyed by funct code
_R_FUNCT_MNEMONICS: dict[int, str] = {
    0x00: "SLL",
    0x02: "SRL",
    0x03: "SRA",
    0x04: "SLLV",
    0x06: "SRLV",
    0x07: "SRAV",
    0x08: "JR",
    0x09: "JALR",
    0x0C: "SYSCALL",
    0x0D: "BREAK",
    0x10: "MFHI",
    0x11: "MTHI",
    0x12: "MFLO",
    0x13: "MTLO",
    0x18: "MULT",
    0x19: "MULTU",
    0x1A: "DIV",
    0x1B: "DIVU",
    0x20: "ADD",
    0x21: "ADDU",
    0x22: "SUB",
    0x23: "SUBU",
    0x24: "AND",
    0x25: "OR",
    0x26: "XOR",
    0x27: "NOR",
    0x2A: "SLT",
    0x2B: "SLTU",
}

# REGIMM (op=0x01): keyed by rt field
_REGIMM_RT_MNEMONICS: dict[int, str] = {
    0x00: "BLTZ",
    0x01: "BGEZ",
    0x10: "BLTZAL",
    0x11: "BGEZAL",
}

# I-type and J-type: keyed by op code
_OP_MNEMONICS: dict[int, str] = {
    0x02: "J",
    0x03: "JAL",
    0x04: "BEQ",
    0x05: "BNE",
    0x06: "BLEZ",
    0x07: "BGTZ",
    0x08: "ADDI",
    0x09: "ADDIU",
    0x0A: "SLTI",
    0x0B: "SLTIU",
    0x0C: "ANDI",
    0x0D: "ORI",
    0x0E: "XORI",
    0x0F: "LUI",
    0x20: "LB",
    0x21: "LH",
    0x22: "LWL",
    0x23: "LW",
    0x24: "LBU",
    0x25: "LHU",
    0x26: "LWR",
    0x28: "SB",
    0x29: "SH",
    0x2A: "SWL",
    0x2B: "SW",
    0x2E: "SWR",
}


# ── Decoder ────────────────────────────────────────────────────────────────────


def decode_instruction(word: int) -> dict:
    """Decode a 32-bit MIPS instruction word into its constituent fields.

    Uses gate-level bit extraction: the instruction word is converted to an
    LSB-first bit list, sub-lists are extracted for each field, then
    converted back to integers.

    Field layout (all fields extracted from bit slices):
      bits[31:26] → op (6 bits)
      bits[25:21] → rs (5 bits)
      bits[20:16] → rt (5 bits)
      bits[15:11] → rd (5 bits, R-type only)
      bits[10:6]  → shamt (5 bits, R-type only)
      bits[5:0]   → funct (6 bits, R-type only)
      bits[15:0]  → imm16 (16 bits, I-type, sign-extended)
      bits[25:0]  → target26 (26 bits, J-type)

    Args:
        word: 32-bit instruction word (unsigned).

    Returns:
        Dictionary with keys:
          format    — 'R', 'I', or 'J'
          op        — 6-bit opcode
          rs        — 5-bit source register field
          rt        — 5-bit target register field
          rd        — 5-bit destination register field (R-type) or 0
          shamt     — 5-bit shift amount (R-type) or 0
          funct     — 6-bit function code (R-type) or 0
          imm16     — sign-extended 16-bit immediate (I-type) or 0
          target26  — 26-bit jump target (J-type) or 0
          mnemonic  — human-readable instruction name

    Example:
        Decoding ADD $t0, $t1, $t2 (R-type):
          word = 0x012A4020
          op=0, rs=9($t1), rt=10($t2), rd=8($t0), shamt=0, funct=0x20
          format='R', mnemonic='ADD'
    """
    # Convert the 32-bit word to an LSB-first bit list for gate-level extraction.
    # In hardware, the 32 wires carrying the instruction bits are directly
    # connected to the decoder logic — no "conversion" happens.  Here we
    # model those wires as a Python list.
    bits = int_to_bits(word, 32)

    # Extract the 6-bit opcode from bits [31:26] (indices 26..31 in LSB-first)
    op_bits = bits[26:32]
    op = bits_to_int(op_bits)

    # Extract rs: bits [25:21] → indices 21..25
    rs_bits = bits[21:26]
    rs = bits_to_int(rs_bits)

    # Extract rt: bits [20:16] → indices 16..20
    rt_bits = bits[16:21]
    rt = bits_to_int(rt_bits)

    # Common to all formats
    result: dict = {
        "op": op,
        "rs": rs,
        "rt": rt,
        "rd": 0,
        "shamt": 0,
        "funct": 0,
        "imm16": 0,
        "target26": 0,
        "format": "I",
        "mnemonic": "UNKNOWN",
    }

    if op == 0:
        # R-type instruction
        rd_bits = bits[11:16]   # bits [15:11]
        shamt_bits = bits[6:11]  # bits [10:6]
        funct_bits = bits[0:6]   # bits [5:0]

        rd = bits_to_int(rd_bits)
        shamt = bits_to_int(shamt_bits)
        funct = bits_to_int(funct_bits)

        result["format"] = "R"
        result["rd"] = rd
        result["shamt"] = shamt
        result["funct"] = funct
        result["mnemonic"] = _R_FUNCT_MNEMONICS.get(funct, "UNKNOWN")

    elif op == 0x01:
        # REGIMM — I-type with special dispatch on rt field
        imm16 = _sign_extend_imm16(bits)
        result["format"] = "I"
        result["imm16"] = imm16
        result["mnemonic"] = _REGIMM_RT_MNEMONICS.get(rt, "UNKNOWN")

    elif op in (0x02, 0x03):
        # J-type: bits [25:0]
        target_bits = bits[0:26]
        target26 = bits_to_int(target_bits)
        result["format"] = "J"
        result["target26"] = target26
        result["mnemonic"] = _OP_MNEMONICS.get(op, "UNKNOWN")

    else:
        # I-type
        imm16 = _sign_extend_imm16(bits)
        result["format"] = "I"
        result["imm16"] = imm16
        result["mnemonic"] = _OP_MNEMONICS.get(op, "UNKNOWN")

    return result


def _sign_extend_imm16(bits: list[int]) -> int:
    """Extract and sign-extend a 16-bit immediate from an instruction's bit list.

    The 16-bit immediate occupies bits[15:0] (indices 0..15 in LSB-first).
    Sign extension copies bit 15 into bits 16..31.

    In hardware, the sign extension circuit is just 16 wires from bit 15
    fanned out to fill positions 16–31.  No gates needed — just wire routing.

    Args:
        bits: 32-element LSB-first bit list for the full instruction word.

    Returns:
        Sign-extended value as a Python int (may be negative).
    """
    imm_bits = bits[0:16]  # lower 16 bits (LSB-first)
    sign_bit = imm_bits[15]  # bit 15 is the sign
    # Sign-extend: fill upper 16 positions with the sign bit
    extended_bits = imm_bits + [sign_bit] * 16
    unsigned_val = bits_to_int(extended_bits)
    # Convert to signed Python int for use in branch offset calculations
    if sign_bit == 1:
        return unsigned_val - 0x1_0000_0000
    return unsigned_val
