"""Tests for intel8051_gatelevel.decoder — gate-tree instruction decoder."""

from intel8051_gatelevel.decoder import decode_opcode


class TestDecodeNopHalt:
    def test_nop(self):
        assert decode_opcode(0x00) == "NOP"

    def test_halt(self):
        assert decode_opcode(0xA5) == "HALT"


class TestDecodeMovFamily:
    def test_mov_a_imm(self):
        assert decode_opcode(0x74) == "MOV A,#imm"

    def test_mov_a_dir(self):
        assert decode_opcode(0xE5) == "MOV A,dir"

    def test_mov_a_at_r0(self):
        assert decode_opcode(0xE6) == "MOV A,@R0"

    def test_mov_a_at_r1(self):
        assert decode_opcode(0xE7) == "MOV A,@R1"

    def test_mov_a_rn(self):
        # 0xE8-0xEF all decode to MOV A,Rn
        for op in range(0xE8, 0xF0):
            assert decode_opcode(op) == "MOV A,Rn"

    def test_mov_rn_a(self):
        for op in range(0xF8, 0x100):
            assert decode_opcode(op) == "MOV Rn,A"

    def test_mov_dir_imm(self):
        assert decode_opcode(0x75) == "MOV dir,#imm"

    def test_mov_dir_a(self):
        assert decode_opcode(0xF5) == "MOV dir,A"

    def test_mov_dptr(self):
        assert decode_opcode(0x90) == "MOV DPTR,#imm16"

    def test_movc_a_dptr(self):
        assert decode_opcode(0x93) == "MOVC A,@A+DPTR"

    def test_movc_a_pc(self):
        assert decode_opcode(0x83) == "MOVC A,@A+PC"

    def test_movx_a_dptr(self):
        assert decode_opcode(0xE0) == "MOVX A,@DPTR"

    def test_movx_dptr_a(self):
        assert decode_opcode(0xF0) == "MOVX @DPTR,A"


class TestDecodeArithmetic:
    def test_add_a_imm(self):
        assert decode_opcode(0x24) == "ADD A,#imm"

    def test_add_a_dir(self):
        assert decode_opcode(0x25) == "ADD A,dir"

    def test_add_a_rn(self):
        for op in range(0x28, 0x30):
            assert decode_opcode(op) == "ADD A,Rn"

    def test_addc_a_imm(self):
        assert decode_opcode(0x34) == "ADDC A,#imm"

    def test_addc_a_rn(self):
        for op in range(0x38, 0x40):
            assert decode_opcode(op) == "ADDC A,Rn"

    def test_subb_a_imm(self):
        assert decode_opcode(0x94) == "SUBB A,#imm"

    def test_subb_a_rn(self):
        for op in range(0x98, 0xA0):
            assert decode_opcode(op) == "SUBB A,Rn"

    def test_inc_a(self):
        assert decode_opcode(0x04) == "INC A"

    def test_inc_rn(self):
        for op in range(0x08, 0x10):
            assert decode_opcode(op) == "INC Rn"

    def test_dec_a(self):
        assert decode_opcode(0x14) == "DEC A"

    def test_dec_rn(self):
        for op in range(0x18, 0x20):
            assert decode_opcode(op) == "DEC Rn"

    def test_mul_ab(self):
        assert decode_opcode(0xA4) == "MUL AB"

    def test_div_ab(self):
        assert decode_opcode(0x84) == "DIV AB"

    def test_da_a(self):
        assert decode_opcode(0xD4) == "DA A"


