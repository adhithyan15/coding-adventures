"""decoder.py — Instruction decoder for the DEC Alpha AXP 21064.

The Alpha AXP uses four 32-bit fixed-width instruction formats:

  Memory format:   [op:6][Ra:5][Rb:5][disp16:16]
  Branch format:   [op:6][Ra:5][disp21:21]
  Operate format:  [op:6][Ra:5][Rb:5][0][sbz:4][func:7][Rc:5]  (i_bit=0)
                   [op:6][Ra:5][lit8:8][1][func:7][Rc:5]        (i_bit=1)
  Jump format:     [0x1A:6][Ra:5][Rb:5][func:2][hint:14]
  PALcode format:  [0x00:6][palcode:26]

Decoding is purely combinational — no state changes, no side effects.
The decoder extracts bit fields from the 32-bit instruction word.

Instruction format disambiguation
───────────────────────────────────
The opcode (bits 31:26) determines the format:
  op == 0x00 → PALcode
  op == 0x08, 0x09, 0x0B, 0x0F → Memory (LDA, LDAH, LDQ_U, STQ_U)
  op == 0x0A, 0x0C–0x0E → Memory (LDBU, LDWU, STW, STB, STW)
  op == 0x10–0x13 → Operate (integer arithmetic/logical/shift/multiply)
  op == 0x1A → Jump
  op == 0x28–0x2F → Memory (loads/stores)
  op == 0x30–0x3F → Branch

Signed displacement sign extension
─────────────────────────────────────
  disp16: 16-bit field, sign-extended to 64 bits for effective address
  disp21: 21-bit field, sign-extended and multiplied by 4 for branch target
"""

from __future__ import annotations

# ── Instruction format constants ───────────────────────────────────────────────

# Memory ops (op field values)
_MEMORY_OPS: frozenset[int] = frozenset({
    0x08,   # LDA
    0x09,   # LDAH
    0x0A,   # LDBU
    0x0B,   # LDQ_U
    0x0C,   # LDWU
    0x0D,   # STW
    0x0E,   # STB
    0x0F,   # STQ_U
    0x28,   # LDL
    0x29,   # LDQ
    0x2A,   # LDL_L
    0x2B,   # LDQ_L
    0x2C,   # STL
    0x2D,   # STQ
    0x2E,   # STL_C
    0x2F,   # STQ_C
})

# Operate group opcodes
_OPERATE_OPS: frozenset[int] = frozenset({0x10, 0x11, 0x12, 0x13})

# Branch opcodes
_BRANCH_OPS: frozenset[int] = frozenset({
    0x30, 0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37,
    0x38, 0x39, 0x3A, 0x3B, 0x3C, 0x3D, 0x3E, 0x3F,
})

# ── Mnemonic tables ────────────────────────────────────────────────────────────

# op=0x10: INTA (integer arithmetic + compare)
_INTA_MNEMONIC: dict[int, str] = {
    0x00: "ADDL", 0x02: "S4ADDL", 0x09: "SUBL",   0x0B: "S4SUBL",
    0x12: "S8ADDL", 0x1B: "S8SUBL",
    0x20: "ADDQ", 0x22: "S4ADDQ", 0x29: "SUBQ",   0x2B: "S4SUBQ",
    0x32: "S8ADDQ", 0x3B: "S8SUBQ",
    0x2D: "CMPEQ", 0x4D: "CMPLT", 0x6D: "CMPLE",
    0x3D: "CMPULT", 0x5D: "CMPULE", 0x7D: "CMPBGE",
    # Overflow variants — same mnemonic, different func
    0x40: "ADDLV", 0x49: "SUBLV", 0x60: "ADDQV", 0x69: "SUBQV",
    0x18: "MULL",  0x58: "MULLV", 0x38: "MULQ",  0x78: "MULQV",
    0x4B: "CMPBGE",
}

# op=0x11: INTL (integer logical + conditional moves)
_INTL_MNEMONIC: dict[int, str] = {
    0x00: "AND",     0x08: "BIC",     0x14: "CMOVLBS", 0x16: "CMOVLBC",
    0x20: "BIS",     0x24: "CMOVEQ",  0x26: "CMOVNE",  0x28: "ORNOT",
    0x40: "XOR",     0x44: "CMOVLT",  0x46: "CMOVGE",  0x48: "EQV",
    0x61: "AMASK",   0x64: "CMOVLE",  0x66: "CMOVGT",  0x6C: "IMPLVER",
}

# op=0x12: INTS (shift and byte manipulation)
_INTS_MNEMONIC: dict[int, str] = {
    0x00: "SEXTB",  0x01: "SEXTW",  0x02: "MSKBL",  0x06: "EXTBL",
    0x0B: "INSBL",  0x12: "MSKWL",  0x16: "EXTWL",  0x1B: "INSWL",
    0x22: "MSKLL",  0x26: "EXTLL",  0x2B: "INSLL",  0x30: "ZAP",
    0x31: "ZAPNOT", 0x32: "MSKQL",  0x34: "SRL",    0x36: "EXTQL",
    0x39: "SLL",    0x3A: "SRA",    0x3B: "INSQL",  0x3C: "SRA",
}

# op=0x13: INTM (integer multiply)
_INTM_MNEMONIC: dict[int, str] = {
    0x00: "MULL", 0x20: "MULQ", 0x30: "UMULH",
    0x40: "MULLV", 0x60: "MULQV",
}

