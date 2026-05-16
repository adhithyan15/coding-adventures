"""decoder.py — Combinational instruction decoder for the AArch64 (ARMv8-A) simulator.

The decoder is a pure function: it takes a 32-bit instruction word and
returns an AArch64Instruction dataclass with all decoded fields.  No state
is modified; this models the combinational decode stage of a real pipeline.

AArch64 Instruction Encoding Overview
──────────────────────────────────────
All AArch64 instructions are 32 bits wide.  The instruction is always stored
big-endian in memory (byte[0] is most significant), and the 32-bit word's
MSB is bit 31.

Encoding classes and discriminant bits
───────────────────────────────────────
The AArch64 manual classifies instructions by their "op0" field at bits[28:25]
combined with other bits.  We use a sequence of pattern checks:

  Encoding                      Bits   Pattern
  ─────────────────────────────────────────────
  B / BL (uncond imm)           [30:26] = 00101
  B.cond                        [31:24] = 01010100, [4] = 0
  CBZ / CBNZ                    [30:25] = 011010
  TBZ / TBNZ                    [30:25] = 011011
  BR / BLR / RET (reg)          [31:24] = 11010110
  ADD/SUB imm                   [28:23] in {100000, 100001}
  MOV wide (MOVZ/MOVN/MOVK)     [28:23] = 100101
  Logical immediate              [28:23] = 010010
  Load/Store unsigned offset     [29:27] = 111, [25:24] = 01, [26] = 0
  Logical shifted register       [28:24] = 01010
  Arithmetic shifted register    [28:24] = 01011, [21] = 0
  Data proc 2-source             [30] = 0, [28:21] = 11010110
  Data proc 1-source             [30] = 1, [28:21] = 11010110, [20:16] = 00000
  3-source (MADD/MSUB)           [28:24] = 11011
  Conditional select             [28:21] = 11010100
  NOP                            raw = 0xD503201F
  HALT                           raw = 0

Bitmask immediate (logical immediate)
──────────────────────────────────────
AArch64 logical immediates encode repeating bitmasks via a (N, immr, imms)
triple.  Decoding is an integer algorithm (not a data-path operation):
  1. If N=1: element size = 64 bits
  2. Else: element size = 2^len where len = highest bit of (~imms & 0x3F) | (N<<6)
  3. S = imms & (esize-1)  — number of set bits minus 1
  4. R = immr & (esize-1)  — rotation amount
  5. welem = (1 << (S+1)) - 1
  6. telem = ror(welem, R, esize)
  7. result = telem replicated to fill 64 bits

This is encoding bookkeeping — it decodes a compact representation into
the actual 64-bit value to use.
"""

from __future__ import annotations

from dataclasses import dataclass


@dataclass
class AArch64Instruction:
    """Decoded AArch64 instruction with all fields extracted.

    Fields that do not apply to a given instruction are left at their default
    values (0 or empty string).

    Attributes
    ──────────
    opcode      : mnemonic string (e.g., "ADD", "LDR", "B.EQ")
    sf          : 1→64-bit operation, 0→32-bit
    Rd          : destination register index (0–31)
    Rn          : first source register index (0–31)
    Rm          : second source register / index register (0–31)
    Ra          : accumulate register for MADD/MSUB (0–31)
    imm         : decoded immediate value (for imm12, imm16, imm26, etc.)
    shift_type  : shift type (0=LSL, 1=LSR, 2=ASR, 3=ROR)
    shift_amount: shift amount (imm6 field)
    cond        : condition code (4 bits, for B.cond / CSEL / etc.)
    bit_num     : bit number for TBZ/TBNZ (0–63)
    size        : load/store size (0=byte, 1=half, 2=word, 3=dword)
    opc         : load/store opc (00=store, 01=load, 10=load-signed)
    N_bit       : N field for bitmask immediate
    immr        : immr field for bitmask immediate
    imms        : imms field for bitmask immediate
    bitmask_imm : decoded 64-bit bitmask immediate value
    op          : operation selector (0/1 for ADD/SUB, etc.)
    S           : set-flags bit (1=sets NZCV)
    op2         : secondary op code (for conditional select, etc.)
    o0          : o0 bit for MADD/MSUB (0=MADD, 1=MSUB)
    hw          : halfword selector for MOV wide (0–3, shift = hw*16)
    opc2        : extended opcode2 for 1-source / 2-source
    """

    opcode: str = ""
    sf: int = 1
    Rd: int = 0
    Rn: int = 0
    Rm: int = 0
    Ra: int = 0
    imm: int = 0
    shift_type: int = 0
    shift_amount: int = 0
    cond: int = 0
    bit_num: int = 0
    size: int = 0
    opc: int = 0
    N_bit: int = 0
    immr: int = 0
    imms: int = 0
    bitmask_imm: int = 0
    op: int = 0
    S: int = 0
    op2: int = 0
    o0: int = 0
    hw: int = 0
    opc2: int = 0


