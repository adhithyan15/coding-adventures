"""Tests for decoder.py — instruction decoding."""



from motorola68k_gatelevel.decoder import decode


def make_mem(*words: int) -> bytearray:
    """Pack a sequence of 16-bit big-endian words into a bytearray."""
    mem = bytearray(0x002000)
    for i, w in enumerate(words):
        mem[0x1000 + i * 2] = (w >> 8) & 0xFF
        mem[0x1000 + i * 2 + 1] = w & 0xFF
    return mem


PC = 0x1000


class TestMOVE:
    def test_move_b(self):
        # MOVE.B D0, D1 — 0x1200
        d = decode(make_mem(0x1200), PC)
        assert "MOVE" in d.mnemonic
        assert d.size == 1

    def test_move_w(self):
        # MOVE.W D0, D1 — 0x3200
        d = decode(make_mem(0x3200), PC)
        assert "MOVE" in d.mnemonic
        assert d.size == 2

    def test_move_l(self):
        # MOVE.L D0, D1 — 0x2200
        d = decode(make_mem(0x2200), PC)
        assert "MOVE" in d.mnemonic
        assert d.size == 4

    def test_movea_w(self):
        # MOVEA.W D0, A0 — 0x3040
        d = decode(make_mem(0x3040), PC)
        assert "MOVEA" in d.mnemonic

    def test_movea_l(self):
        # MOVEA.L D0, A0 — 0x2040
        d = decode(make_mem(0x2040), PC)
        assert "MOVEA" in d.mnemonic
        assert d.size == 4


class TestMOVEQ:
    def test_moveq(self):
        # MOVEQ #5, D0 — 0x7005
        d = decode(make_mem(0x7005), PC)
        assert "MOVEQ" in d.mnemonic
        assert d.size == 4


class TestADD:
    def test_add_l(self):
        # ADD.L D1, D0 — 0xD081
        d = decode(make_mem(0xD081), PC)
        assert "ADD" in d.mnemonic

    def test_adda_w(self):
        # ADDA.W D0, A0 — 0xD0C0
        d = decode(make_mem(0xD0C0), PC)
        assert "ADDA" in d.mnemonic

    def test_addi(self):
        # ADDI.L #5, D0 — 0x0680 0x0000 0x0005
        d = decode(make_mem(0x0680, 0x0000, 0x0005), PC)
        assert "ADDI" in d.mnemonic
        assert d.size == 4
        assert d.byte_length == 6


class TestSUB:
    def test_sub_l(self):
        d = decode(make_mem(0x9081), PC)
        assert "SUB" in d.mnemonic

    def test_suba_w(self):
        d = decode(make_mem(0x90C0), PC)
        assert "SUBA" in d.mnemonic


class TestLogic:
    def test_and_l(self):
        d = decode(make_mem(0xC081), PC)
        assert "AND" in d.mnemonic

    def test_or_l(self):
        d = decode(make_mem(0x8081), PC)
        assert "OR" in d.mnemonic

    def test_eor_l(self):
        d = decode(make_mem(0xB181), PC)
        assert "EOR" in d.mnemonic

    def test_andi(self):
        d = decode(make_mem(0x0280, 0x0000, 0x00FF), PC)
        assert "ANDI" in d.mnemonic

    def test_ori(self):
        d = decode(make_mem(0x0080, 0x0000, 0x00FF), PC)
        assert "ORI" in d.mnemonic

    def test_eori(self):
        d = decode(make_mem(0x0A80, 0x0000, 0x00FF), PC)
        assert "EORI" in d.mnemonic


class TestCMP:
    def test_cmp_l(self):
        d = decode(make_mem(0xB081), PC)
        assert "CMP" in d.mnemonic

    def test_cmpa(self):
        d = decode(make_mem(0xB0C0), PC)
        assert "CMPA" in d.mnemonic

    def test_cmpi(self):
        d = decode(make_mem(0x0C80, 0x0000, 0x0005), PC)
        assert "CMPI" in d.mnemonic


class TestMisc:
    def test_nop(self):
        d = decode(make_mem(0x4E71), PC)
        assert d.mnemonic == "NOP"
        assert d.byte_length == 2

    def test_rts(self):
        d = decode(make_mem(0x4E75), PC)
        assert d.mnemonic == "RTS"

    def test_rtr(self):
        d = decode(make_mem(0x4E77), PC)
        assert d.mnemonic == "RTR"

    def test_rte(self):
        d = decode(make_mem(0x4E73), PC)
        assert d.mnemonic == "RTE"

    def test_reset(self):
        d = decode(make_mem(0x4E70), PC)
        assert d.mnemonic == "RESET"

    def test_illegal(self):
        d = decode(make_mem(0x4AFC), PC)
        assert d.mnemonic == "ILLEGAL"

    def test_trap(self):
        d = decode(make_mem(0x4E4F), PC)
        assert "TRAP" in d.mnemonic

    def test_stop(self):
        d = decode(make_mem(0x4E72, 0x2700), PC)
        assert "STOP" in d.mnemonic
        assert d.byte_length == 4

    def test_link(self):
        d = decode(make_mem(0x4E50, 0xFFFC), PC)
        assert "LINK" in d.mnemonic
        assert d.byte_length == 4

    def test_unlk(self):
        d = decode(make_mem(0x4E58), PC)
        assert "UNLK" in d.mnemonic

    def test_jsr(self):
        d = decode(make_mem(0x4E90), PC)  # JSR (A0)
        assert "JSR" in d.mnemonic

    def test_jmp(self):
        d = decode(make_mem(0x4ED0), PC)  # JMP (A0)
        assert "JMP" in d.mnemonic

    def test_lea(self):
        d = decode(make_mem(0x41D0), PC)  # LEA (A0), A0
        assert "LEA" in d.mnemonic

    def test_pea(self):
        d = decode(make_mem(0x4850), PC)  # PEA (A0)
        assert "PEA" in d.mnemonic

    def test_clr(self):
        d = decode(make_mem(0x4280), PC)  # CLR.L D0
        assert "CLR" in d.mnemonic

    def test_neg(self):
        d = decode(make_mem(0x4480), PC)  # NEG.L D0
        assert "NEG" in d.mnemonic

    def test_not(self):
        d = decode(make_mem(0x4680), PC)  # NOT.L D0
        assert "NOT" in d.mnemonic

    def test_tst(self):
        d = decode(make_mem(0x4A80), PC)  # TST.L D0
        assert "TST" in d.mnemonic

    def test_swap(self):
        d = decode(make_mem(0x4840), PC)  # SWAP D0
        assert "SWAP" in d.mnemonic

    def test_ext_w(self):
        d = decode(make_mem(0x4880), PC)  # EXT.W D0
        assert "EXT" in d.mnemonic

    def test_ext_l(self):
        d = decode(make_mem(0x48C0), PC)  # EXT.L D0
        assert "EXT" in d.mnemonic

    def test_negx(self):
        d = decode(make_mem(0x4080), PC)  # NEGX.L D0
        assert "NEGX" in d.mnemonic


