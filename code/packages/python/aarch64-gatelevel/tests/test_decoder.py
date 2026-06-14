"""test_decoder.py — Tests for the AArch64 instruction decoder (decoder.py).

Tests decode() for each instruction encoding class:
  - HALT (0x00000000)
  - NOP (0xD503201F)
  - B / BL (unconditional branch immediate)
  - B.cond (conditional branch)
  - CBZ / CBNZ
  - TBZ / TBNZ
  - BR / BLR / RET
  - ADD/SUB immediate (sf=0 and sf=1)
  - MOVZ / MOVN / MOVK
  - AND/ORR/EOR/ANDS immediate (bitmask)
  - Load/Store unsigned offset
  - Logical shifted register (AND/ORR/EOR/BIC etc.)
  - Arithmetic shifted register (ADD/SUB register)
  - UDIV/SDIV/LSLV/LSRV/ASRV/RORV (data proc 2-source)
  - CLZ/RBIT/REV/REV16/REV32 (data proc 1-source)
  - MADD/MSUB (3-source)
  - CSEL/CSINC/CSINV/CSNEG (conditional select)
"""

import struct
import pytest
from aarch64_gatelevel.decoder import decode, AArch64Instruction


def _u32be(v: int) -> int:
    """Pack and unpack a 32-bit big-endian word to get the Python int."""
    return struct.unpack(">I", struct.pack(">I", v & 0xFFFFFFFF))[0]


# ── Encoding helpers (same as in the behavioral simulator) ────────────────────

def dp_imm(sf, op, S, imm12, sh, Rn, Rd):
    v = ((sf & 1) << 31) | ((op & 1) << 30) | ((S & 1) << 29)
    v |= (0b100000 << 23) | ((sh & 1) << 22)
    v |= ((imm12 & 0xFFF) << 10) | ((Rn & 0x1F) << 5) | (Rd & 0x1F)
    return v

def movwide(sf, opc, hw, imm16, Rd):
    v = ((sf & 1) << 31) | ((opc & 3) << 29)
    v |= (0b100101 << 23) | ((hw & 3) << 21)
    v |= ((imm16 & 0xFFFF) << 5) | (Rd & 0x1F)
    return v

def logic_imm(sf, opc, N, immr, imms, Rn, Rd):
    v = ((sf & 1) << 31) | ((opc & 3) << 29) | (0 << 28) | (0b100100 << 22)
    v |= ((N & 1) << 22) | ((immr & 0x3F) << 16) | ((imms & 0x3F) << 10)
    v |= ((Rn & 0x1F) << 5) | (Rd & 0x1F)
    return v

def logic_reg(sf, opc, shift, N, Rm, imm6, Rn, Rd):
    v = ((sf & 1) << 31) | ((opc & 3) << 29)
    v |= (0b01010 << 24) | ((shift & 3) << 22) | ((N & 1) << 21)
    v |= ((Rm & 0x1F) << 16) | ((imm6 & 0x3F) << 10)
    v |= ((Rn & 0x1F) << 5) | (Rd & 0x1F)
    return v

def dp_reg(sf, op, S, shift, Rm, imm6, Rn, Rd):
    v = ((sf & 1) << 31) | ((op & 1) << 30) | ((S & 1) << 29)
    v |= (0b01011 << 24) | ((shift & 3) << 22) | ((Rm & 0x1F) << 16)
    v |= ((imm6 & 0x3F) << 10) | ((Rn & 0x1F) << 5) | (Rd & 0x1F)
    return v

def branch_imm(op, imm26):
    v = ((op & 1) << 31) | (0b00101 << 26) | (imm26 & 0x3FF_FFFF)
    return v

def branch_cond(imm19, cond):
    v = (0b01010100 << 24) | ((imm19 & 0x7FFFF) << 5) | (cond & 0xF)
    return v

def cbz_cbnz(sf, op, imm19, Rt):
    v = ((sf & 1) << 31) | (0b011010 << 25) | ((op & 1) << 24)
    v |= ((imm19 & 0x7FFFF) << 5) | (Rt & 0x1F)
    return v

def tbz_tbnz(b5, op, b40, imm14, Rt):
    v = ((b5 & 1) << 31) | (0b011011 << 25) | ((op & 1) << 24)
    v |= ((b40 & 0x1F) << 19) | ((imm14 & 0x3FFF) << 5) | (Rt & 0x1F)
    return v

def branch_reg(op, Rn):
    v = (0b1101011_0 << 24) | ((op & 0x7) << 21) | (0b11111 << 16) | ((Rn & 0x1F) << 5)
    return v