class TestDecodeLogical:
    def test_anl_a_imm(self):
        assert decode_opcode(0x54) == "ANL A,#imm"

    def test_anl_a_rn(self):
        for op in range(0x58, 0x60):
            assert decode_opcode(op) == "ANL A,Rn"

    def test_orl_a_imm(self):
        assert decode_opcode(0x44) == "ORL A,#imm"

    def test_orl_a_rn(self):
        for op in range(0x48, 0x50):
            assert decode_opcode(op) == "ORL A,Rn"

    def test_xrl_a_imm(self):
        assert decode_opcode(0x64) == "XRL A,#imm"

    def test_xrl_a_rn(self):
        for op in range(0x68, 0x70):
            assert decode_opcode(op) == "XRL A,Rn"

    def test_clr_a(self):
        assert decode_opcode(0xE4) == "CLR A"

    def test_cpl_a(self):
        assert decode_opcode(0xF4) == "CPL A"

    def test_rl_a(self):
        assert decode_opcode(0x23) == "RL A"

    def test_rlc_a(self):
        assert decode_opcode(0x33) == "RLC A"

    def test_rr_a(self):
        assert decode_opcode(0x03) == "RR A"

    def test_rrc_a(self):
        assert decode_opcode(0x13) == "RRC A"

    def test_swap_a(self):
        assert decode_opcode(0xC4) == "SWAP A"


class TestDecodeBitOps:
    def test_clr_c(self):
        assert decode_opcode(0xC3) == "CLR C"

    def test_setb_c(self):
        assert decode_opcode(0xD3) == "SETB C"

    def test_cpl_c(self):
        assert decode_opcode(0xB3) == "CPL C"

    def test_anl_c_bit(self):
        assert decode_opcode(0x82) == "ANL C,bit"

    def test_orl_c_bit(self):
        assert decode_opcode(0x72) == "ORL C,bit"

    def test_mov_c_bit(self):
        assert decode_opcode(0xA2) == "MOV C,bit"

    def test_mov_bit_c(self):
        assert decode_opcode(0x92) == "MOV bit,C"

    def test_clr_bit(self):
        assert decode_opcode(0xC2) == "CLR bit"

    def test_setb_bit(self):
        assert decode_opcode(0xD2) == "SETB bit"


class TestDecodeBranch:
    def test_ljmp(self):
        assert decode_opcode(0x02) == "LJMP"

    def test_sjmp(self):
        assert decode_opcode(0x80) == "SJMP"

    def test_jmp_indirect(self):
        assert decode_opcode(0x73) == "JMP @A+DPTR"

    def test_jz(self):
        assert decode_opcode(0x60) == "JZ"

    def test_jnz(self):
        assert decode_opcode(0x70) == "JNZ"

    def test_jc(self):
        assert decode_opcode(0x40) == "JC"

    def test_jnc(self):
        assert decode_opcode(0x50) == "JNC"

    def test_jb(self):
        assert decode_opcode(0x20) == "JB"

    def test_jnb(self):
        assert decode_opcode(0x30) == "JNB"

    def test_jbc(self):
        assert decode_opcode(0x10) == "JBC"

    def test_ajmp_pages(self):
        # AJMP for all 8 pages: 0x01, 0x21, 0x41, 0x61, 0x81, 0xA1, 0xC1, 0xE1
        for page in range(8):
            op = (page << 5) | 0x01
            assert decode_opcode(op) == "AJMP"

    def test_acall_pages(self):
        # ACALL for all 8 pages: 0x11, 0x31, 0x51, 0x71, 0x91, 0xB1, 0xD1, 0xF1
        for page in range(8):
            op = (page << 5) | 0x11
            assert decode_opcode(op) == "ACALL"

    def test_djnz_rn(self):
        for op in range(0xD8, 0xE0):
            assert decode_opcode(op) == "DJNZ Rn"

    def test_djnz_dir(self):
        assert decode_opcode(0xD5) == "DJNZ dir"

    def test_cjne_a_imm(self):
        assert decode_opcode(0xB4) == "CJNE A,#imm"

    def test_cjne_a_dir(self):
        assert decode_opcode(0xB5) == "CJNE A,dir"

    def test_cjne_rn(self):
        for op in range(0xB8, 0xC0):
            assert decode_opcode(op) == "CJNE Rn,#imm"


class TestDecodeSubroutine:
    def test_lcall(self):
        assert decode_opcode(0x12) == "LCALL"

    def test_ret(self):
        assert decode_opcode(0x22) == "RET"

    def test_reti(self):
        assert decode_opcode(0x32) == "RETI"


class TestDecodeStack:
    def test_push(self):
        assert decode_opcode(0xC0) == "PUSH"

    def test_pop(self):
        assert decode_opcode(0xD0) == "POP"