# Memory opcode mnemonics
_MEM_MNEMONIC: dict[int, str] = {
    0x08: "LDA",   0x09: "LDAH",  0x0A: "LDBU",  0x0B: "LDQ_U",
    0x0C: "LDWU",  0x0D: "STW",   0x0E: "STB",   0x0F: "STQ_U",
    0x28: "LDL",   0x29: "LDQ",   0x2A: "LDL_L", 0x2B: "LDQ_L",
    0x2C: "STL",   0x2D: "STQ",   0x2E: "STL_C", 0x2F: "STQ_C",
}

# Branch opcode mnemonics
_BRANCH_MNEMONIC: dict[int, str] = {
    0x30: "BR",   0x31: "FBEQ",  0x32: "FBLT",  0x33: "FBLE",
    0x34: "BSR",  0x35: "FBNE",  0x36: "FBGE",  0x37: "FBGT",
    0x38: "BLBC", 0x39: "BEQ",   0x3A: "BLT",   0x3B: "BLE",
    0x3C: "BLBS", 0x3D: "BNE",   0x3E: "BGE",   0x3F: "BGT",
}

# Jump func code mnemonics
_JUMP_MNEMONIC: dict[int, str] = {
    0: "JMP", 1: "JSR", 2: "RET", 3: "JSR_COROUTINE",
}


# ── Sign-extension helpers ─────────────────────────────────────────────────────

def _sext16(v: int) -> int:
    """Sign-extend a 16-bit value to a Python int."""
    v = v & 0xFFFF
    if v >= 0x8000:
        v -= 0x10000
    return v


def _sext21(v: int) -> int:
    """Sign-extend a 21-bit branch displacement to a Python int."""
    v = v & 0x1F_FFFF
    if v >= 0x10_0000:
        v -= 0x20_0000
    return v


# ── Main decode function ───────────────────────────────────────────────────────

def decode_instruction(word: int) -> dict:
    """Decode a 32-bit Alpha AXP instruction word into its fields.

    Returns a dict with keys:
      op       : 6-bit opcode (bits 31:26)
      ra       : 5-bit Ra field (bits 25:21)
      rb       : 5-bit Rb field (bits 20:16)
      rc       : 5-bit Rc field (bits 4:0) — operate format destination
      func7    : 7-bit function code (bits 11:5) — operate format
      i_bit    : bit 12 — 0=register operand, 1=literal operand
      lit8     : 8-bit zero-extended literal (bits 20:13) — when i_bit=1
      disp16   : signed 16-bit displacement — memory format
      disp21   : signed 21-bit branch offset — branch format
      jump_func: 2-bit jump function code (bits 15:14) — jump format
      palcode  : 26-bit PALcode value (bits 25:0) — PALcode format
      mnemonic : human-readable instruction name

    All fields are always present in the returned dict.  Fields irrelevant
    to the current instruction format will contain default/zero values.

    Examples
    ────────
    >>> d = decode_instruction(0x00000000)  # HALT
    >>> d['mnemonic']
    'HALT'
    >>> d = decode_instruction((0x08 << 26) | (1 << 21) | (2 << 16) | 100)  # LDA r1, 100(r2)
    >>> d['mnemonic']
    'LDA'
    >>> d['ra']
    1
    >>> d['disp16']
    100
    """
    word = word & 0xFFFF_FFFF

    op       = (word >> 26) & 0x3F    # bits 31:26
    ra       = (word >> 21) & 0x1F    # bits 25:21
    rb       = (word >> 16) & 0x1F    # bits 20:16
    rc       =  word        & 0x1F    # bits 4:0
    func7    = (word >>  5) & 0x7F    # bits 11:5
    i_bit    = (word >> 12) & 0x1     # bit 12
    lit8     = (word >> 13) & 0xFF    # bits 20:13
    disp16   = _sext16(word & 0xFFFF) # bits 15:0, signed
    disp21   = _sext21(word & 0x1F_FFFF)  # bits 20:0, signed
    jump_func = (word >> 14) & 0x3    # bits 15:14
    palcode  = word & 0x03FF_FFFF     # bits 25:0

    # Determine mnemonic
    if op == 0x00:
        mnemonic = "HALT" if palcode == 0 else f"PAL_{palcode:#010x}"
    elif op == 0x1A:
        mnemonic = _JUMP_MNEMONIC.get(jump_func, f"JMP_{jump_func}")
    elif op in _MEMORY_OPS:
        mnemonic = _MEM_MNEMONIC.get(op, f"MEM_0x{op:02X}")
    elif op in _BRANCH_OPS:
        mnemonic = _BRANCH_MNEMONIC.get(op, f"BR_0x{op:02X}")
    elif op == 0x10:
        mnemonic = _INTA_MNEMONIC.get(func7, f"INTA_0x{func7:02X}")
    elif op == 0x11:
        mnemonic = _INTL_MNEMONIC.get(func7, f"INTL_0x{func7:02X}")
    elif op == 0x12:
        mnemonic = _INTS_MNEMONIC.get(func7, f"INTS_0x{func7:02X}")
    elif op == 0x13:
        mnemonic = _INTM_MNEMONIC.get(func7, f"INTM_0x{func7:02X}")
    else:
        mnemonic = f"OP_0x{op:02X}"

    return {
        "op":        op,
        "ra":        ra,
        "rb":        rb,
        "rc":        rc,
        "func7":     func7,
        "i_bit":     i_bit,
        "lit8":      lit8,
        "disp16":    disp16,
        "disp21":    disp21,
        "jump_func": jump_func,
        "palcode":   palcode,
        "mnemonic":  mnemonic,
    }
