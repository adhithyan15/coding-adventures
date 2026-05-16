"""Tests for mos6502_gatelevel.decoder — instruction decoder."""

from __future__ import annotations

import pytest

from mos6502_gatelevel.decoder import (
    ABS, ABX, ABY, ACC, IMP, IMM, IND, INX, INY, REL, ZP, ZPX, ZPY,
    Decoder6502, DecodedInstruction,
)


@pytest.fixture()
def dec():
    return Decoder6502()


# ── DecodedInstruction ────────────────────────────────────────────────────────

class TestDecodedInstruction:
    def test_fields(self):
        d = DecodedInstruction(opcode=0xA9, mnemonic="LDA", mode=IMM, mode_name="IMM")
        assert d.opcode == 0xA9
        assert d.mnemonic == "LDA"
        assert d.mode == IMM
        assert d.mode_name == "IMM"

    def test_frozen(self):
        d = DecodedInstruction(opcode=0xA9, mnemonic="LDA", mode=IMM, mode_name="IMM")
        with pytest.raises((AttributeError, TypeError)):
            d.opcode = 0x00  # type: ignore[misc]


# ── Decoder6502.decode ────────────────────────────────────────────────────────

class TestDecode:
    def test_lda_imm(self, dec):
        d = dec.decode(0xA9)
        assert d.mnemonic == "LDA"
        assert d.mode == IMM
        assert d.mode_name == "IMM"

    def test_lda_zp(self, dec):
        d = dec.decode(0xA5)
        assert d.mnemonic == "LDA"
        assert d.mode == ZP

    def test_lda_zpx(self, dec):
        d = dec.decode(0xB5)
        assert d.mnemonic == "LDA"
        assert d.mode == ZPX

    def test_lda_abs(self, dec):
        d = dec.decode(0xAD)
        assert d.mnemonic == "LDA"
        assert d.mode == ABS

    def test_lda_abx(self, dec):
        d = dec.decode(0xBD)
        assert d.mnemonic == "LDA"
        assert d.mode == ABX

    def test_lda_aby(self, dec):
        d = dec.decode(0xB9)
        assert d.mnemonic == "LDA"
        assert d.mode == ABY

    def test_lda_inx(self, dec):
        d = dec.decode(0xA1)
        assert d.mnemonic == "LDA"
        assert d.mode == INX

    def test_lda_iny(self, dec):
        d = dec.decode(0xB1)
        assert d.mnemonic == "LDA"
        assert d.mode == INY

    def test_ldx_imm(self, dec):
        assert dec.decode(0xA2).mnemonic == "LDX"
        assert dec.decode(0xA2).mode == IMM

    def test_ldx_zpy(self, dec):
        assert dec.decode(0xB6).mode == ZPY

    def test_ldy_imm(self, dec):
        assert dec.decode(0xA0).mnemonic == "LDY"

    def test_sta_modes(self, dec):
        assert dec.decode(0x85).mnemonic == "STA"
        assert dec.decode(0x85).mode == ZP
        assert dec.decode(0x9D).mode == ABX

    def test_stx_modes(self, dec):
        assert dec.decode(0x86).mnemonic == "STX"
        assert dec.decode(0x96).mode == ZPY

    def test_sty_modes(self, dec):
        assert dec.decode(0x84).mnemonic == "STY"
        assert dec.decode(0x8C).mode == ABS

    def test_transfers(self, dec):
        for opcode, mn in [
            (0xAA, "TAX"), (0xA8, "TAY"), (0x8A, "TXA"), (0x98, "TYA"),
            (0xBA, "TSX"), (0x9A, "TXS"),
        ]:
            d = dec.decode(opcode)
            assert d.mnemonic == mn
            assert d.mode == IMP

    def test_stack_ops(self, dec):
        for opcode, mn in [(0x48, "PHA"), (0x68, "PLA"), (0x08, "PHP"), (0x28, "PLP")]:
            assert dec.decode(opcode).mnemonic == mn
            assert dec.decode(opcode).mode == IMP

    def test_adc_modes(self, dec):
        assert dec.decode(0x69).mnemonic == "ADC"
        assert dec.decode(0x69).mode == IMM
        assert dec.decode(0x61).mode == INX
        assert dec.decode(0x71).mode == INY

    def test_sbc_modes(self, dec):
        assert dec.decode(0xE9).mnemonic == "SBC"
        assert dec.decode(0xE9).mode == IMM
        assert dec.decode(0xFD).mode == ABX

    def test_and_modes(self, dec):
        for opcode in [0x29, 0x25, 0x35, 0x2D, 0x3D, 0x39, 0x21, 0x31]:
            assert dec.decode(opcode).mnemonic == "AND"

    def test_ora_modes(self, dec):
        for opcode in [0x09, 0x05, 0x15, 0x0D, 0x1D, 0x19, 0x01, 0x11]:
            assert dec.decode(opcode).mnemonic == "ORA"

    def test_eor_modes(self, dec):
        for opcode in [0x49, 0x45, 0x55, 0x4D, 0x5D, 0x59, 0x41, 0x51]:
            assert dec.decode(opcode).mnemonic == "EOR"

    def test_bit_modes(self, dec):
        assert dec.decode(0x24).mnemonic == "BIT"
        assert dec.decode(0x24).mode == ZP
        assert dec.decode(0x2C).mode == ABS

    def test_shift_modes(self, dec):
        assert dec.decode(0x0A).mnemonic == "ASL"
        assert dec.decode(0x0A).mode == ACC
        assert dec.decode(0x06).mode == ZP
        assert dec.decode(0x4A).mnemonic == "LSR"
        assert dec.decode(0x4A).mode == ACC
        assert dec.decode(0x2A).mnemonic == "ROL"
        assert dec.decode(0x6A).mnemonic == "ROR"

    def test_inc_dec_memory(self, dec):
        for opcode in [0xE6, 0xF6, 0xEE, 0xFE]:
            assert dec.decode(opcode).mnemonic == "INC"
        for opcode in [0xC6, 0xD6, 0xCE, 0xDE]:
            assert dec.decode(opcode).mnemonic == "DEC"

    def test_inx_iny_dex_dey(self, dec):
        assert dec.decode(0xE8).mnemonic == "INX"
        assert dec.decode(0xC8).mnemonic == "INY"
        assert dec.decode(0xCA).mnemonic == "DEX"
        assert dec.decode(0x88).mnemonic == "DEY"

    def test_compare_modes(self, dec):
        assert dec.decode(0xC9).mnemonic == "CMP"
        assert dec.decode(0xE0).mnemonic == "CPX"
        assert dec.decode(0xC0).mnemonic == "CPY"

    def test_branches(self, dec):
        branches = {
            0x90: "BCC", 0xB0: "BCS", 0xF0: "BEQ", 0xD0: "BNE",
            0x10: "BPL", 0x30: "BMI", 0x50: "BVC", 0x70: "BVS",
        }
        for opcode, mn in branches.items():
            d = dec.decode(opcode)
            assert d.mnemonic == mn
            assert d.mode == REL

    def test_jmp_abs(self, dec):
        d = dec.decode(0x4C)
        assert d.mnemonic == "JMP"
        assert d.mode == ABS

    def test_jmp_ind(self, dec):
        d = dec.decode(0x6C)
        assert d.mnemonic == "JMP"
        assert d.mode == IND

    def test_jsr(self, dec):
        d = dec.decode(0x20)
        assert d.mnemonic == "JSR"
        assert d.mode == ABS

    def test_rts(self, dec):
        d = dec.decode(0x60)
        assert d.mnemonic == "RTS"
        assert d.mode == IMP

    def test_rti(self, dec):
        d = dec.decode(0x40)
        assert d.mnemonic == "RTI"
        assert d.mode == IMP

    def test_flag_instructions(self, dec):
        flags = {
            0x18: "CLC", 0x38: "SEC",
            0xD8: "CLD", 0xF8: "SED",
            0x58: "CLI", 0x78: "SEI",
            0xB8: "CLV",
        }
        for opcode, mn in flags.items():
            d = dec.decode(opcode)
            assert d.mnemonic == mn
            assert d.mode == IMP

    def test_brk(self, dec):
        d = dec.decode(0x00)
        assert d.mnemonic == "BRK"
        assert d.mode == IMP

    def test_nop(self, dec):
        d = dec.decode(0xEA)
        assert d.mnemonic == "NOP"
        assert d.mode == IMP

    def test_illegal_opcode_raises(self, dec):
        # Some illegal opcodes: 0x02, 0x12, 0x80, 0x89
        for illegal in [0x02, 0x12, 0x80, 0x89, 0xFF]:
            with pytest.raises(ValueError, match="Illegal"):
                dec.decode(illegal)

    def test_all_151_legal_opcodes_decode(self, dec):
        legal_opcodes = [
            0x00, 0xEA,
            0xA9, 0xA5, 0xB5, 0xAD, 0xBD, 0xB9, 0xA1, 0xB1,
            0xA2, 0xA6, 0xB6, 0xAE, 0xBE,
            0xA0, 0xA4, 0xB4, 0xAC, 0xBC,
            0x85, 0x95, 0x8D, 0x9D, 0x99, 0x81, 0x91,
            0x86, 0x96, 0x8E,
            0x84, 0x94, 0x8C,
            0xAA, 0xA8, 0x8A, 0x98, 0xBA, 0x9A,
            0x48, 0x68, 0x08, 0x28,
            0x69, 0x65, 0x75, 0x6D, 0x7D, 0x79, 0x61, 0x71,
            0xE9, 0xE5, 0xF5, 0xED, 0xFD, 0xF9, 0xE1, 0xF1,
            0x29, 0x25, 0x35, 0x2D, 0x3D, 0x39, 0x21, 0x31,
            0x09, 0x05, 0x15, 0x0D, 0x1D, 0x19, 0x01, 0x11,
            0x49, 0x45, 0x55, 0x4D, 0x5D, 0x59, 0x41, 0x51,
            0x24, 0x2C,
            0xE6, 0xF6, 0xEE, 0xFE,
            0xE8, 0xC8, 0xCA, 0x88,
            0xC6, 0xD6, 0xCE, 0xDE,
            0x0A, 0x06, 0x16, 0x0E, 0x1E,
            0x4A, 0x46, 0x56, 0x4E, 0x5E,
            0x2A, 0x26, 0x36, 0x2E, 0x3E,
            0x6A, 0x66, 0x76, 0x6E, 0x7E,
            0xC9, 0xC5, 0xD5, 0xCD, 0xDD, 0xD9, 0xC1, 0xD1,
            0xE0, 0xE4, 0xEC,
            0xC0, 0xC4, 0xCC,
            0x90, 0xB0, 0xF0, 0xD0, 0x10, 0x30, 0x50, 0x70,
            0x4C, 0x6C, 0x20, 0x60, 0x40,
            0x18, 0x38, 0xD8, 0xF8, 0x58, 0x78, 0xB8,
        ]
        for opcode in legal_opcodes:
            d = dec.decode(opcode)
            assert isinstance(d, DecodedInstruction)
            assert len(d.mnemonic) >= 2


# ── Decoder6502.is_branch ─────────────────────────────────────────────────────

class TestIsBranch:
    def test_all_branches(self, dec):
        for opcode in [0x90, 0xB0, 0xF0, 0xD0, 0x10, 0x30, 0x50, 0x70]:
            assert dec.is_branch(opcode), f"{opcode:#04x} should be branch"

    def test_non_branches(self, dec):
        for opcode in [0xA9, 0x69, 0x4C, 0x00, 0xAA, 0xE8]:
            assert not dec.is_branch(opcode), f"{opcode:#04x} should not be branch"


# ── Decoder6502.is_legal ──────────────────────────────────────────────────────

class TestIsLegal:
    def test_legal_opcodes(self, dec):
        for opcode in [0x00, 0xA9, 0x69, 0xEA, 0x4C, 0x20]:
            assert dec.is_legal(opcode)

    def test_illegal_opcodes(self, dec):
        for opcode in [0x02, 0x12, 0x22, 0x80, 0xFF]:
            assert not dec.is_legal(opcode)
