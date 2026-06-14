"""Instruction decoder for the Intel 8086 gate-level simulator.

=== The 8086 Instruction Encoding ===

The 8086 uses a variable-length encoding, 1–6 bytes per instruction.
The general pattern:

    [prefix*]  OPCODE  [ModRM]  [disp8|disp16]  [imm8|imm16]

OPCODE byte often encodes:
    bit 1 (d): direction — 0: r/m is destination
    1: reg is destination
    bit 0 (w): width     — 0: byte (8-bit)
    1: word (16-bit)

=== ModRM byte ===

    mod[7:6]  reg[5:3]  r/m[2:0]

    mod=00: indirect via effective address table
            Exception: r/m=110 → direct [disp16] addressing
    mod=01: indirect + 8-bit signed displacement
    mod=10: indirect + 16-bit signed displacement
    mod=11: register-to-register (r/m is a register index)

=== Effective address (EA) base table ===

    r/m=000 → BX + SI    r/m=100 → SI
    r/m=001 → BX + DI    r/m=101 → DI
    r/m=010 → BP + SI    r/m=110 → BP (or [disp16] if mod=00)
    r/m=011 → BP + DI    r/m=111 → BX

=== Segment override prefixes ===

    0x26: ES:    0x2E: CS:    0x36: SS:    0x3E: DS:

=== This module's role ===

The decoder is used by tests and is a conceptual separation between
instruction fetch/decode and execution.  The simulator itself inlines
decoding for performance, but this module provides a clean decode
interface for testing and analysis.
"""

from __future__ import annotations

from dataclasses import dataclass, field


@dataclass
class DecodedInstr:
    """A decoded 8086 instruction.

    Fields:
        mnemonic:    Instruction name string (e.g. "MOV", "ADD").
        length:      Total bytes consumed (including opcode, ModRM, disp, imm).
        word:        True if 16-bit operation
        False if 8-bit.
        mod:         ModRM mod field (0–3), or -1 if no ModRM.
        reg:         ModRM reg field (0–7), or -1 if no ModRM.
        rm:          ModRM r/m field (0–7), or -1 if no ModRM.
        disp:        Displacement value (signed), or 0.
        imm:         Immediate value, or 0.
        seg_override: Segment register value to use, or None.
        rep_prefix:  0xF3 (REP/REPE), 0xF2 (REPNE), or None.
        opcode:      Raw opcode byte.
        has_modrm:   True if a ModRM byte was consumed.
    """

    mnemonic: str = ""
    length: int = 1
    word: bool = False
    mod: int = -1
    reg: int = -1
    rm: int = -1
    disp: int = 0
    imm: int = 0
    seg_override: int | None = None
    rep_prefix: int | None = None
    opcode: int = 0
    has_modrm: bool = False
    extra: dict = field(default_factory=dict)


_REG8_NAMES = ["AL", "CL", "DL", "BL", "AH", "CH", "DH", "BH"]
_REG16_NAMES = ["AX", "CX", "DX", "BX", "SP", "BP", "SI", "DI"]
_SREG_NAMES = ["ES", "CS", "SS", "DS"]
_EA_NAMES = ["BX+SI", "BX+DI", "BP+SI", "BP+DI", "SI", "DI", "BP", "BX"]

_ALU_NAMES = ["ADD", "OR", "ADC", "SBB", "AND", "SUB", "XOR", "CMP"]
_SHIFT_NAMES = {0: "ROL", 1: "ROR", 2: "RCL", 3: "RCR", 4: "SHL", 5: "SHR",
                6: "SHL", 7: "SAR"}
_JCC_NAMES = ["JO", "JNO", "JB", "JNB", "JZ", "JNZ", "JBE", "JA",
              "JS", "JNS", "JP", "JNP", "JL", "JGE", "JLE", "JG"]