# ── Bitmask immediate decoder ──────────────────────────────────────────────────


def _ror(value: int, amount: int, width: int) -> int:
    """Rotate `value` right by `amount` within a field of `width` bits.

    Used in bitmask immediate decoding (bookkeeping, not data-path).
    """
    amount %= width
    if amount == 0:
        return value
    mask = (1 << width) - 1
    return ((value >> amount) | (value << (width - amount))) & mask


def decode_bitmask(N: int, immr: int, imms: int) -> int:
    """Decode AArch64 logical-immediate encoding (N, immr, imms) → 64-bit mask.

    This is encoding bookkeeping (integer arithmetic), not a data-path operation.
    The result is the 64-bit immediate value to be used by the ALU.

    Algorithm
    ─────────
    1. Element size (esize) from N and imms:
       - N=1 → 64-bit element (len=6)
       - N=0 → len = (highest set bit of ((~imms & 0x3F) | (N << 6))) - 1
    2. S = imms & (esize - 1)  — number of set bits minus 1
    3. R = immr & (esize - 1)  — right-rotation amount
    4. welem = (1 << (S+1)) - 1  — S+1 consecutive 1-bits
    5. telem = ror(welem, R, esize)
    6. Replicate telem to fill 64 bits

    Raises ValueError for the UNDEFINED encoding.

    Examples
    ────────
    >>> decode_bitmask(1, 0, 62)  # N=1, immr=0, imms=62 → 63 ones = 0x7FFFFFFFFFFFFFFF
    9223372036854775807
    >>> decode_bitmask(1, 0, 0)   # N=1, immr=0, imms=0  → 1 one = 0x1
    1
    """
    if N == 1:
        len_ = 6
    else:
        combined = (~imms & 0x3F) | (N << 6)
        len_ = combined.bit_length() - 1
        if len_ <= 0:
            raise ValueError(f"UNDEFINED bitmask: N={N}, immr={immr}, imms={imms}")
    esize = 1 << len_
    S = imms & (esize - 1)
    R = immr & (esize - 1)
    welem = (1 << (S + 1)) - 1
    telem = _ror(welem, R, esize)
    result = 0
    for pos in range(0, 64, esize):
        result |= telem << pos
    return result & 0xFFFF_FFFF_FFFF_FFFF


# ── Sign-extension helpers (bookkeeping, not data-path) ───────────────────────


def _sext(v: int, bits: int) -> int:
    """Sign-extend a value in `bits` low bits to a Python signed integer."""
    v = v & ((1 << bits) - 1)
    if v >> (bits - 1):
        return v - (1 << bits)
    return v


def _sext14(v: int) -> int:
    """Sign-extend a 14-bit value (TBZ/TBNZ offset)."""
    return _sext(v, 14)


def _sext19(v: int) -> int:
    """Sign-extend a 19-bit value (CBZ/B.cond offset)."""
    return _sext(v, 19)


