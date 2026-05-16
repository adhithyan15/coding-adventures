"""Tests for decoder.py — instruction decode."""

import pytest

from intel8086_gatelevel.decoder import decode_instruction


def make_mem(*bytes_: int, offset: int = 0) -> bytearray:
    """Create a 64KB memory buffer with bytes at the given offset."""
    mem = bytearray(0x10000)
    for i, b in enumerate(bytes_):
        mem[offset + i] = b
    return mem


class TestDecodeSimple:
    def test_hlt(self):
        mem = make_mem(0xF4)
        d = decode_instruction(mem, 0, 0)
        assert d.mnemonic == "HLT"
        assert d.length == 1

    def test_nop(self):
        mem = make_mem(0x90)
        d = decode_instruction(mem, 0, 0)
        assert d.mnemonic == "NOP"
        assert d.length == 1

    def test_clc(self):
        d = decode_instruction(make_mem(0xF8), 0, 0)
        assert d.mnemonic == "CLC"

    def test_stc(self):
        d = decode_instruction(make_mem(0xF9), 0, 0)
        assert d.mnemonic == "STC"

    def test_cmc(self):
        d = decode_instruction(make_mem(0xF5), 0, 0)
        assert d.mnemonic == "CMC"

    def test_cld(self):
        d = decode_instruction(make_mem(0xFC), 0, 0)
        assert d.mnemonic == "CLD"

    def test_std(self):
        d = decode_instruction(make_mem(0xFD), 0, 0)
        assert d.mnemonic == "STD"

    def test_cli(self):
        d = decode_instruction(make_mem(0xFA), 0, 0)
        assert d.mnemonic == "CLI"

    def test_sti(self):
        d = decode_instruction(make_mem(0xFB), 0, 0)
        assert d.mnemonic == "STI"

    def test_wait(self):
        d = decode_instruction(make_mem(0x9B), 0, 0)
        assert d.mnemonic == "WAIT"

    def test_lahf(self):
        d = decode_instruction(make_mem(0x9F), 0, 0)
        assert d.mnemonic == "LAHF"

    def test_sahf(self):
        d = decode_instruction(make_mem(0x9E), 0, 0)
        assert d.mnemonic == "SAHF"

    def test_cbw(self):
        d = decode_instruction(make_mem(0x98), 0, 0)
        assert d.mnemonic == "CBW"

    def test_cwd(self):
        d = decode_instruction(make_mem(0x99), 0, 0)
        assert d.mnemonic == "CWD"

    def test_xlat(self):
        d = decode_instruction(make_mem(0xD7), 0, 0)
        assert d.mnemonic == "XLAT"

    def test_pushf(self):
        d = decode_instruction(make_mem(0x9C), 0, 0)
        assert d.mnemonic == "PUSHF"

    def test_popf(self):
        d = decode_instruction(make_mem(0x9D), 0, 0)
        assert d.mnemonic == "POPF"


class TestDecodeMov:
    def test_mov_reg16_imm(self):
        # MOV AX, 0x1234  → B8 34 12
        mem = make_mem(0xB8, 0x34, 0x12)
        d = decode_instruction(mem, 0, 0)
        assert d.mnemonic == "MOV"
        assert d.imm == 0x1234
        assert d.word is True
        assert d.length == 3

    def test_mov_reg8_imm(self):
        # MOV AL, 0x42  → B0 42
        mem = make_mem(0xB0, 0x42)
        d = decode_instruction(mem, 0, 0)
        assert d.mnemonic == "MOV"
        assert d.imm == 0x42
        assert d.word is False
        assert d.length == 2

    def test_mov_bx_imm(self):
        # MOV BX, 0xABCD  → BB CD AB
        mem = make_mem(0xBB, 0xCD, 0xAB)
        d = decode_instruction(mem, 0, 0)
        assert d.mnemonic == "MOV"
        assert d.imm == 0xABCD
        assert d.length == 3

    def test_mov_rm_reg_8bit(self):
        # MOV AL, CL → 88 C8 (C8 = mod=11, reg=1 (CL), rm=0 (AL))
        mem = make_mem(0x88, 0xC8)
        d = decode_instruction(mem, 0, 0)
        assert d.mnemonic == "MOV"
        assert d.word is False
        assert d.length == 2

    def test_mov_rm_reg_16bit(self):
        # MOV AX, BX → 89 D8 (D8 = mod=11, reg=3 (BX), rm=0 (AX))
        mem = make_mem(0x89, 0xD8)
        d = decode_instruction(mem, 0, 0)
        assert d.mnemonic == "MOV"
        assert d.word is True
        assert d.length == 2

    def test_mov_rm_imm16(self):
        # MOV [BX], 0x1234 → C7 07 34 12 (mod=00, reg=0, rm=7 BX)
        mem = make_mem(0xC7, 0x07, 0x34, 0x12)
        d = decode_instruction(mem, 0, 0)
        assert d.mnemonic == "MOV"
        assert d.imm == 0x1234
        assert d.length == 4

    def test_mov_ax_mem(self):
        # MOV AX, [0x1234] → A1 34 12
        mem = make_mem(0xA1, 0x34, 0x12)
        d = decode_instruction(mem, 0, 0)
        assert d.mnemonic == "MOV"
        assert d.word is True
        assert d.length == 3


