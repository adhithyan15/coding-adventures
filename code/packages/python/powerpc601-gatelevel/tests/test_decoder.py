"""test_decoder.py — Unit tests for the PowerPC 601 instruction decoder.

Tests cover:
- HALT (0x00000000)
- D-form: ADDI, LWZ, STW, CMPI
- I-form: B, BL
- B-form: BEQ, BNE
- XO-form: ADD, SUBF, MULLW, DIVWU
- X-form: AND, OR, SLW, CMP
- XFX-form: MFSPR, MTSPR
"""

from __future__ import annotations

import struct

from powerpc601_gatelevel.decoder import decode_instruction


def pack(word: int) -> int:
    """Return a 32-bit big-endian integer."""
    return int.from_bytes(struct.pack(">I", word), "big")


# ── Encoding helpers ──────────────────────────────────────────────────────────

def d_form(op: int, rd: int, ra: int, imm: int) -> int:
    return (op << 26) | (rd << 21) | (ra << 16) | (imm & 0xFFFF)

def i_form(op: int, li: int, aa: int = 0, lk: int = 0) -> int:
    LI = (li >> 2) & 0xFFFFFF
    return (op << 26) | (LI << 2) | (aa << 1) | lk

def b_form(op: int, bo: int, bi: int, bd: int, aa: int = 0, lk: int = 0) -> int:
    BD = (bd >> 2) & 0x3FFF
    return (op << 26) | (bo << 21) | (bi << 16) | (BD << 2) | (aa << 1) | lk

def x_form(op: int, rs: int, ra: int, rb: int, xo: int, rc: int = 0) -> int:
    return (op << 26) | (rs << 21) | (ra << 16) | (rb << 11) | (xo << 1) | rc

def xo_form(op: int, rd: int, ra: int, rb: int, oe: int, xo: int, rc: int = 0) -> int:
    return (op << 26) | (rd << 21) | (ra << 16) | (rb << 11) | (oe << 10) | (xo << 1) | rc

def xfx_form(op: int, rs: int, spr: int, xo: int) -> int:
    spr_enc = ((spr & 0x1F) << 5) | ((spr >> 5) & 0x1F)
    return (op << 26) | (rs << 21) | (spr_enc << 11) | (xo << 1)


# ── HALT ──────────────────────────────────────────────────────────────────────

class TestHalt:
    def test_zero_word(self):
        d = decode_instruction(0)
        assert d["mnemonic"] == "halt"
        assert d["op"] == 0

    def test_all_fields_zero(self):
        d = decode_instruction(0)
        assert d["rd"] == 0
        assert d["ra"] == 0
        assert d["simm"] == 0


# ── D-form ────────────────────────────────────────────────────────────────────

class TestDFormADDI:
    def test_addi_r3_r0_1(self):
        word = d_form(14, 3, 0, 1)
        d = decode_instruction(word)
        assert d["op"] == 14
        assert d["rd"] == 3
        assert d["ra"] == 0
        assert d["simm"] == 1
        assert d["mnemonic"] == "addi"

    def test_addi_negative_simm(self):
        word = d_form(14, 5, 1, -1)
        d = decode_instruction(word)
        assert d["simm"] == -1

    def test_addi_large_simm(self):
        word = d_form(14, 3, 0, 32767)
        d = decode_instruction(word)
        assert d["simm"] == 32767

    def test_addi_max_neg_simm(self):
        word = d_form(14, 3, 0, -32768)
        d = decode_instruction(word)
        assert d["simm"] == -32768


class TestDFormLWZ:
    def test_lwz_r3_0_r1(self):
        word = d_form(32, 3, 1, 0)
        d = decode_instruction(word)
        assert d["op"] == 32
        assert d["mnemonic"] == "lwz"
        assert d["rd"] == 3
        assert d["ra"] == 1

    def test_lwz_with_offset(self):
        word = d_form(32, 4, 2, 8)
        d = decode_instruction(word)
        assert d["simm"] == 8


