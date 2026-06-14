"""Instruction decoder for the Motorola 68000 gate-level simulator.

=== How 68000 decoding works ===

The 68000 decodes instructions from a 16-bit *opword* fetched from memory.
The top 4 bits (bits 15–12) give a rough classification of the instruction,
while lower bits encode size, addressing mode, and register fields.

Most instructions are followed by 0–4 extension words (16 bits each) that
carry immediate values, displacements, or the second part of the effective
address.  Decoding consumes these extension words in program order.

=== EA (effective address) encoding ===

A 6-bit EA field = mode[2:0] : reg[2:0].

    mode  reg   Notation        Description
    000   Dn    Dn              Data register direct
    001   An    An              Address register direct
    010   An    (An)            Address register indirect
    011   An    (An)+           Indirect with postincrement
    100   An    -(An)           Indirect with predecrement
    101   An    d16(An)         Indirect + signed 16-bit displacement
    110   An    d8(An,Xn.sz)    Indirect + index register + 8-bit disp
    111   000   (abs).W         Absolute short (sign-extended 16-bit addr)
    111   001   (abs).L         Absolute long (32-bit addr)
    111   010   d16(PC)         PC-relative + 16-bit displacement
    111   011   d8(PC,Xn.sz)    PC-relative + index register
    111   100   #imm            Immediate data

=== Instruction class table (bits 15–12) ===

    0000  immediate / bit ops
    0001  MOVE.B
    0010  MOVE.L
    0011  MOVE.W
    0100  misc (NEG, CLR, TST, LEA, JSR, JMP, RTS, LINK…)
    0101  ADDQ, SUBQ, Scc, DBcc
    0110  BRA, BSR, Bcc
    0111  MOVEQ
    1000  OR, DIVU, DIVS, SBCD
    1001  SUB, SUBA, SUBX
    1010  (A-line trap)
    1011  CMP, CMPA, EOR, CMPM
    1100  AND, MULU, MULS, EXG, ABCD
    1101  ADD, ADDA, ADDX
    1110  shift/rotate family

=== Size codes ===

Most instructions encode size in bits 7–6:
    00 = byte (1 byte)
    01 = word (2 bytes)
    10 = long (4 bytes)

MOVE uses a different encoding in bits 13–12:
    01 = byte
    11 = word
    10 = long
"""

from __future__ import annotations

from dataclasses import dataclass


@dataclass
class DecodedInstr68k:
    """Decoded instruction descriptor for the 68000.

    Fields:
        mnemonic:   Human-readable instruction name (e.g. 'ADD.L').
        size:       Operand size in bytes: 1, 2, or 4 (or 0 for size-less ops).
        src_mode:   Source EA mode field (0–7; -1 if not applicable).
        src_reg:    Source EA register field (0–7; -1 if not applicable).
        dst_mode:   Destination EA mode field (-1 if not applicable).
        dst_reg:    Destination EA register field (-1 if not applicable).
        byte_length: Total instruction length in bytes (2 + extension words × 2).
    """

    mnemonic: str
    size: int
    src_mode: int
    src_reg: int
    dst_mode: int
    dst_reg: int
    byte_length: int


# Size code → bytes (for the standard arith encoding)
_SZ_ARITH = {0: 1, 1: 2, 2: 4}
# Size code → bytes (for MOVE encoding: bits 13-12)
_SZ_MOVE = {1: 1, 3: 2, 2: 4}
# Condition code names, indexed 0–15
_CC_NAMES = [
    "T", "F", "HI", "LS",
    "CC", "CS", "NE", "EQ",
    "VC", "VS", "PL", "MI",
    "GE", "LT", "GT", "LE",
]


def _ea_ext_words(mode: int, reg: int, size: int) -> int:
    """Return the number of extension words consumed by an EA field.

    Args:
        mode: EA mode (0–7).
        reg:  EA register (0–7).
        size: Operand size in bytes (for immediate mode).

    Returns:
        Number of 16-bit extension words (0, 1, or 2).
    """
    if mode in (0, 1, 2, 3, 4):
        return 0
    if mode == 5:   # d16(An)
        return 1
    if mode == 6:   # d8(An,Xn)
        return 1
    if mode == 7:
        if reg == 0:  # (abs).W
            return 1
        if reg == 1:  # (abs).L
            return 2
        if reg == 2:  # d16(PC)
            return 1
        if reg == 3:  # d8(PC,Xn)
            return 1
        if reg == 4:  # #imm
            return 2 if size == 4 else 1
    return 0