class TestDecodeAlu:
    def test_add_ax_imm(self):
        # ADD AX, 0x0005 → 05 05 00
        mem = make_mem(0x05, 0x05, 0x00)
        d = decode_instruction(mem, 0, 0)
        assert d.mnemonic == "ADD"
        assert d.imm == 5
        assert d.length == 3

    def test_sub_al_imm(self):
        # SUB AL, 3 → 2C 03
        mem = make_mem(0x2C, 0x03)
        d = decode_instruction(mem, 0, 0)
        assert d.mnemonic == "SUB"
        assert d.imm == 3

    def test_cmp_ax_imm(self):
        # CMP AX, 10 → 3D 0A 00
        mem = make_mem(0x3D, 0x0A, 0x00)
        d = decode_instruction(mem, 0, 0)
        assert d.mnemonic == "CMP"
        assert d.imm == 10

    def test_and_rm_imm(self):
        # AND AX, 0x00FF → 81 E0 FF 00
        # (mod=11, reg=4 AND, rm=0 AX)
        mem = make_mem(0x81, 0xE0, 0xFF, 0x00)
        d = decode_instruction(mem, 0, 0)
        assert d.mnemonic == "AND"
        assert d.imm == 0xFF

    def test_or_rm_imm8(self):
        # OR AX, 5 → 83 C8 05 (sign-extended)
        # (mod=11, reg=1 OR, rm=0 AX)
        mem = make_mem(0x83, 0xC8, 0x05)
        d = decode_instruction(mem, 0, 0)
        assert d.mnemonic == "OR"

    def test_xor_rm_reg(self):
        # XOR AX, AX → 31 C0 (mod=11, reg=0 AX, rm=0 AX)
        mem = make_mem(0x31, 0xC0)
        d = decode_instruction(mem, 0, 0)
        assert d.mnemonic == "XOR"


class TestDecodeIncDec:
    def test_inc_ax(self):
        # INC AX → 40
        d = decode_instruction(make_mem(0x40), 0, 0)
        assert d.mnemonic == "INC"
        assert d.reg == 0

    def test_dec_bx(self):
        # DEC BX → 4B (48 + 3)
        d = decode_instruction(make_mem(0x4B), 0, 0)
        assert d.mnemonic == "DEC"
        assert d.reg == 3


class TestDecodePushPop:
    def test_push_ax(self):
        d = decode_instruction(make_mem(0x50), 0, 0)
        assert d.mnemonic == "PUSH"
        assert d.reg == 0

    def test_pop_bx(self):
        d = decode_instruction(make_mem(0x5B), 0, 0)
        assert d.mnemonic == "POP"
        assert d.reg == 3

    def test_push_es(self):
        d = decode_instruction(make_mem(0x06), 0, 0)
        assert d.mnemonic == "PUSH"

    def test_pop_ds(self):
        d = decode_instruction(make_mem(0x1F), 0, 0)
        assert d.mnemonic == "POP"


class TestDecodeJumps:
    def test_jmp_short(self):
        # JMP SHORT +5 → EB 05
        mem = make_mem(0xEB, 0x05)
        d = decode_instruction(mem, 0, 0)
        assert d.mnemonic == "JMP"
        assert d.disp == 5
        assert d.length == 2

    def test_jmp_near(self):
        # JMP NEAR -10 → E9 F6 FF
        mem = make_mem(0xE9, 0xF6, 0xFF)
        d = decode_instruction(mem, 0, 0)
        assert d.mnemonic == "JMP"
        assert d.disp == -10

    def test_jz(self):
        mem = make_mem(0x74, 0x03)
        d = decode_instruction(mem, 0, 0)
        assert d.mnemonic == "JZ"
        assert d.disp == 3

    def test_jnz(self):
        mem = make_mem(0x75, 0xFE)  # JNZ -2
        d = decode_instruction(mem, 0, 0)
        assert d.mnemonic == "JNZ"
        assert d.disp == -2

    def test_loop(self):
        mem = make_mem(0xE2, 0xFE)
        d = decode_instruction(mem, 0, 0)
        assert d.mnemonic == "LOOP"

    def test_jcxz(self):
        mem = make_mem(0xE3, 0x05)
        d = decode_instruction(mem, 0, 0)
        assert d.mnemonic == "JCXZ"