def ldst_uoff(size, V, opc, imm12, Rn, Rt):
    v = ((size & 3) << 30) | (0b111 << 27) | ((V & 1) << 26) | (0b01 << 24)
    v |= ((opc & 3) << 22) | ((imm12 & 0xFFF) << 10)
    v |= ((Rn & 0x1F) << 5) | (Rt & 0x1F)
    return v

def madd_msub(sf, op54, Rm, o0, Ra, Rn, Rd):
    v = ((sf & 1) << 31) | (0b00_11011 << 24)
    v |= ((op54 & 7) << 21) | ((Rm & 0x1F) << 16)
    v |= ((o0 & 1) << 15) | ((Ra & 0x1F) << 10)
    v |= ((Rn & 0x1F) << 5) | (Rd & 0x1F)
    return v

def csel_enc(sf, op, S, Rm, cond, op2, Rn, Rd):
    v = ((sf & 1) << 31) | ((op & 1) << 30) | ((S & 1) << 29)
    v |= (0b11010100 << 21) | ((Rm & 0x1F) << 16)
    v |= ((cond & 0xF) << 12) | ((op2 & 3) << 10)
    v |= ((Rn & 0x1F) << 5) | (Rd & 0x1F)
    return v

def dp2src(sf, Rm, opc2, Rn, Rd):
    v = ((sf & 1) << 31) | (0b11010110 << 21)
    v |= ((Rm & 0x1F) << 16) | ((opc2 & 0x3F) << 10)
    v |= ((Rn & 0x1F) << 5) | (Rd & 0x1F)
    return v

def dp1src(sf, opc2, Rn, Rd):
    v = ((sf & 1) << 31) | (1 << 30) | (0b11010110 << 21) | (0 << 16)
    v |= ((opc2 & 0x3F) << 10) | ((Rn & 0x1F) << 5) | (Rd & 0x1F)
    return v


# ── Tests ─────────────────────────────────────────────────────────────────────


def test_decode_halt():
    d = decode(0)
    assert d.opcode == "HALT"


def test_decode_nop():
    d = decode(0xD503201F)
    assert d.opcode == "NOP"


def test_decode_b():
    raw = branch_imm(0, 5)   # B #+20
    d = decode(raw)
    assert d.opcode == "B"
    assert d.imm == 5


def test_decode_bl():
    raw = branch_imm(1, -3)  # BL #-12
    d = decode(raw)
    assert d.opcode == "BL"
    assert d.imm == -3


def test_decode_b_cond_eq():
    raw = branch_cond(10, 0)  # B.EQ #+40
    d = decode(raw)
    assert d.opcode == "B.EQ"
    assert d.imm == 10
    assert d.cond == 0


def test_decode_b_cond_ne():
    raw = branch_cond(-5, 1)  # B.NE #-20
    d = decode(raw)
    assert d.opcode == "B.NE"
    assert d.imm == -5
    assert d.cond == 1


def test_decode_b_cond_ge():
    raw = branch_cond(3, 0b1010)  # B.GE #+12
    d = decode(raw)
    assert d.opcode == "B.GE"
    assert d.cond == 0b1010


def test_decode_cbz():
    raw = cbz_cbnz(1, 0, -3, 5)  # CBZ X5, #-12
    d = decode(raw)
    assert d.opcode == "CBZ"
    assert d.Rd == 5
    assert d.imm == -3
    assert d.sf == 1


def test_decode_cbnz():
    raw = cbz_cbnz(0, 1, 2, 7)  # CBNZ W7, #+8
    d = decode(raw)
    assert d.opcode == "CBNZ"
    assert d.Rd == 7
    assert d.sf == 0


def test_decode_tbz():
    raw = tbz_tbnz(0, 0, 3, 4, 2)  # TBZ W2, #3, #+16
    d = decode(raw)
    assert d.opcode == "TBZ"
    assert d.Rd == 2
    assert d.bit_num == 3
    assert d.imm == 4


def test_decode_tbnz():
    raw = tbz_tbnz(1, 1, 0, -2, 0)  # TBNZ X0, #32, #-8
    d = decode(raw)
    assert d.opcode == "TBNZ"
    assert d.bit_num == 32


def test_decode_br():
    raw = branch_reg(0b000, 5)  # BR X5
    d = decode(raw)
    assert d.opcode == "BR"
    assert d.Rn == 5


def test_decode_blr():
    raw = branch_reg(0b001, 30)  # BLR X30
    d = decode(raw)
    assert d.opcode == "BLR"
    assert d.Rn == 30


def test_decode_ret():
    raw = branch_reg(0b010, 30)  # RET X30 (LR)
    d = decode(raw)
    assert d.opcode == "RET"
    assert d.Rn == 30