def decode(memory: bytes | bytearray, pc: int) -> DecodedInstr68k:
    """Decode one instruction from memory at the given PC.

    Reads the 16-bit opword and any required extension words, returning a
    DecodedInstr68k that describes the instruction without executing it.

    Note: This decoder is used for informational purposes (test assertions,
    disassembly).  The simulator itself decodes-and-executes in one pass.

    Args:
        memory: Flat byte array (at least 16 MB for full address space).
        pc:     Program counter (byte offset into memory).

    Returns:
        DecodedInstr68k descriptor.
    """
    addr_mask = 0xFF_FFFF

    def read_word(addr: int) -> int:
        a = addr & addr_mask
        return (memory[a] << 8) | memory[a + 1]

    op = read_word(pc)
    hi = (op >> 12) & 0xF
    length = 2  # minimum: just the opword

    # Helper to count extension words for one EA and update length
    def ea_words(mode: int, reg: int, size: int) -> int:
        return _ea_ext_words(mode, reg, size) * 2

    # ── Line 0: immediate / bit ops ───────────────────────────────────────────
    if hi == 0x0:
        sz_code = (op >> 6) & 3
        mode = (op >> 3) & 7
        reg = op & 7
        sz = _SZ_ARITH.get(sz_code, 2)

        # BTST/BCHG/BCLR/BSET immediate bit number
        if (op & 0xFF00) == 0x0800:
            bit_op = {0: "BTST", 1: "BCHG", 2: "BCLR", 3: "BSET"}[(op >> 6) & 3]
            length += 2 + ea_words(mode, reg, 4)  # bit num word + EA ext
            return DecodedInstr68k(f"{bit_op} #imm,<ea>", 0, 7, 4, mode, reg, length)

        # BTST/BCHG/BCLR/BSET register bit number
        if (op & 0x0138) == 0x0100 and sz_code <= 3:
            dn = (op >> 9) & 7
            bit_op = {0: "BTST", 1: "BCHG", 2: "BCLR", 3: "BSET"}[(op >> 6) & 3]
            length += ea_words(mode, reg, 4)
            return DecodedInstr68k(f"{bit_op} D{dn},<ea>", 0, 0, dn, mode, reg, length)

        op8 = (op >> 8) & 0xFF
        imm_names = {
            0x00: "ORI", 0x02: "ANDI", 0x04: "SUBI", 0x06: "ADDI",
            0x0A: "EORI", 0x0C: "CMPI",
        }
        if op8 in imm_names:
            name = imm_names[op8]
            length += (4 if sz == 4 else 2) + ea_words(mode, reg, sz)
            return DecodedInstr68k(
                f"{name}.{'BWL'[list(_SZ_ARITH.values()).index(sz)]} #imm,<ea>",
                sz, 7, 4, mode, reg, length,
            )
        return DecodedInstr68k(f"LINE0 0x{op:04X}", 0, -1, -1, -1, -1, 2)

    # ── Lines 1/2/3: MOVE ─────────────────────────────────────────────────────
    if hi in (0x1, 0x2, 0x3):
        sz = _SZ_MOVE.get(hi, 2)
        dst_reg = (op >> 9) & 7
        dst_mode_raw = (op >> 6) & 7
        src_mode = (op >> 3) & 7
        src_reg = op & 7
        # MOVE.W An,... is MOVEA
        _sz_suffix = {1: "B", 2: "L", 3: "W"}
        mnemonic = f"MOVE{'A' if dst_mode_raw == 1 else ''}.{_sz_suffix[hi]}"
        length += ea_words(src_mode, src_reg, sz) + ea_words(dst_mode_raw, dst_reg, sz)
        return DecodedInstr68k(mnemonic, sz, src_mode, src_reg, dst_mode_raw, dst_reg, length)

    # ── Line 4: misc ──────────────────────────────────────────────────────────
    if hi == 0x4:
        return _decode_line4(op, pc, length, ea_words)

    # ── Line 5: ADDQ/SUBQ/Scc/DBcc ───────────────────────────────────────────
    if hi == 0x5:
        sz_code = (op >> 6) & 3
        mode = (op >> 3) & 7
        reg = op & 7
        cc = (op >> 8) & 0xF
        if sz_code == 3:
            if mode == 1:  # DBcc
                length += 2  # 16-bit displacement
                return DecodedInstr68k(f"DB{_CC_NAMES[cc]} D{reg},d16", 2, 0, reg, -1, -1, length)
            # Scc
            length += ea_words(mode, reg, 1)
            return DecodedInstr68k(f"S{_CC_NAMES[cc]} <ea>", 1, -1, -1, mode, reg, length)
        sz = _SZ_ARITH.get(sz_code, 2)
        name = "ADDQ" if (op >> 8) & 1 == 0 else "SUBQ"
        length += ea_words(mode, reg, sz)
        return DecodedInstr68k(f"{name}.{'BWL'[sz_code]} #imm,<ea>", sz, -1, -1, mode, reg, length)

    # ── Line 6: BRA/BSR/Bcc ──────────────────────────────────────────────────
    if hi == 0x6:
        cc = (op >> 8) & 0xF
        disp8 = op & 0xFF
        if disp8 == 0:
            length += 2  # 16-bit displacement
        if cc == 0:
            name = "BRA"
        elif cc == 1:
            name = "BSR"
        else:
            name = f"B{_CC_NAMES[cc]}"
        return DecodedInstr68k(name, 0, -1, -1, -1, -1, length)

    # ── Line 7: MOVEQ ─────────────────────────────────────────────────────────
    if hi == 0x7:
        dn = (op >> 9) & 7
        return DecodedInstr68k(f"MOVEQ #imm,D{dn}", 4, -1, -1, 0, dn, 2)

    # ── Line 8: OR/DIVU/DIVS/SBCD ────────────────────────────────────────────
    if hi == 0x8:
        dn = (op >> 9) & 7
        sz_code = (op >> 6) & 3
        mode = (op >> 3) & 7
        reg = op & 7
        opmode = (op >> 6) & 7
        if opmode == 3:   # DIVU
            length += ea_words(mode, reg, 2)
            return DecodedInstr68k(f"DIVU <ea>,D{dn}", 2, mode, reg, 0, dn, length)
        if opmode == 7:   # DIVS
            length += ea_words(mode, reg, 2)
            return DecodedInstr68k(f"DIVS <ea>,D{dn}", 2, mode, reg, 0, dn, length)
        if (op & 0x01F0) == 0x0100:  # SBCD
            return DecodedInstr68k("SBCD", 1, -1, -1, -1, -1, 2)
        sz = _SZ_ARITH.get(sz_code, 2)
        length += ea_words(mode, reg, sz)
        return DecodedInstr68k(f"OR.{'BWL'[sz_code]} <ea>,D{dn}", sz, mode, reg, 0, dn, length)

    # ── Line 9: SUB/SUBA/SUBX ────────────────────────────────────────────────
    if hi == 0x9:
        dn = (op >> 9) & 7
        opmode = (op >> 6) & 7
        mode = (op >> 3) & 7
        reg = op & 7
        if opmode in (3, 7):  # SUBA
            sz = 4 if opmode == 7 else 2
            length += ea_words(mode, reg, sz)
            return DecodedInstr68k(f"SUBA.{'WL'[opmode == 7]} <ea>,A{dn}", sz, mode, reg, 1, dn, length)
        if opmode in (1, 5) and mode == 0:  # SUBX Dm,Dn
            sz = _SZ_ARITH.get(opmode - 1 if opmode == 1 else 2, 1)
            return DecodedInstr68k(f"SUBX D{reg},D{dn}", sz, 0, reg, 0, dn, 2)
        sz = _SZ_ARITH.get(opmode, 2)
        length += ea_words(mode, reg, sz)
        return DecodedInstr68k(f"SUB.{'BWL'[opmode]} <ea>", sz, mode, reg, 0, dn, length)

    # ── Line B: CMP/CMPA/EOR/CMPM ────────────────────────────────────────────
    if hi == 0xB:
        dn = (op >> 9) & 7
        opmode = (op >> 6) & 7
        mode = (op >> 3) & 7
        reg = op & 7
        if opmode in (3, 7):  # CMPA
            sz = 4 if opmode == 7 else 2
            length += ea_words(mode, reg, sz)
            return DecodedInstr68k(f"CMPA.{'WL'[opmode == 7]} <ea>,A{dn}", sz, mode, reg, 1, dn, length)
        sz = _SZ_ARITH.get(opmode if opmode <= 2 else opmode - 4, 2)
        if opmode >= 4 and mode == 1:  # CMPM (An)+,(An)+
            return DecodedInstr68k(f"CMPM (A{reg})+,(A{dn})+", sz, 3, reg, 3, dn, 2)
        if opmode >= 4:  # EOR
            length += ea_words(mode, reg, sz)
            return DecodedInstr68k(f"EOR.{'BWL'[opmode-4]} D{dn},<ea>", sz, 0, dn, mode, reg, length)
        length += ea_words(mode, reg, sz)
        return DecodedInstr68k(f"CMP.{'BWL'[opmode]} <ea>,D{dn}", sz, mode, reg, 0, dn, length)

    # ── Line C: AND/MULU/MULS/EXG/ABCD ───────────────────────────────────────
    if hi == 0xC:
        dn = (op >> 9) & 7
        opmode = (op >> 6) & 7
        mode = (op >> 3) & 7
        reg = op & 7
        if opmode == 3:  # MULU
            length += ea_words(mode, reg, 2)
            return DecodedInstr68k(f"MULU <ea>,D{dn}", 2, mode, reg, 0, dn, length)
        if opmode == 7:  # MULS
            length += ea_words(mode, reg, 2)
            return DecodedInstr68k(f"MULS <ea>,D{dn}", 2, mode, reg, 0, dn, length)
        if (op & 0x01F0) == 0x0100:  # ABCD
            return DecodedInstr68k("ABCD", 1, -1, -1, -1, -1, 2)
        if (op & 0x01F0) in (0x0140, 0x0148, 0x0188):  # EXG
            return DecodedInstr68k("EXG", 4, -1, -1, -1, -1, 2)
        sz = _SZ_ARITH.get(opmode if opmode <= 2 else opmode - 4, 2)
        length += ea_words(mode, reg, sz)
        return DecodedInstr68k(f"AND.{'BWL'[opmode if opmode <= 2 else opmode-4]} <ea>,D{dn}", sz, mode, reg, 0, dn, length)

    # ── Line D: ADD/ADDA/ADDX ────────────────────────────────────────────────
    if hi == 0xD:
        dn = (op >> 9) & 7
        opmode = (op >> 6) & 7
        mode = (op >> 3) & 7
        reg = op & 7
        if opmode in (3, 7):  # ADDA
            sz = 4 if opmode == 7 else 2
            length += ea_words(mode, reg, sz)
            return DecodedInstr68k(f"ADDA.{'WL'[opmode == 7]} <ea>,A{dn}", sz, mode, reg, 1, dn, length)
        if opmode in (1, 5) and mode == 0:  # ADDX Dm,Dn
            sz_idx = opmode - 1 if opmode == 1 else (opmode - 1 - 3)
            sz = _SZ_ARITH.get(max(0, sz_idx), 1)
            return DecodedInstr68k(f"ADDX D{reg},D{dn}", sz, 0, reg, 0, dn, 2)
        sz = _SZ_ARITH.get(opmode if opmode <= 2 else opmode - 4, 2)
        length += ea_words(mode, reg, sz)
        return DecodedInstr68k(f"ADD.{'BWL'[opmode if opmode <= 2 else opmode-4]} <ea>,D{dn}", sz, mode, reg, 0, dn, length)

    # ── Line E: shifts/rotates ────────────────────────────────────────────────
    # Register/immediate shifts: bits 7-6 = size (00=B,01=W,10=L), bit 8 = dir,
    # bits 4-3 = shift type (00=AS,01=LS,10=ROXS,11=ROS), bits 2-0 = Dn.
    # Memory shifts: bits 7-6 = 11 (sz_code=3), bits 11-9 = shift type.
    if hi == 0xE:
        sz_code = (op >> 6) & 3
        dir_bit = (op >> 8) & 1   # 0=right, 1=left
        names = ["AS", "LS", "ROX", "RO"]
        direction = "L" if dir_bit else "R"
        if sz_code == 3:  # memory shift (1 bit only), type in bits 11-9
            shift_type = (op >> 9) & 3
            mode = (op >> 3) & 7
            reg = op & 7
            length += ea_words(mode, reg, 2)
            return DecodedInstr68k(f"{names[shift_type]}{direction}.W <ea>", 2, -1, -1, mode, reg, length)
        # Register/immediate shift: type in bits 4-3
        shift_type = (op >> 3) & 3
        sz = _SZ_ARITH.get(sz_code, 2)
        reg = op & 7
        return DecodedInstr68k(f"{names[shift_type]}{direction}.{'BWL'[sz_code]} D{reg}", sz, -1, -1, 0, reg, 2)

    return DecodedInstr68k(f"UNKNOWN 0x{op:04X}", 0, -1, -1, -1, -1, 2)


