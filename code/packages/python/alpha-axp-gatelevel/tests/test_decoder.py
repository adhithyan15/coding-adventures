"""test_decoder.py — Unit tests for the instruction decoder.

Tests:
  - PALcode format (HALT word)
  - Memory format (LDQ, STQ, LDA, LDAH)
  - Branch format (BEQ, BNE, BR, BSR)
  - Operate format — register variant (i_bit=0)
  - Operate format — literal variant (i_bit=1)
  - Jump format (JMP, JSR, RET)
  - All field values are correctly extracted
"""

from __future__ import annotations

from alpha_axp_gatelevel.decoder import decode_instruction

# ── Encoding helpers ──────────────────────────────────────────────────────────

def enc_mem(op, ra, rb, disp):
    """Encode a memory-format instruction."""
    return (op << 26) | (ra << 21) | (rb << 16) | (disp & 0xFFFF)


def enc_branch(op, ra, disp21):
    """Encode a branch-format instruction."""
    return (op << 26) | (ra << 21) | (disp21 & 0x1F_FFFF)


def enc_operate_reg(op, ra, rb, func, rc):
    """Encode an operate-format instruction (register operand, i_bit=0)."""
    return (op << 26) | (ra << 21) | (rb << 16) | (0 << 12) | (func << 5) | rc


def enc_operate_lit(op, ra, lit8, func, rc):
    """Encode an operate-format instruction (literal operand, i_bit=1)."""
    return (op << 26) | (ra << 21) | (lit8 << 13) | (1 << 12) | (func << 5) | rc


def enc_jump(ra, rb, func, hint=0):
    """Encode a jump-format instruction."""
    return (0x1A << 26) | (ra << 21) | (rb << 16) | (func << 14) | (hint & 0x3FFF)


# ── PALcode ───────────────────────────────────────────────────────────────────

class TestPALcode:
    def test_halt_all_zeros(self):
        d = decode_instruction(0x00000000)
        assert d["op"] == 0x00
        assert d["palcode"] == 0
        assert d["mnemonic"] == "HALT"

    def test_halt_fields(self):
        d = decode_instruction(0x00000000)
        assert d["ra"] == 0
        assert d["rb"] == 0


# ── Memory format ─────────────────────────────────────────────────────────────

class TestMemoryFormat:
    def test_lda(self):
        # LDA r1, 100(r2)
        word = enc_mem(0x08, 1, 2, 100)
        d = decode_instruction(word)
        assert d["op"] == 0x08
        assert d["ra"] == 1
        assert d["rb"] == 2
        assert d["disp16"] == 100
        assert d["mnemonic"] == "LDA"

    def test_ldah(self):
        # LDAH r5, -1(r6)
        word = enc_mem(0x09, 5, 6, 0xFFFF)  # -1 as 16-bit
        d = decode_instruction(word)
        assert d["op"] == 0x09
        assert d["ra"] == 5
        assert d["rb"] == 6
        assert d["disp16"] == -1
        assert d["mnemonic"] == "LDAH"

    def test_ldq(self):
        word = enc_mem(0x29, 3, 4, 8)
        d = decode_instruction(word)
        assert d["op"] == 0x29
        assert d["ra"] == 3
        assert d["rb"] == 4
        assert d["disp16"] == 8
        assert d["mnemonic"] == "LDQ"

    def test_stq(self):
        word = enc_mem(0x2D, 7, 8, 16)
        d = decode_instruction(word)
        assert d["op"] == 0x2D
        assert d["ra"] == 7
        assert d["rb"] == 8
        assert d["disp16"] == 16
        assert d["mnemonic"] == "STQ"

    def test_ldl(self):
        word = enc_mem(0x28, 1, 2, 0)
        d = decode_instruction(word)
        assert d["mnemonic"] == "LDL"

    def test_stl(self):
        word = enc_mem(0x2C, 1, 2, 0)
        d = decode_instruction(word)
        assert d["mnemonic"] == "STL"

    def test_negative_displacement(self):
        # disp16 = -4 = 0xFFFC
        word = enc_mem(0x29, 0, 30, 0xFFFC)
        d = decode_instruction(word)
        assert d["disp16"] == -4

    def test_zero_displacement(self):
        word = enc_mem(0x29, 1, 2, 0)
        d = decode_instruction(word)
        assert d["disp16"] == 0


# ── Branch format ─────────────────────────────────────────────────────────────