class TestBranches:
    def test_bra_short(self):
        d = decode(make_mem(0x6002), PC)  # BRA +2
        assert d.mnemonic == "BRA"
        assert d.byte_length == 2

    def test_bra_long(self):
        d = decode(make_mem(0x6000, 0x0010), PC)  # BRA.W
        assert "BRA" in d.mnemonic
        assert d.byte_length == 4

    def test_bsr(self):
        d = decode(make_mem(0x6102), PC)
        assert "BSR" in d.mnemonic

    def test_beq(self):
        d = decode(make_mem(0x6702), PC)  # BEQ +2
        assert "EQ" in d.mnemonic

    def test_bne(self):
        d = decode(make_mem(0x6602), PC)
        assert "NE" in d.mnemonic

    def test_bcc(self):
        d = decode(make_mem(0x6402), PC)  # BCC
        assert "CC" in d.mnemonic


class TestAddQ_SubQ:
    def test_addq(self):
        d = decode(make_mem(0x5280), PC)  # ADDQ.L #1, D0
        assert "ADDQ" in d.mnemonic

    def test_subq(self):
        d = decode(make_mem(0x5380), PC)  # SUBQ.L #1, D0
        assert "SUBQ" in d.mnemonic

    def test_dbcc(self):
        d = decode(make_mem(0x51C8, 0xFFFE), PC)  # DBF D0, -2
        assert "DB" in d.mnemonic
        assert d.byte_length == 4


class TestShifts:
    def test_asl(self):
        d = decode(make_mem(0xE180), PC)  # ASL.L #1, D0
        assert "ASL" in d.mnemonic

    def test_asr(self):
        d = decode(make_mem(0xE080), PC)  # ASR.L #1, D0
        assert "ASR" in d.mnemonic

    def test_lsl(self):
        d = decode(make_mem(0xE388), PC)  # LSL.L #1, D0
        assert "LSL" in d.mnemonic

    def test_lsr(self):
        d = decode(make_mem(0xE288), PC)  # LSR.L #1, D0
        assert "LSR" in d.mnemonic

    def test_rol(self):
        d = decode(make_mem(0xE398), PC)  # ROL.L #1, D0
        assert "ROL" in d.mnemonic

    def test_ror(self):
        d = decode(make_mem(0xE298), PC)  # ROR.L #1, D0
        assert "ROR" in d.mnemonic

    def test_roxl(self):
        d = decode(make_mem(0xE390), PC)  # ROXL.L #1, D0
        assert "ROXL" in d.mnemonic

    def test_roxr(self):
        d = decode(make_mem(0xE290), PC)  # ROXR.L #1, D0
        assert "ROXR" in d.mnemonic


class TestMul_Div:
    def test_mulu(self):
        d = decode(make_mem(0xC0C0), PC)  # MULU D0, D0
        assert "MULU" in d.mnemonic

    def test_muls(self):
        d = decode(make_mem(0xC1C0), PC)  # MULS D0, D0
        assert "MULS" in d.mnemonic

    def test_divu(self):
        d = decode(make_mem(0x80C0), PC)  # DIVU D0, D0
        assert "DIVU" in d.mnemonic

    def test_divs(self):
        d = decode(make_mem(0x81C0), PC)  # DIVS D0, D0
        assert "DIVS" in d.mnemonic


class TestBitOps:
    def test_btst_imm(self):
        d = decode(make_mem(0x0800, 0x0007), PC)  # BTST #7, D0
        assert "BTST" in d.mnemonic

    def test_bset_imm(self):
        d = decode(make_mem(0x08C0, 0x0007), PC)  # BSET #7, D0
        assert "BSET" in d.mnemonic

    def test_bclr_reg(self):
        d = decode(make_mem(0x0180), PC)  # BCLR D0, D0
        assert "BCLR" in d.mnemonic


class TestByteLengths:
    """Verify byte_length accounts for extension words."""

    def test_move_with_displacement(self):
        # MOVE.W d16(A0), D0 — opword + 1 ext word
        d = decode(make_mem(0x3028, 0x0010), PC)
        assert d.byte_length == 4

    def test_move_with_abs_long(self):
        # MOVE.L (abs).L, D0 — opword + 2 ext words
        d = decode(make_mem(0x2039, 0x0010, 0x0000), PC)
        assert d.byte_length == 6