def _sext26(v: int) -> int:
    """Sign-extend a 26-bit value (B/BL offset)."""
    return _sext(v, 26)


def _sext12(v: int) -> int:
    """Sign-extend a 12-bit value."""
    return _sext(v, 12)


# ── Condition code names ───────────────────────────────────────────────────────

_COND_NAMES = [
    "EQ", "NE", "CS", "CC", "MI", "PL", "VS", "VC",
    "HI", "LS", "GE", "LT", "GT", "LE", "AL", "NV",
]


def _cond_name(cond: int) -> str:
    """Return the mnemonic suffix for a 4-bit condition code."""
    return _COND_NAMES[cond & 0xF]


# ── Main decoder function ──────────────────────────────────────────────────────


def decode(raw: int) -> AArch64Instruction:
    """Decode a 32-bit AArch64 instruction word into structured fields.

    This is a pure combinational function: no state is read or modified.
    It models the decode stage of an AArch64 pipeline.

    The function returns an AArch64Instruction dataclass with all relevant
    fields populated.  Fields that don't apply to the decoded instruction
    are left at their default values.

    Parameters
    ──────────
    raw : 32-bit instruction word (big-endian; bit 31 = MSB in Python)

    Returns
    ───────
    AArch64Instruction with fields decoded from `raw`

    Raises
    ───────
    ValueError for UNDEFINED encodings (e.g., bad bitmask immediate).
    """
    def bits(hi: int, lo: int) -> int:
        """Extract bits[hi:lo] inclusive."""
        width = hi - lo + 1
        return (raw >> lo) & ((1 << width) - 1)

    sf = bits(31, 31)

    # ── HALT ──────────────────────────────────────────────────────────────────
    if raw == 0:
        return AArch64Instruction(opcode="HALT", sf=sf)

    # ── NOP ───────────────────────────────────────────────────────────────────
    if raw == 0xD503201F:
        return AArch64Instruction(opcode="NOP", sf=sf)

    # ── Unconditional branch (immediate): B / BL ──────────────────────────────
    # Encoding: op[31] | 00101[30:26] | imm26[25:0]
    if bits(30, 26) == 0b00101:
        op = bits(31, 31)
        imm26 = _sext26(bits(25, 0))
        mnem = "BL" if op else "B"
        return AArch64Instruction(opcode=mnem, sf=1, imm=imm26, op=op)

    # ── Conditional branch (immediate): B.cond ───────────────────────────────
    # Encoding: 01010100[31:24] | imm19[23:5] | 0[4] | cond[3:0]
    if bits(31, 24) == 0b01010100 and bits(4, 4) == 0:
        imm19 = _sext19(bits(23, 5))
        cond = bits(3, 0)
        return AArch64Instruction(
            opcode=f"B.{_cond_name(cond)}", sf=1, imm=imm19, cond=cond
        )

    # ── Compare-and-Branch: CBZ / CBNZ ───────────────────────────────────────
    # Encoding: sf[31] | 011010[30:25] | op[24] | imm19[23:5] | Rt[4:0]
    if bits(30, 25) == 0b011010:
        op = bits(24, 24)
        imm19 = _sext19(bits(23, 5))
        Rt = bits(4, 0)
        mnem = "CBNZ" if op else "CBZ"
        return AArch64Instruction(opcode=mnem, sf=sf, Rd=Rt, imm=imm19, op=op)

    # ── Test-and-Branch: TBZ / TBNZ ──────────────────────────────────────────
    # Encoding: b5[31] | 011011[30:25] | op[24] | b40[23:19] | imm14[18:5] | Rt[4:0]
    if bits(30, 25) == 0b011011:
        b5 = bits(31, 31)
        op = bits(24, 24)
        b40 = bits(23, 19)
        bit_num = (b5 << 5) | b40
        imm14 = _sext14(bits(18, 5))
        Rt = bits(4, 0)
        mnem = "TBNZ" if op else "TBZ"
        return AArch64Instruction(
            opcode=mnem, sf=sf, Rd=Rt, imm=imm14, bit_num=bit_num, op=op
        )

    # ── Unconditional branch (register): BR / BLR / RET ──────────────────────
    # Encoding: 1101011_0[31:24] | op[23:21] | 11111[20:16] | 000000[15:10] | Rn[9:5] | 00000[4:0]
    if bits(31, 24) == 0b1101_0110:
        op = bits(23, 21)
        Rn = bits(9, 5)
        if op == 0b000:
            mnem = "BR"
        elif op == 0b001:
            mnem = "BLR"
        elif op == 0b010:
            mnem = "RET"
        else:
            return AArch64Instruction(opcode=f"UNKNOWN(0x{raw:08X})", sf=sf)
        return AArch64Instruction(opcode=mnem, sf=1, Rn=Rn, op=op)

    # ── Data Processing Immediate: ADD/SUB (immediate) ────────────────────────
    # Encoding: sf[31] | op[30] | S[29] | 100000[28:23] | sh[22] | imm12[21:10] | Rn[9:5] | Rd[4:0]
    if bits(28, 23) in (0b100000, 0b100001):
        op = bits(30, 30)
        S = bits(29, 29)
        sh = bits(22, 22)
        imm12 = bits(21, 10)
        Rn = bits(9, 5)
        Rd = bits(4, 0)
        imm = imm12 << 12 if sh else imm12
        if op == 0:
            mnem = "ADDS" if S else "ADD"
        else:
            mnem = "SUBS" if S else "SUB"
        return AArch64Instruction(
            opcode=mnem, sf=sf, Rd=Rd, Rn=Rn, imm=imm, op=op, S=S
        )

    # ── Move Wide Immediate: MOVZ / MOVN / MOVK ──────────────────────────────
    # Encoding: sf[31] | opc[30:29] | 100101[28:23] | hw[22:21] | imm16[20:5] | Rd[4:0]
    if bits(28, 23) == 0b100101:
        opc = bits(30, 29)
        hw = bits(22, 21)
        imm16 = bits(20, 5)
        Rd = bits(4, 0)
        if opc == 0b10:
            mnem = "MOVZ"
        elif opc == 0b00:
            mnem = "MOVN"
        elif opc == 0b11:
            mnem = "MOVK"
        else:
            return AArch64Instruction(opcode=f"UNKNOWN(0x{raw:08X})", sf=sf)
        return AArch64Instruction(opcode=mnem, sf=sf, Rd=Rd, imm=imm16, hw=hw, opc=opc)

    # ── Logical Immediate: AND / ORR / EOR / ANDS ─────────────────────────────
    # Encoding: sf[31] | opc[30:29] | 0[28] | 10010[27:23] | N[22] | immr[21:16] | imms[15:10] | Rn[9:5] | Rd[4:0]
    # bits[28:23] = 010010 when bit28=0 and bits27:23=10010
    if bits(28, 23) == 0b010010:
        opc = bits(30, 29)
        N = bits(22, 22)
        immr = bits(21, 16)
        imms = bits(15, 10)
        Rn = bits(9, 5)
        Rd = bits(4, 0)
        try:
            bitmask = decode_bitmask(N, immr, imms)
        except ValueError:
            return AArch64Instruction(opcode=f"UNKNOWN(0x{raw:08X})", sf=sf)
        if opc == 0b00:
            mnem = "AND"
        elif opc == 0b01:
            mnem = "ORR"
        elif opc == 0b10:
            mnem = "EOR"
        else:
            mnem = "ANDS"
        return AArch64Instruction(
            opcode=mnem, sf=sf, Rd=Rd, Rn=Rn,
            N_bit=N, immr=immr, imms=imms, bitmask_imm=bitmask,
            opc=opc,
        )

    # ── Load/Store Unsigned Offset ─────────────────────────────────────────────
    # Encoding: size[31:30] | 111[29:27] | V[26] | 01[25:24] | opc[23:22] | imm12[21:10] | Rn[9:5] | Rt[4:0]
    if bits(29, 27) == 0b111 and bits(25, 24) == 0b01 and bits(26, 26) == 0:
        size = bits(31, 30)
        opc = bits(23, 22)
        imm12 = bits(21, 10)
        Rn = bits(9, 5)
        Rt = bits(4, 0)
        # Determine mnemonic from size/opc
        _ldst_mnemonics = {
            (0, 0b00): "STRB",   (0, 0b01): "LDRB",   (0, 0b10): "LDRSB",  (0, 0b11): "LDRSB32",
            (1, 0b00): "STRH",   (1, 0b01): "LDRH",   (1, 0b10): "LDRSH",  (1, 0b11): "LDRSH32",
            (2, 0b00): "STR32",  (2, 0b01): "LDR32",  (2, 0b10): "LDRSW",
            (3, 0b00): "STR",    (3, 0b01): "LDR",
        }
        mnem = _ldst_mnemonics.get((size, opc), f"UNKNOWN(0x{raw:08X})")
        return AArch64Instruction(
            opcode=mnem, sf=sf, Rd=Rt, Rn=Rn, imm=imm12, size=size, opc=opc
        )

    # ── Logical Shifted Register: AND/ORR/EOR/ANDS and BIC/ORN/EON/BICS ──────
    # Encoding: sf[31] | opc[30:29] | 01010[28:24] | shift[23:22] | N[21] | Rm[20:16] | imm6[15:10] | Rn[9:5] | Rd[4:0]
    if bits(28, 24) == 0b01010:
        opc = bits(30, 29)
        shift_type = bits(23, 22)
        N = bits(21, 21)
        Rm = bits(20, 16)
        imm6 = bits(15, 10)
        Rn = bits(9, 5)
        Rd = bits(4, 0)
        if opc == 0b00:
            mnem = "BIC" if N else "AND"
        elif opc == 0b01:
            mnem = "ORN" if N else "ORR"
        elif opc == 0b10:
            mnem = "EON" if N else "EOR"
        else:
            mnem = "BICS" if N else "ANDS"
        return AArch64Instruction(
            opcode=mnem, sf=sf, Rd=Rd, Rn=Rn, Rm=Rm,
            shift_type=shift_type, shift_amount=imm6,
            N_bit=N, opc=opc,
        )

    # ── Arithmetic Shifted Register: ADD/SUB ──────────────────────────────────
    # Encoding: sf[31] | op[30] | S[29] | 01011[28:24] | shift[23:22] | 0[21] | Rm[20:16] | imm6[15:10] | Rn[9:5] | Rd[4:0]
    if bits(28, 24) == 0b01011 and bits(21, 21) == 0:
        op = bits(30, 30)
        S = bits(29, 29)
        shift_type = bits(23, 22)
        Rm = bits(20, 16)
        imm6 = bits(15, 10)
        Rn = bits(9, 5)
        Rd = bits(4, 0)
        if op == 0:
            mnem = "ADDS" if S else "ADD"
        else:
            mnem = "SUBS" if S else "SUB"
        return AArch64Instruction(
            opcode=mnem, sf=sf, Rd=Rd, Rn=Rn, Rm=Rm,
            shift_type=shift_type, shift_amount=imm6,
            op=op, S=S,
        )

    # ── Data Processing 2-Source: UDIV/SDIV/LSLV/LSRV/ASRV/RORV ────────────
    # Encoding: sf[31] | 0[30] | S[29] | 11010110[28:21] | Rm[20:16] | opc2[15:10] | Rn[9:5] | Rd[4:0]
    if bits(30, 30) == 0 and bits(28, 21) == 0b11010110:
        Rm = bits(20, 16)
        opc2 = bits(15, 10)
        Rn = bits(9, 5)
        Rd = bits(4, 0)
        _dp2_mnemonics = {
            0b000010: "UDIV",
            0b000011: "SDIV",
            0b001000: "LSLV",
            0b001001: "LSRV",
            0b001010: "ASRV",
            0b001011: "RORV",
        }
        mnem = _dp2_mnemonics.get(opc2, f"UNKNOWN(0x{raw:08X})")
        return AArch64Instruction(
            opcode=mnem, sf=sf, Rd=Rd, Rn=Rn, Rm=Rm, opc2=opc2
        )

    # ── Data Processing 1-Source: CLZ/RBIT/REV/REV16/REV32 ──────────────────
    # Encoding: sf[31] | 1[30] | S[29] | 11010110[28:21] | 00000[20:16] | opc2[15:10] | Rn[9:5] | Rd[4:0]
    if bits(30, 30) == 1 and bits(28, 21) == 0b11010110 and bits(20, 16) == 0:
        opc2 = bits(15, 10)
        Rn = bits(9, 5)
        Rd = bits(4, 0)
        _dp1_mnemonics = {
            0b000000: "RBIT",
            0b000001: "REV16",
            0b000010: "REV",
            0b000011: "REV32",   # only valid for sf=1
            0b000100: "CLZ",
        }
        mnem = _dp1_mnemonics.get(opc2, f"UNKNOWN(0x{raw:08X})")
        return AArch64Instruction(opcode=mnem, sf=sf, Rd=Rd, Rn=Rn, opc2=opc2)

    # ── 3-Source: MADD / MSUB (MUL = MADD Ra=XZR, MNEG = MSUB Ra=XZR) ───────
    # Encoding: sf[31] | 0[30] | 0[29] | 11011[28:24] | op54[23:21] | Rm[20:16] | o0[15] | Ra[14:10] | Rn[9:5] | Rd[4:0]
    if bits(28, 24) == 0b11011:
        op54 = bits(23, 21)
        Rm = bits(20, 16)
        o0 = bits(15, 15)
        Ra = bits(14, 10)
        Rn = bits(9, 5)
        Rd = bits(4, 0)
        if op54 == 0b000:
            mnem = "MSUB" if o0 else "MADD"
        elif op54 == 0b001 and sf == 1:
            mnem = "SMULH"
        elif op54 == 0b010 and sf == 1:
            mnem = "UMULH"
        else:
            mnem = f"UNKNOWN(0x{raw:08X})"
        return AArch64Instruction(
            opcode=mnem, sf=sf, Rd=Rd, Rn=Rn, Rm=Rm, Ra=Ra, o0=o0, op=op54
        )

    # ── Conditional Select: CSEL / CSINC / CSINV / CSNEG ─────────────────────
    # Encoding: sf[31] | op[30] | S[29] | 11010100[28:21] | Rm[20:16] | cond[15:12] | op2[11:10] | Rn[9:5] | Rd[4:0]
    if bits(28, 21) == 0b11010100:
        op = bits(30, 30)
        Rm = bits(20, 16)
        cond = bits(15, 12)
        op2 = bits(11, 10)
        Rn = bits(9, 5)
        Rd = bits(4, 0)
        if op == 0 and op2 == 0b00:
            mnem = "CSEL"
        elif op == 0 and op2 == 0b01:
            mnem = "CSINC"
        elif op == 1 and op2 == 0b00:
            mnem = "CSINV"
        elif op == 1 and op2 == 0b01:
            mnem = "CSNEG"
        else:
            mnem = f"UNKNOWN(0x{raw:08X})"
        return AArch64Instruction(
            opcode=mnem, sf=sf, Rd=Rd, Rn=Rn, Rm=Rm, cond=cond, op=op, op2=op2
        )

    # ── Unknown encoding ──────────────────────────────────────────────────────
    return AArch64Instruction(opcode=f"UNKNOWN(0x{raw:08X})", sf=sf)