class TestDecodeCallRet:
    def test_call_near(self):
        mem = make_mem(0xE8, 0x00, 0x01)
        d = decode_instruction(mem, 0, 0)
        assert d.mnemonic == "CALL"
        assert d.disp == 0x100

    def test_ret(self):
        d = decode_instruction(make_mem(0xC3), 0, 0)
        assert d.mnemonic == "RET"

    def test_retf(self):
        d = decode_instruction(make_mem(0xCB), 0, 0)
        assert d.mnemonic == "RETF"

    def test_iret(self):
        d = decode_instruction(make_mem(0xCF), 0, 0)
        assert d.mnemonic == "IRET"


class TestDecodeString:
    def test_movsb(self):
        d = decode_instruction(make_mem(0xA4), 0, 0)
        assert d.mnemonic == "MOVS"
        assert d.word is False

    def test_movsw(self):
        d = decode_instruction(make_mem(0xA5), 0, 0)
        assert d.mnemonic == "MOVS"
        assert d.word is True

    def test_cmpsb(self):
        d = decode_instruction(make_mem(0xA6), 0, 0)
        assert d.mnemonic == "CMPS"

    def test_scasw(self):
        d = decode_instruction(make_mem(0xAF), 0, 0)
        assert d.mnemonic == "SCAS"
        assert d.word is True

    def test_lodsb(self):
        d = decode_instruction(make_mem(0xAC), 0, 0)
        assert d.mnemonic == "LODS"

    def test_stosb(self):
        d = decode_instruction(make_mem(0xAA), 0, 0)
        assert d.mnemonic == "STOS"


class TestDecodeShift:
    def test_shl_rm16_1(self):
        # SHL AX, 1 → D1 E0 (mod=11, ext=4 SHL, rm=0 AX)
        mem = make_mem(0xD1, 0xE0)
        d = decode_instruction(mem, 0, 0)
        assert d.mnemonic == "SHL"
        assert d.word is True

    def test_sar_rm8_cl(self):
        # SAR AL, CL → D2 F8 (mod=11, ext=7 SAR, rm=0 AL)
        mem = make_mem(0xD2, 0xF8)
        d = decode_instruction(mem, 0, 0)
        assert d.mnemonic == "SAR"
        assert d.extra.get("use_cl") is True


class TestDecodeBcd:
    def test_daa(self):
        d = decode_instruction(make_mem(0x27), 0, 0)
        assert d.mnemonic == "DAA"

    def test_das(self):
        d = decode_instruction(make_mem(0x2F), 0, 0)
        assert d.mnemonic == "DAS"

    def test_aaa(self):
        d = decode_instruction(make_mem(0x37), 0, 0)
        assert d.mnemonic == "AAA"

    def test_aas(self):
        d = decode_instruction(make_mem(0x3F), 0, 0)
        assert d.mnemonic == "AAS"

    def test_aam(self):
        mem = make_mem(0xD4, 0x0A)
        d = decode_instruction(mem, 0, 0)
        assert d.mnemonic == "AAM"

    def test_aad(self):
        mem = make_mem(0xD5, 0x0A)
        d = decode_instruction(mem, 0, 0)
        assert d.mnemonic == "AAD"


class TestDecodeInOut:
    def test_in_al_imm(self):
        mem = make_mem(0xE4, 0x20)
        d = decode_instruction(mem, 0, 0)
        assert d.mnemonic == "IN"
        assert d.imm == 0x20

    def test_out_imm_al(self):
        mem = make_mem(0xE6, 0x20)
        d = decode_instruction(mem, 0, 0)
        assert d.mnemonic == "OUT"

    def test_in_al_dx(self):
        d = decode_instruction(make_mem(0xEC), 0, 0)
        assert d.mnemonic == "IN"

    def test_out_dx_al(self):
        d = decode_instruction(make_mem(0xEE), 0, 0)
        assert d.mnemonic == "OUT"


class TestDecodeWithPrefix:
    def test_rep_movsb(self):
        mem = make_mem(0xF3, 0xA4)
        d = decode_instruction(mem, 0, 0)
        assert d.mnemonic == "MOVS"
        assert d.rep_prefix == 0xF3
        assert d.length == 2

    def test_repne_scasb(self):
        mem = make_mem(0xF2, 0xAE)
        d = decode_instruction(mem, 0, 0)
        assert d.mnemonic == "SCAS"
        assert d.rep_prefix == 0xF2

    def test_es_override(self):
        mem = make_mem(0x26, 0xA5)
        d = decode_instruction(mem, 0, 0)
        assert d.seg_override is not None
        assert d.mnemonic == "MOVS"

    def test_unknown_opcode(self):
        d = decode_instruction(make_mem(0x0F), 0, 0)
        assert "0x0f" in d.mnemonic.lower() or "DB" in d.mnemonic