class TestBranchFormat:
    def test_beq(self):
        # BEQ r1, +4 instructions
        word = enc_branch(0x39, 1, 4)
        d = decode_instruction(word)
        assert d["op"] == 0x39
        assert d["ra"] == 1
        assert d["disp21"] == 4
        assert d["mnemonic"] == "BEQ"

    def test_bne(self):
        word = enc_branch(0x3D, 2, 10)
        d = decode_instruction(word)
        assert d["op"] == 0x3D
        assert d["ra"] == 2
        assert d["disp21"] == 10
        assert d["mnemonic"] == "BNE"

    def test_br(self):
        word = enc_branch(0x30, 31, 0)
        d = decode_instruction(word)
        assert d["mnemonic"] == "BR"
        assert d["ra"] == 31

    def test_bsr(self):
        word = enc_branch(0x34, 26, 5)  # r26 = link register
        d = decode_instruction(word)
        assert d["mnemonic"] == "BSR"
        assert d["ra"] == 26
        assert d["disp21"] == 5

    def test_negative_displacement(self):
        # Backward branch: disp21 = -1 = 0x1FFFFF
        word = enc_branch(0x39, 1, 0x1F_FFFF)
        d = decode_instruction(word)
        assert d["disp21"] == -1

    def test_blt(self):
        word = enc_branch(0x3A, 3, 2)
        d = decode_instruction(word)
        assert d["mnemonic"] == "BLT"

    def test_bgt(self):
        word = enc_branch(0x3F, 4, 3)
        d = decode_instruction(word)
        assert d["mnemonic"] == "BGT"


# ── Operate format — register variant ────────────────────────────────────────

class TestOperateRegister:
    def test_addq(self):
        # ADDQ r1, r2, r3
        word = enc_operate_reg(0x10, 1, 2, 0x20, 3)
        d = decode_instruction(word)
        assert d["op"] == 0x10
        assert d["ra"] == 1
        assert d["rb"] == 2
        assert d["func7"] == 0x20
        assert d["rc"] == 3
        assert d["i_bit"] == 0
        assert d["mnemonic"] == "ADDQ"

    def test_subq(self):
        word = enc_operate_reg(0x10, 5, 6, 0x29, 7)
        d = decode_instruction(word)
        assert d["mnemonic"] == "SUBQ"

    def test_and(self):
        word = enc_operate_reg(0x11, 1, 2, 0x00, 3)
        d = decode_instruction(word)
        assert d["mnemonic"] == "AND"

    def test_bis(self):
        word = enc_operate_reg(0x11, 1, 2, 0x20, 3)
        d = decode_instruction(word)
        assert d["mnemonic"] == "BIS"

    def test_xor(self):
        word = enc_operate_reg(0x11, 1, 2, 0x40, 3)
        d = decode_instruction(word)
        assert d["mnemonic"] == "XOR"

    def test_sll(self):
        word = enc_operate_reg(0x12, 1, 2, 0x39, 3)
        d = decode_instruction(word)
        assert d["mnemonic"] == "SLL"

    def test_mulq(self):
        word = enc_operate_reg(0x13, 1, 2, 0x20, 3)
        d = decode_instruction(word)
        assert d["mnemonic"] == "MULQ"


# ── Operate format — literal variant ─────────────────────────────────────────

class TestOperateLiteral:
    def test_addq_lit(self):
        # ADDQ r1, #10, r2
        word = enc_operate_lit(0x10, 1, 10, 0x20, 2)
        d = decode_instruction(word)
        assert d["op"] == 0x10
        assert d["ra"] == 1
        assert d["lit8"] == 10
        assert d["i_bit"] == 1
        assert d["rc"] == 2
        assert d["mnemonic"] == "ADDQ"

    def test_bis_lit_zero(self):
        # BIS r31, #0, r0 — NOP idiom
        word = enc_operate_lit(0x11, 31, 0, 0x20, 0)
        d = decode_instruction(word)
        assert d["i_bit"] == 1
        assert d["lit8"] == 0
        assert d["ra"] == 31

    def test_bis_lit_max(self):
        # BIS r31, #255, r0 — load 255
        word = enc_operate_lit(0x11, 31, 255, 0x20, 0)
        d = decode_instruction(word)
        assert d["lit8"] == 255

    def test_i_bit_distinguishes(self):
        reg_word = enc_operate_reg(0x10, 1, 2, 0x20, 3)
        lit_word = enc_operate_lit(0x10, 1, 5, 0x20, 3)
        d_reg = decode_instruction(reg_word)
        d_lit = decode_instruction(lit_word)
        assert d_reg["i_bit"] == 0
        assert d_lit["i_bit"] == 1


# ── Jump format ───────────────────────────────────────────────────────────────

class TestJumpFormat:
    def test_jmp(self):
        # JMP r31, (r26) — jump to r26, discard link
        word = enc_jump(ra=31, rb=26, func=0)
        d = decode_instruction(word)
        assert d["op"] == 0x1A
        assert d["ra"] == 31
        assert d["rb"] == 26
        assert d["jump_func"] == 0
        assert d["mnemonic"] == "JMP"

    def test_jsr(self):
        # JSR r26, (r27) — indirect call
        word = enc_jump(ra=26, rb=27, func=1)
        d = decode_instruction(word)
        assert d["jump_func"] == 1
        assert d["mnemonic"] == "JSR"

    def test_ret(self):
        # RET r31, (r26) — return
        word = enc_jump(ra=31, rb=26, func=2)
        d = decode_instruction(word)
        assert d["jump_func"] == 2
        assert d["mnemonic"] == "RET"

    def test_jsr_coroutine(self):
        word = enc_jump(ra=26, rb=27, func=3)
        d = decode_instruction(word)
        assert d["jump_func"] == 3
        assert d["mnemonic"] == "JSR_COROUTINE"