class TestDFormSTW:
    def test_stw(self):
        word = d_form(36, 5, 1, -4)
        d = decode_instruction(word)
        assert d["op"] == 36
        assert d["mnemonic"] == "stw"
        assert d["rd"] == 5  # rS is at rd field
        assert d["ra"] == 1
        assert d["simm"] == -4


class TestDFormCMPI:
    def test_cmpi_cr0(self):
        # cmpi cr0, r3, 5 — op=11, rd=0 (crfD shifted), ra=3, simm=5
        word = d_form(11, 0, 3, 5)
        d = decode_instruction(word)
        assert d["op"] == 11
        assert d["mnemonic"] == "cmpi"
        assert d["ra"] == 3
        assert d["simm"] == 5


# ── I-form ────────────────────────────────────────────────────────────────────

class TestIFormBranch:
    def test_b_forward(self):
        word = i_form(18, 12, aa=0, lk=0)
        d = decode_instruction(word)
        assert d["op"] == 18
        assert d["mnemonic"] == "b"
        assert d["li"] == 12
        assert d["lk"] == 0
        assert d["aa"] == 0

    def test_bl_forward(self):
        word = i_form(18, 16, aa=0, lk=1)
        d = decode_instruction(word)
        assert d["mnemonic"] == "bl"
        assert d["lk"] == 1
        assert d["li"] == 16

    def test_ba_absolute(self):
        word = i_form(18, 0x1000, aa=1, lk=0)
        d = decode_instruction(word)
        assert d["mnemonic"] == "ba"
        assert d["aa"] == 1

    def test_bla_absolute_link(self):
        word = i_form(18, 0x1000, aa=1, lk=1)
        d = decode_instruction(word)
        assert d["mnemonic"] == "bla"

    def test_negative_offset(self):
        # -4 in 26-bit field
        word = i_form(18, -4, aa=0, lk=0)
        d = decode_instruction(word)
        assert d["li"] == -4


# ── B-form ────────────────────────────────────────────────────────────────────

class TestBFormBranch:
    def test_beq(self):
        # bc BO_TRUE, BI_EQ, offset
        word = b_form(16, 18, 2, 8)  # BO=18 (true), BI=2 (EQ), offset=8
        d = decode_instruction(word)
        assert d["op"] == 16
        assert d["mnemonic"] == "bc"
        assert d["bo"] == 18
        assert d["bi"] == 2
        assert d["bd"] == 8

    def test_bne(self):
        # bc BO_FALSE, BI_EQ, offset
        word = b_form(16, 16, 2, 8)  # BO=16 (false), BI=2 (EQ)
        d = decode_instruction(word)
        assert d["bo"] == 16
        assert d["bi"] == 2

    def test_bdnz(self):
        # bc BO_BDNZ, 0, offset
        word = b_form(16, 4, 0, -8)
        d = decode_instruction(word)
        assert d["bo"] == 4
        assert d["bd"] == -8

    def test_bcl_link(self):
        word = b_form(16, 18, 2, 8, aa=0, lk=1)
        d = decode_instruction(word)
        assert d["lk"] == 1


# ── XO-form ───────────────────────────────────────────────────────────────────

class TestXOFormArith:
    def test_add(self):
        word = xo_form(31, 3, 4, 5, 0, 266, 0)
        d = decode_instruction(word)
        assert d["op"] == 31
        assert d["xo9"] == 266
        assert d["rd"] == 3
        assert d["ra"] == 4
        assert d["rb"] == 5
        assert d["oe"] == 0
        assert d["rc"] == 0
        assert "add" in d["mnemonic"]

    def test_add_dot(self):
        word = xo_form(31, 3, 4, 5, 0, 266, 1)
        d = decode_instruction(word)
        assert d["rc"] == 1
        assert "add." in d["mnemonic"]

    def test_add_oe(self):
        word = xo_form(31, 3, 4, 5, 1, 266, 0)
        d = decode_instruction(word)
        assert d["oe"] == 1
        assert "addo" in d["mnemonic"]

    def test_subf(self):
        word = xo_form(31, 3, 4, 5, 0, 40, 0)
        d = decode_instruction(word)
        assert d["xo9"] == 40
        assert "subf" in d["mnemonic"]

    def test_mullw(self):
        word = xo_form(31, 3, 4, 5, 0, 235, 0)
        d = decode_instruction(word)
        assert d["xo9"] == 235
        assert "mullw" in d["mnemonic"]

    def test_divwu(self):
        word = xo_form(31, 3, 4, 5, 0, 459, 0)
        d = decode_instruction(word)
        assert d["xo9"] == 459
        assert "divwu" in d["mnemonic"]

    def test_neg(self):
        word = xo_form(31, 3, 4, 0, 0, 104, 0)
        d = decode_instruction(word)
        assert d["xo9"] == 104
        assert "neg" in d["mnemonic"]