def _decode_line4(
    op: int, pc: int, base_length: int, ea_words: object
) -> DecodedInstr68k:
    """Decode line 4 (miscellaneous) instructions.

    Line 4 is the most heterogeneous group, containing NEG, CLR, TST,
    NOT, LEA, PEA, SWAP, EXT, LINK, UNLK, MOVEM, JSR, JMP, RTS, RTE,
    NOP, TRAP, STOP, and others.
    """
    length = base_length
    mode = (op >> 3) & 7
    reg = op & 7
    sz_code = (op >> 6) & 3

    def ew(m: int, r: int, s: int) -> int:
        return _ea_ext_words(m, r, s) * 2

    # NOP
    if op == 0x4E71:
        return DecodedInstr68k("NOP", 0, -1, -1, -1, -1, 2)
    # RTS
    if op == 0x4E75:
        return DecodedInstr68k("RTS", 0, -1, -1, -1, -1, 2)
    # RTR
    if op == 0x4E77:
        return DecodedInstr68k("RTR", 0, -1, -1, -1, -1, 2)
    # RTE
    if op == 0x4E73:
        return DecodedInstr68k("RTE", 0, -1, -1, -1, -1, 2)
    # RESET
    if op == 0x4E70:
        return DecodedInstr68k("RESET", 0, -1, -1, -1, -1, 2)
    # ILLEGAL
    if op == 0x4AFC:
        return DecodedInstr68k("ILLEGAL", 0, -1, -1, -1, -1, 2)

    # TRAP #n (0x4E40–0x4E4F)
    if (op & 0xFFF0) == 0x4E40:
        return DecodedInstr68k(f"TRAP #{op & 0xF}", 0, -1, -1, -1, -1, 2)

    # STOP #imm
    if op == 0x4E72:
        return DecodedInstr68k("STOP #imm", 0, -1, -1, -1, -1, 4)

    # LINK An, #imm
    if (op & 0xFFF8) == 0x4E50:
        return DecodedInstr68k(f"LINK A{reg},#imm", 0, -1, -1, -1, -1, 4)

    # UNLK An
    if (op & 0xFFF8) == 0x4E58:
        return DecodedInstr68k(f"UNLK A{reg}", 0, -1, -1, -1, -1, 2)

    # JSR <ea>
    if (op & 0xFFC0) == 0x4E80:
        length += ew(mode, reg, 4)
        return DecodedInstr68k("JSR <ea>", 0, mode, reg, -1, -1, length)

    # JMP <ea>
    if (op & 0xFFC0) == 0x4EC0:
        length += ew(mode, reg, 4)
        return DecodedInstr68k("JMP <ea>", 0, mode, reg, -1, -1, length)

    # LEA <ea>, An
    if (op & 0xF1C0) == 0x41C0:
        an = (op >> 9) & 7
        length += ew(mode, reg, 4)
        return DecodedInstr68k(f"LEA <ea>,A{an}", 4, mode, reg, 1, an, length)

    # SWAP Dn — must come before PEA (same 0xFFC0 base but mode=0 = Dn direct)
    if (op & 0xFFF8) == 0x4840:
        return DecodedInstr68k(f"SWAP D{reg}", 4, -1, -1, 0, reg, 2)

    # EXT.W / EXT.L — must come before MOVEM (same 0xFB80 base but mode=0)
    if (op & 0xFFF8) == 0x4880:
        return DecodedInstr68k(f"EXT.W D{reg}", 2, -1, -1, 0, reg, 2)
    if (op & 0xFFF8) == 0x48C0:
        return DecodedInstr68k(f"EXT.L D{reg}", 4, -1, -1, 0, reg, 2)

    # PEA <ea>
    if (op & 0xFFC0) == 0x4840:
        length += ew(mode, reg, 4)
        return DecodedInstr68k("PEA <ea>", 4, mode, reg, -1, -1, length)

    # MOVEM (0x48xx = register to memory; 0x4Cxx = memory to register)
    if (op & 0xFB80) == 0x4880:
        sz = 4 if (op >> 6) & 1 else 2
        length += 2 + ew(mode, reg, sz)  # register mask word
        return DecodedInstr68k(f"MOVEM.{'WL'[sz==4]} regs,<ea>", sz, -1, -1, mode, reg, length)
    if (op & 0xFB80) == 0x4C80:
        sz = 4 if (op >> 6) & 1 else 2
        length += 2 + ew(mode, reg, sz)
        return DecodedInstr68k(f"MOVEM.{'WL'[sz==4]} <ea>,regs", sz, mode, reg, -1, -1, length)

    # CLR
    if (op & 0xFF00) == 0x4200:
        sz = _SZ_ARITH.get(sz_code, 2)
        length += ew(mode, reg, sz)
        return DecodedInstr68k(f"CLR.{'BWL'[sz_code]} <ea>", sz, -1, -1, mode, reg, length)

    # NEG
    if (op & 0xFF00) == 0x4400:
        sz = _SZ_ARITH.get(sz_code, 2)
        length += ew(mode, reg, sz)
        return DecodedInstr68k(f"NEG.{'BWL'[sz_code]} <ea>", sz, -1, -1, mode, reg, length)

    # NEGX
    if (op & 0xFF00) == 0x4000:
        sz = _SZ_ARITH.get(sz_code, 2)
        length += ew(mode, reg, sz)
        return DecodedInstr68k(f"NEGX.{'BWL'[sz_code]} <ea>", sz, -1, -1, mode, reg, length)

    # NOT
    if (op & 0xFF00) == 0x4600:
        sz = _SZ_ARITH.get(sz_code, 2)
        length += ew(mode, reg, sz)
        return DecodedInstr68k(f"NOT.{'BWL'[sz_code]} <ea>", sz, -1, -1, mode, reg, length)

    # TST
    if (op & 0xFF00) == 0x4A00:
        sz = _SZ_ARITH.get(sz_code, 2)
        length += ew(mode, reg, sz)
        return DecodedInstr68k(f"TST.{'BWL'[sz_code]} <ea>", sz, mode, reg, -1, -1, length)

    # NBCD
    if (op & 0xFFC0) == 0x4800:
        length += ew(mode, reg, 1)
        return DecodedInstr68k("NBCD <ea>", 1, -1, -1, mode, reg, length)

    # CHK
    if (op & 0xF1C0) == 0x4180:
        dn = (op >> 9) & 7
        length += ew(mode, reg, 2)
        return DecodedInstr68k(f"CHK <ea>,D{dn}", 2, mode, reg, 0, dn, length)

    # MOVE to SR (0x46C0)
    if (op & 0xFFC0) == 0x46C0:
        length += ew(mode, reg, 2)
        return DecodedInstr68k("MOVE <ea>,SR", 2, mode, reg, -1, -1, length)

    # MOVE from SR (0x40C0)
    if (op & 0xFFC0) == 0x40C0:
        length += ew(mode, reg, 2)
        return DecodedInstr68k("MOVE SR,<ea>", 2, -1, -1, mode, reg, length)

    # MOVE to CCR (0x44C0)
    if (op & 0xFFC0) == 0x44C0:
        length += ew(mode, reg, 1)
        return DecodedInstr68k("MOVE <ea>,CCR", 1, mode, reg, -1, -1, length)

    return DecodedInstr68k(f"LINE4 0x{op:04X}", 0, -1, -1, -1, -1, 2)