def test_decode_add_imm_64():
    raw = dp_imm(1, 0, 0, 5, 0, 1, 0)  # ADD X0, X1, #5
    d = decode(raw)
    assert d.opcode == "ADD"
    assert d.sf == 1
    assert d.Rd == 0
    assert d.Rn == 1
    assert d.imm == 5


def test_decode_adds_imm_32():
    raw = dp_imm(0, 0, 1, 100, 0, 2, 3)  # ADDS W3, W2, #100
    d = decode(raw)
    assert d.opcode == "ADDS"
    assert d.sf == 0
    assert d.Rd == 3
    assert d.Rn == 2
    assert d.imm == 100


def test_decode_sub_imm():
    raw = dp_imm(1, 1, 0, 1, 0, 5, 4)  # SUB X4, X5, #1
    d = decode(raw)
    assert d.opcode == "SUB"
    assert d.Rd == 4
    assert d.Rn == 5
    assert d.imm == 1


def test_decode_subs_imm_shifted():
    # SUBS X0, X0, #1<<12 (sh=1)
    raw = dp_imm(1, 1, 1, 1, 1, 0, 0)
    d = decode(raw)
    assert d.opcode == "SUBS"
    assert d.imm == 4096   # 1 << 12


def test_decode_movz_64():
    raw = movwide(1, 0b10, 0, 42, 0)  # MOVZ X0, #42
    d = decode(raw)
    assert d.opcode == "MOVZ"
    assert d.sf == 1
    assert d.Rd == 0
    assert d.imm == 42
    assert d.hw == 0


def test_decode_movn():
    raw = movwide(1, 0b00, 0, 0, 1)  # MOVN X1, #0
    d = decode(raw)
    assert d.opcode == "MOVN"
    assert d.Rd == 1


def test_decode_movk():
    raw = movwide(1, 0b11, 1, 0xABCD, 3)  # MOVK X3, #0xABCD, LSL #16
    d = decode(raw)
    assert d.opcode == "MOVK"
    assert d.Rd == 3
    assert d.imm == 0xABCD
    assert d.hw == 1


def test_decode_logic_imm_orr():
    # ORR X1, X0, #all-ones (N=1, immr=0, imms=62)
    raw = logic_imm(1, 0b01, 1, 0, 62, 0, 1)
    d = decode(raw)
    assert d.opcode == "ORR"
    assert d.sf == 1
    assert d.Rd == 1
    assert d.Rn == 0
    assert d.bitmask_imm != 0   # should decode to a mask


def test_decode_logic_imm_ands():
    raw = logic_imm(1, 0b11, 1, 0, 62, 1, 31)  # TST X1, #all
    d = decode(raw)
    assert d.opcode == "ANDS"
    assert d.Rd == 31  # XZR


def test_decode_ldr_64():
    raw = ldst_uoff(3, 0, 0b01, 0, 0, 1)  # LDR X1, [X0]
    d = decode(raw)
    assert d.opcode == "LDR"
    assert d.Rd == 1
    assert d.Rn == 0
    assert d.imm == 0
    assert d.size == 3


def test_decode_str_64():
    raw = ldst_uoff(3, 0, 0b00, 0, 1, 0)  # STR X0, [X1]
    d = decode(raw)
    assert d.opcode == "STR"
    assert d.Rd == 0
    assert d.Rn == 1


def test_decode_ldrb():
    raw = ldst_uoff(0, 0, 0b01, 5, 2, 3)  # LDRB W3, [X2, #5]
    d = decode(raw)
    assert d.opcode == "LDRB"
    assert d.Rd == 3
    assert d.Rn == 2
    assert d.imm == 5
    assert d.size == 0


def test_decode_ldrh():
    raw = ldst_uoff(1, 0, 0b01, 2, 5, 7)  # LDRH W7, [X5, #4]
    d = decode(raw)
    assert d.opcode == "LDRH"
    assert d.size == 1
    assert d.imm == 2


def test_decode_ldrsw():
    raw = ldst_uoff(2, 0, 0b10, 0, 5, 3)  # LDRSW X3, [X5]
    d = decode(raw)
    assert d.opcode == "LDRSW"
    assert d.size == 2


def test_decode_strb():
    raw = ldst_uoff(0, 0, 0b00, 1, 3, 0)  # STRB W0, [X3, #1]
    d = decode(raw)
    assert d.opcode == "STRB"


def test_decode_logic_reg_and():
    raw = logic_reg(1, 0b00, 0, 0, 2, 0, 1, 0)  # AND X0, X1, X2
    d = decode(raw)
    assert d.opcode == "AND"
    assert d.sf == 1
    assert d.Rd == 0
    assert d.Rn == 1
    assert d.Rm == 2


def test_decode_logic_reg_bic():
    raw = logic_reg(1, 0b00, 0, 1, 3, 0, 2, 4)  # BIC X4, X2, X3
    d = decode(raw)
    assert d.opcode == "BIC"
    assert d.N_bit == 1