# ── X-form ────────────────────────────────────────────────────────────────────

class TestXFormLogic:
    def test_and(self):
        word = x_form(31, 3, 4, 5, 28, 0)
        d = decode_instruction(word)
        assert d["op"] == 31
        assert d["xo"] == 28
        assert "and" in d["mnemonic"]

    def test_and_dot(self):
        word = x_form(31, 3, 4, 5, 28, 1)
        d = decode_instruction(word)
        assert d["rc"] == 1
        assert d["mnemonic"] == "and."

    def test_or(self):
        word = x_form(31, 3, 4, 5, 444, 0)
        d = decode_instruction(word)
        assert "or" in d["mnemonic"]

    def test_slw(self):
        word = x_form(31, 3, 4, 5, 24, 0)
        d = decode_instruction(word)
        assert d["xo"] == 24
        assert "slw" in d["mnemonic"]

    def test_cmp(self):
        word = x_form(31, 0, 3, 5, 0, 0)  # CMP crfD=0, rA=3, rB=5
        d = decode_instruction(word)
        assert d["xo"] == 0
        assert d["ra"] == 3
        assert d["rb"] == 5
        assert "cmp" in d["mnemonic"]


# ── XFX-form ──────────────────────────────────────────────────────────────────

class TestXFXForm:
    def test_mfspr_lr(self):
        word = xfx_form(31, 3, 8, 339)  # mfspr r3, LR
        d = decode_instruction(word)
        assert d["op"] == 31
        assert d["xo"] == 339
        assert d["spr"] == 8  # LR
        assert d["rd"] == 3

    def test_mfspr_ctr(self):
        word = xfx_form(31, 4, 9, 339)  # mfspr r4, CTR
        d = decode_instruction(word)
        assert d["spr"] == 9

    def test_mtspr_lr(self):
        word = xfx_form(31, 3, 8, 467)  # mtspr LR, r3
        d = decode_instruction(word)
        assert d["xo"] == 467
        assert d["spr"] == 8

    def test_mtspr_ctr(self):
        word = xfx_form(31, 5, 9, 467)  # mtspr CTR, r5
        d = decode_instruction(word)
        assert d["spr"] == 9


# ── Rotate/mask fields ────────────────────────────────────────────────────────

class TestMFormFields:
    def test_rlwinm_fields(self):
        # rlwinm r4, r5, 8, 0, 23
        # op=21, rs=5, ra=4, sh=8, mb=0, me=23
        word = (21 << 26) | (5 << 21) | (4 << 16) | (8 << 11) | (0 << 6) | (23 << 1)
        d = decode_instruction(word)
        assert d["op"] == 21
        assert d["mnemonic"] == "rlwinm"
        assert d["rd"] == 5   # rS
        assert d["ra"] == 4
        assert d["rb"] == 8   # SH in rB slot
        assert d["mb"] == 0
        assert d["me"] == 23

    def test_rlwimi_fields(self):
        word = (20 << 26) | (5 << 21) | (4 << 16) | (1 << 11) | (0 << 6) | (31 << 1)
        d = decode_instruction(word)
        assert d["op"] == 20
        assert d["mnemonic"] == "rlwimi"

    def test_rlwnm_fields(self):
        word = (23 << 26) | (5 << 21) | (4 << 16) | (3 << 11) | (0 << 6) | (31 << 1)
        d = decode_instruction(word)
        assert d["op"] == 23
        assert d["mnemonic"] == "rlwnm"