def decode_instruction(
    memory: bytes | bytearray,
    cs: int,
    ip: int,
) -> DecodedInstr:
    """Decode the instruction at CS:IP in memory.

    Reads bytes from the flat memory array using the physical address
    CS×16 + IP (mod 0xFFFFF).  Handles all prefix bytes, ModRM, displacement,
    and immediate values.

    Args:
        memory:  Flat 1 MB memory array.
        cs:      Code segment register value.
        ip:      Instruction pointer (offset within CS).

    Returns:
        DecodedInstr with mnemonic, length, and decoded fields.

    Examples:
        >>> mem = bytearray(0x10000)
        >>> mem[0] = 0xB8
        mem[1] = 0x34
        mem[2] = 0x12  # MOV AX, 0x1234
        >>> d = decode_instruction(mem, 0, 0)
        >>> d.mnemonic
        'MOV'
        >>> d.length
        3
    """
    phys_mask = 0xFFFFF
    seg_shift = cs << 4

    # Local fetch helpers (no side effects on IP)
    pos = 0  # bytes consumed so far

    def fetch8() -> int:
        nonlocal pos
        addr = (seg_shift + ip + pos) & phys_mask
        pos += 1
        return memory[addr]

    def fetch16() -> int:
        lo = fetch8()
        hi = fetch8()
        return lo | (hi << 8)

    def fetch_s8() -> int:
        v = fetch8()
        return v if v < 0x80 else v - 0x100

    def fetch_s16() -> int:
        v = fetch16()
        return v if v < 0x8000 else v - 0x10000

    def decode_modrm_fields(modrm: int, word: bool) -> tuple[int, int, int]:
        """Return (mod, reg, rm) from a ModRM byte."""
        mod = (modrm >> 6) & 3
        reg = (modrm >> 3) & 7
        rm = modrm & 7
        return mod, reg, rm

    def consume_modrm_disp(mod: int, rm: int) -> int:
        """Consume displacement bytes for a ModRM, return displacement."""
        if mod == 3:
            return 0
        if mod == 0 and rm == 6:
            return fetch16()  # direct address
        if mod == 1:
            return fetch_s8()
        if mod == 2:
            return fetch_s16()
        return 0

    d = DecodedInstr()
    seg_override: int | None = None
    rep_prefix: int | None = None

    # Prefix loop
    while True:
        op = fetch8()
        if op == 0x26:
            seg_override = 0  # ES:
        elif op == 0x2E:
            seg_override = 1  # CS:
        elif op == 0x36:
            seg_override = 2  # SS:
        elif op == 0x3E:
            seg_override = 3  # DS:
        elif op in (0xF2, 0xF3):
            rep_prefix = op
        elif op == 0xF0:
            pass  # LOCK ignored
        else:
            break

    d.opcode = op
    d.seg_override = seg_override
    d.rep_prefix = rep_prefix

    # ── Decode by opcode ──────────────────────────────────────────────────────

    # MOV r/m, reg or reg, r/m (88–8B)
    if op in (0x88, 0x89, 0x8A, 0x8B):
        word = bool(op & 1)
        d_bit = bool(op & 2)
        modrm = fetch8()
        mod, reg, rm = decode_modrm_fields(modrm, word)
        disp = consume_modrm_disp(mod, rm)
        _REG16_NAMES[reg] if word else _REG8_NAMES[reg]
        d.mnemonic = "MOV"
        d.word = word
        d.mod = mod
        d.reg = reg
        d.rm = rm
        d.disp = disp
        d.has_modrm = True
        d.extra["d"] = d_bit

    # MOV r/m8, imm8 (C6)
    elif op == 0xC6:
        modrm = fetch8()
        mod, reg, rm = decode_modrm_fields(modrm, False)
        disp = consume_modrm_disp(mod, rm)
        imm = fetch8()
        d.mnemonic = "MOV"
        d.word = False
        d.mod = mod
        d.reg = reg
        d.rm = rm
        d.disp = disp
        d.imm = imm
        d.has_modrm = True

    # MOV r/m16, imm16 (C7)
    elif op == 0xC7:
        modrm = fetch8()
        mod, reg, rm = decode_modrm_fields(modrm, True)
        disp = consume_modrm_disp(mod, rm)
        imm = fetch16()
        d.mnemonic = "MOV"
        d.word = True
        d.mod = mod
        d.reg = reg
        d.rm = rm
        d.disp = disp
        d.imm = imm
        d.has_modrm = True

    # MOV reg8, imm8 (B0–B7)
    elif 0xB0 <= op <= 0xB7:
        reg = op - 0xB0
        imm = fetch8()
        d.mnemonic = "MOV"
        d.word = False
        d.reg = reg
        d.imm = imm
        d.extra["reg_name"] = _REG8_NAMES[reg]

    # MOV reg16, imm16 (B8–BF)
    elif 0xB8 <= op <= 0xBF:
        reg = op - 0xB8
        imm = fetch16()
        d.mnemonic = "MOV"
        d.word = True
        d.reg = reg
        d.imm = imm
        d.extra["reg_name"] = _REG16_NAMES[reg]

    # MOV AL/AX, [addr] (A0/A1)
    elif op in (0xA0, 0xA1) or op in (0xA2, 0xA3):
        word = bool(op & 1)
        addr = fetch16()
        d.mnemonic = "MOV"
        d.word = word
        d.disp = addr

    # MOV r/m, sreg (8C)
    elif op == 0x8C or op == 0x8E:
        modrm = fetch8()
        mod, reg, rm = decode_modrm_fields(modrm, True)
        disp = consume_modrm_disp(mod, rm)
        d.mnemonic = "MOV"
        d.word = True
        d.mod = mod
        d.reg = reg
        d.rm = rm
        d.disp = disp
        d.has_modrm = True

    # XCHG AX, reg (90–97); 90 = NOP
    elif 0x90 <= op <= 0x97:
        reg = op - 0x90
        d.mnemonic = "NOP" if reg == 0 else "XCHG"
        d.reg = reg
        d.word = True

    # XCHG r/m, reg (86/87)
    elif op in (0x86, 0x87):
        word = bool(op & 1)
        modrm = fetch8()
        mod, reg, rm = decode_modrm_fields(modrm, word)
        disp = consume_modrm_disp(mod, rm)
        d.mnemonic = "XCHG"
        d.word = word
        d.mod = mod
        d.reg = reg
        d.rm = rm
        d.disp = disp
        d.has_modrm = True

    # PUSH reg (50–57)
    elif 0x50 <= op <= 0x57:
        d.mnemonic = "PUSH"
        d.reg = op - 0x50
        d.word = True

    # POP reg (58–5F)
    elif 0x58 <= op <= 0x5F:
        d.mnemonic = "POP"
        d.reg = op - 0x58
        d.word = True

    # PUSH sreg
    elif op in (0x06, 0x0E, 0x16, 0x1E):
        d.mnemonic = "PUSH"
        d.extra["sreg"] = {0x06: 0, 0x0E: 1, 0x16: 2, 0x1E: 3}[op]

    # POP sreg
    elif op in (0x07, 0x17, 0x1F):
        d.mnemonic = "POP"
        d.extra["sreg"] = {0x07: 0, 0x17: 2, 0x1F: 3}[op]

    # POP r/m (8F)
    elif op == 0x8F:
        modrm = fetch8()
        mod, reg, rm = decode_modrm_fields(modrm, True)
        disp = consume_modrm_disp(mod, rm)
        d.mnemonic = "POP"
        d.word = True
        d.mod = mod
        d.rm = rm
        d.has_modrm = True

    # PUSHF / POPF
    elif op == 0x9C:
        d.mnemonic = "PUSHF"
    elif op == 0x9D:
        d.mnemonic = "POPF"

    # LEA (8D)
    elif op == 0x8D:
        modrm = fetch8()
        mod, reg, rm = decode_modrm_fields(modrm, True)
        disp = consume_modrm_disp(mod, rm)
        d.mnemonic = "LEA"
        d.word = True
        d.mod = mod
        d.reg = reg
        d.rm = rm
        d.disp = disp
        d.has_modrm = True

    # LDS (C5) / LES (C4)
    elif op in (0xC4, 0xC5):
        modrm = fetch8()
        mod, reg, rm = decode_modrm_fields(modrm, True)
        disp = consume_modrm_disp(mod, rm)
        d.mnemonic = "LDS" if op == 0xC5 else "LES"
        d.word = True
        d.mod = mod
        d.reg = reg
        d.rm = rm
        d.disp = disp
        d.has_modrm = True

    # LAHF / SAHF
    elif op == 0x9F:
        d.mnemonic = "LAHF"
    elif op == 0x9E:
        d.mnemonic = "SAHF"

    # CBW / CWD
    elif op == 0x98:
        d.mnemonic = "CBW"
    elif op == 0x99:
        d.mnemonic = "CWD"

    # XLAT (D7)
    elif op == 0xD7:
        d.mnemonic = "XLAT"

    # 80-group ALU ops
    elif op in (0x80, 0x81, 0x82, 0x83):
        word = op == 0x81 or op == 0x83
        modrm = fetch8()
        mod, ext, rm = decode_modrm_fields(modrm, word)
        disp = consume_modrm_disp(mod, rm)
        if op in (0x80, 0x82):
            imm = fetch8()
        elif op == 0x81:
            imm = fetch16()
        else:
            v = fetch8()
            imm = v if v < 0x80 else v - 0x100
        d.mnemonic = _ALU_NAMES[ext]
        d.word = word
        d.mod = mod
        d.reg = ext
        d.rm = rm
        d.disp = disp
        d.imm = imm
        d.has_modrm = True

    # TEST r/m, reg (84/85)
    elif op in (0x84, 0x85):
        word = bool(op & 1)
        modrm = fetch8()
        mod, reg, rm = decode_modrm_fields(modrm, word)
        disp = consume_modrm_disp(mod, rm)
        d.mnemonic = "TEST"
        d.word = word
        d.mod = mod
        d.reg = reg
        d.rm = rm
        d.disp = disp
        d.has_modrm = True

    # Standard ALU r/m ↔ reg (00–3F pairs)
    elif op in (
        0x00, 0x01, 0x02, 0x03,
        0x08, 0x09, 0x0A, 0x0B,
        0x10, 0x11, 0x12, 0x13,
        0x18, 0x19, 0x1A, 0x1B,
        0x20, 0x21, 0x22, 0x23,
        0x28, 0x29, 0x2A, 0x2B,
        0x30, 0x31, 0x32, 0x33,
        0x38, 0x39, 0x3A, 0x3B,
    ):
        alu_op = (op >> 3) & 7
        word = bool(op & 1)
        modrm = fetch8()
        mod, reg, rm = decode_modrm_fields(modrm, word)
        disp = consume_modrm_disp(mod, rm)
        d.mnemonic = _ALU_NAMES[alu_op]
        d.word = word
        d.mod = mod
        d.reg = reg
        d.rm = rm
        d.disp = disp
        d.has_modrm = True
        d.extra["d"] = bool(op & 2)

    # Accumulator-imm ALU ops
    elif op in (
        0x04, 0x05, 0x0C, 0x0D, 0x14, 0x15, 0x1C, 0x1D,
        0x24, 0x25, 0x2C, 0x2D, 0x34, 0x35, 0x3C, 0x3D,
        0xA8, 0xA9,
    ):
        alu_op_map = {
            0x04: 0, 0x05: 0, 0x0C: 1, 0x0D: 1,
            0x14: 2, 0x15: 2, 0x1C: 3, 0x1D: 3,
            0x24: 4, 0x25: 4, 0x2C: 5, 0x2D: 5,
            0x34: 6, 0x35: 6, 0x3C: 7, 0x3D: 7,
            0xA8: 4, 0xA9: 4,
        }
        word = bool(op & 1)
        alu_op = alu_op_map[op]
        imm = fetch16() if word else fetch8()
        d.mnemonic = _ALU_NAMES[alu_op] if op not in (0xA8, 0xA9) else "TEST"
        d.word = word
        d.imm = imm

    # INC reg16 (40–47)
    elif 0x40 <= op <= 0x47:
        d.mnemonic = "INC"
        d.reg = op - 0x40
        d.word = True

    # DEC reg16 (48–4F)
    elif 0x48 <= op <= 0x4F:
        d.mnemonic = "DEC"
        d.reg = op - 0x48
        d.word = True

    # FE group: INC/DEC r/m8
    elif op == 0xFE:
        modrm = fetch8()
        mod, ext, rm = decode_modrm_fields(modrm, False)
        disp = consume_modrm_disp(mod, rm)
        d.mnemonic = "INC" if ext == 0 else "DEC"
        d.word = False
        d.mod = mod
        d.rm = rm
        d.disp = disp
        d.has_modrm = True

    # FF group: INC/DEC/CALL/JMP/PUSH r/m16
    elif op == 0xFF:
        modrm = fetch8()
        mod, ext, rm = decode_modrm_fields(modrm, True)
        disp = consume_modrm_disp(mod, rm)
        ff_names = {0: "INC", 1: "DEC", 2: "CALL", 3: "CALL",
                    4: "JMP", 5: "JMP", 6: "PUSH"}
        d.mnemonic = ff_names.get(ext, "UNKNOWN")
        d.word = True
        d.mod = mod
        d.reg = ext
        d.rm = rm
        d.disp = disp
        d.has_modrm = True

    # F6/F7 group: TEST/NOT/NEG/MUL/IMUL/DIV/IDIV
    elif op in (0xF6, 0xF7):
        word = bool(op & 1)
        modrm = fetch8()
        mod, ext, rm = decode_modrm_fields(modrm, word)
        disp = consume_modrm_disp(mod, rm)
        if ext == 0:
            imm = fetch16() if word else fetch8()
            d.imm = imm
        f_names = {0: "TEST", 2: "NOT", 3: "NEG", 4: "MUL",
                   5: "IMUL", 6: "DIV", 7: "IDIV"}
        d.mnemonic = f_names.get(ext, "UNKNOWN")
        d.word = word
        d.mod = mod
        d.reg = ext
        d.rm = rm
        d.disp = disp
        d.has_modrm = True

    # BCD / ASCII adjust
    elif op == 0x27:
        d.mnemonic = "DAA"
    elif op == 0x2F:
        d.mnemonic = "DAS"
    elif op == 0x37:
        d.mnemonic = "AAA"
    elif op == 0x3F:
        d.mnemonic = "AAS"
    elif op == 0xD4:
        fetch8()
        d.mnemonic = "AAM"
    elif op == 0xD5:
        fetch8()
        d.mnemonic = "AAD"

    # Shifts/rotates D0–D3
    elif op in (0xD0, 0xD1, 0xD2, 0xD3):
        word = bool(op & 1)
        modrm = fetch8()
        mod, ext, rm = decode_modrm_fields(modrm, word)
        disp = consume_modrm_disp(mod, rm)
        d.mnemonic = _SHIFT_NAMES[ext]
        d.word = word
        d.mod = mod
        d.reg = ext
        d.rm = rm
        d.disp = disp
        d.has_modrm = True
        d.extra["use_cl"] = op >= 0xD2

    # JMP short (EB)
    elif op == 0xEB:
        disp = fetch_s8()
        d.mnemonic = "JMP"
        d.disp = disp

    # JMP near (E9)
    elif op == 0xE9:
        disp = fetch_s16()
        d.mnemonic = "JMP"
        d.disp = disp

    # JMP far (EA)
    elif op == 0xEA:
        new_ip = fetch16()
        new_cs = fetch16()
        d.mnemonic = "JMP"
        d.extra["far_ip"] = new_ip
        d.extra["far_cs"] = new_cs

    # CALL near (E8)
    elif op == 0xE8:
        disp = fetch_s16()
        d.mnemonic = "CALL"
        d.disp = disp

    # CALL far (9A)
    elif op == 0x9A:
        new_ip = fetch16()
        new_cs = fetch16()
        d.mnemonic = "CALL"
        d.extra["far_ip"] = new_ip
        d.extra["far_cs"] = new_cs

    # RET variants
    elif op == 0xC3:
        d.mnemonic = "RET"
    elif op == 0xC2:
        d.mnemonic = "RET"
        d.imm = fetch16()
    elif op == 0xCB:
        d.mnemonic = "RETF"
    elif op == 0xCA:
        d.mnemonic = "RETF"
        d.imm = fetch16()

    # Jcc (70–7F)
    elif 0x70 <= op <= 0x7F:
        disp = fetch_s8()
        d.mnemonic = _JCC_NAMES[op - 0x70]
        d.disp = disp

    # LOOP variants (E0–E3)
    elif op in (0xE0, 0xE1, 0xE2, 0xE3):
        disp = fetch_s8()
        ln = {0xE0: "LOOPNE", 0xE1: "LOOPE", 0xE2: "LOOP", 0xE3: "JCXZ"}
        d.mnemonic = ln[op]
        d.disp = disp

    # INT / IRET
    elif op == 0xCC:
        d.mnemonic = "INT3"
    elif op == 0xCE:
        d.mnemonic = "INTO"
    elif op == 0xCD:
        d.mnemonic = "INT"
        d.imm = fetch8()
    elif op == 0xCF:
        d.mnemonic = "IRET"

    # String ops
    elif op in (0xA4, 0xA5):
        d.mnemonic = "MOVS"
        d.word = bool(op & 1)
    elif op in (0xA6, 0xA7):
        d.mnemonic = "CMPS"
        d.word = bool(op & 1)
    elif op in (0xAE, 0xAF):
        d.mnemonic = "SCAS"
        d.word = bool(op & 1)
    elif op in (0xAC, 0xAD):
        d.mnemonic = "LODS"
        d.word = bool(op & 1)
    elif op in (0xAA, 0xAB):
        d.mnemonic = "STOS"
        d.word = bool(op & 1)

    # Misc
    elif op == 0xF4:
        d.mnemonic = "HLT"
    elif op == 0xF8:
        d.mnemonic = "CLC"
    elif op == 0xF9:
        d.mnemonic = "STC"
    elif op == 0xF5:
        d.mnemonic = "CMC"
    elif op == 0xFC:
        d.mnemonic = "CLD"
    elif op == 0xFD:
        d.mnemonic = "STD"
    elif op == 0xFA:
        d.mnemonic = "CLI"
    elif op == 0xFB:
        d.mnemonic = "STI"
    elif op == 0x9B:
        d.mnemonic = "WAIT"

    # IN / OUT
    elif op == 0xE4:
        d.mnemonic = "IN"
        d.word = False
        d.imm = fetch8()
    elif op == 0xE5:
        d.mnemonic = "IN"
        d.word = True
        d.imm = fetch8()
    elif op == 0xEC:
        d.mnemonic = "IN"
        d.word = False
    elif op == 0xED:
        d.mnemonic = "IN"
        d.word = True
    elif op == 0xE6:
        d.mnemonic = "OUT"
        d.word = False
        d.imm = fetch8()
    elif op == 0xE7:
        d.mnemonic = "OUT"
        d.word = True
        d.imm = fetch8()
    elif op == 0xEE:
        d.mnemonic = "OUT"
        d.word = False
    elif op == 0xEF:
        d.mnemonic = "OUT"
        d.word = True

    else:
        d.mnemonic = f"DB({op:#04x})"

    d.length = pos
    return d