def test_decode_logic_reg_orn():
    raw = logic_reg(1, 0b01, 0, 1, 5, 0, 6, 7)  # ORN X7, X6, X5
    d = decode(raw)
    assert d.opcode == "ORN"


def test_decode_arithmetic_reg_add():
    raw = dp_reg(1, 0, 0, 0, 2, 0, 1, 0)  # ADD X0, X1, X2
    d = decode(raw)
    assert d.opcode == "ADD"
    assert d.Rd == 0
    assert d.Rn == 1
    assert d.Rm == 2
    assert d.shift_type == 0
    assert d.shift_amount == 0


def test_decode_arithmetic_reg_subs():
    raw = dp_reg(1, 1, 1, 0, 3, 0, 4, 31)  # CMP X4, X3 (SUBS XZR,X4,X3)
    d = decode(raw)
    assert d.opcode == "SUBS"
    assert d.Rd == 31  # XZR
    assert d.S == 1


def test_decode_arithmetic_reg_shifted():
    raw = dp_reg(1, 0, 0, 1, 2, 4, 1, 0)  # ADD X0, X1, X2, LSR #4
    d = decode(raw)
    assert d.opcode == "ADD"
    assert d.shift_type == 1   # LSR
    assert d.shift_amount == 4


def test_decode_udiv():
    raw = dp2src(1, 3, 0b000010, 1, 0)  # UDIV X0, X1, X3
    d = decode(raw)
    assert d.opcode == "UDIV"
    assert d.Rn == 1
    assert d.Rm == 3
    assert d.Rd == 0


def test_decode_sdiv():
    raw = dp2src(1, 2, 0b000011, 1, 0)  # SDIV X0, X1, X2
    d = decode(raw)
    assert d.opcode == "SDIV"


def test_decode_lslv():
    raw = dp2src(1, 2, 0b001000, 1, 0)  # LSLV X0, X1, X2
    d = decode(raw)
    assert d.opcode == "LSLV"


def test_decode_rorv():
    raw = dp2src(1, 2, 0b001011, 1, 0)  # RORV X0, X1, X2
    d = decode(raw)
    assert d.opcode == "RORV"


def test_decode_clz():
    raw = dp1src(1, 0b000100, 5, 3)  # CLZ X3, X5
    d = decode(raw)
    assert d.opcode == "CLZ"
    assert d.Rn == 5
    assert d.Rd == 3


def test_decode_rev():
    raw = dp1src(1, 0b000010, 1, 0)  # REV X0, X1
    d = decode(raw)
    assert d.opcode == "REV"


def test_decode_rev32():
    raw = dp1src(1, 0b000011, 1, 0)  # REV32 X0, X1
    d = decode(raw)
    assert d.opcode == "REV32"


def test_decode_rev16():
    raw = dp1src(1, 0b000001, 2, 1)  # REV16 X1, X2
    d = decode(raw)
    assert d.opcode == "REV16"


def test_decode_madd():
    raw = madd_msub(1, 0, 3, 0, 31, 2, 0)  # MUL X0, X2, X3 (MADD Ra=XZR)
    d = decode(raw)
    assert d.opcode == "MADD"
    assert d.Rn == 2
    assert d.Rm == 3
    assert d.Ra == 31
    assert d.o0 == 0


def test_decode_msub():
    raw = madd_msub(1, 0, 3, 1, 5, 2, 0)  # MSUB X0, X2, X3, X5
    d = decode(raw)
    assert d.opcode == "MSUB"
    assert d.o0 == 1
    assert d.Ra == 5


def test_decode_csel():
    raw = csel_enc(1, 0, 0, 2, 0b0000, 0b00, 1, 0)  # CSEL X0, X1, X2, EQ
    d = decode(raw)
    assert d.opcode == "CSEL"
    assert d.Rn == 1
    assert d.Rm == 2
    assert d.cond == 0b0000


def test_decode_csinc():
    raw = csel_enc(1, 0, 0, 2, 0b0001, 0b01, 1, 0)  # CSINC X0, X1, X2, NE
    d = decode(raw)
    assert d.opcode == "CSINC"
    assert d.cond == 0b0001


def test_decode_csinv():
    raw = csel_enc(1, 1, 0, 2, 0b1010, 0b00, 1, 0)  # CSINV X0, X1, X2, GE
    d = decode(raw)
    assert d.opcode == "CSINV"


def test_decode_csneg():
    raw = csel_enc(1, 1, 0, 2, 0b1011, 0b01, 1, 0)  # CSNEG X0, X1, X2, LT
    d = decode(raw)
    assert d.opcode == "CSNEG"
