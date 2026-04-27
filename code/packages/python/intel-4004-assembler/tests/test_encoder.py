"""test_encoder.py — Unit tests for the Intel 4004 instruction encoder.

Each test class covers a group of related instructions.  Tests verify:

1. Correct byte output for every instruction class.
2. Boundary values (min/max registers, min/max immediates).
3. Error conditions (unknown mnemonics, out-of-range values).
"""

from __future__ import annotations

import pytest

from intel_4004_assembler.encoder import (
    AssemblerError,
    encode_instruction,
    instruction_size,
)

# Convenience alias — most tests use empty symbol table and PC=0.
def enc(mnemonic: str, *operands: str, symbols: dict | None = None, pc: int = 0) -> bytes:
    """Shorthand for encode_instruction with default symbols/pc."""
    return encode_instruction(mnemonic, tuple(operands), symbols or {}, pc)


class TestNoOperandInstructions:
    """Zero-operand instructions that map to a single fixed byte."""

    def test_nop(self) -> None:
        assert enc("NOP") == bytes([0x00])

    def test_hlt(self) -> None:
        """HLT is a simulator-only extension — maps to 0x01."""
        assert enc("HLT") == bytes([0x01])

    def test_wrm(self) -> None:
        assert enc("WRM") == bytes([0xE0])

    def test_wmp(self) -> None:
        assert enc("WMP") == bytes([0xE1])

    def test_wrr(self) -> None:
        assert enc("WRR") == bytes([0xE2])

    def test_wr0(self) -> None:
        assert enc("WR0") == bytes([0xE4])

    def test_wr1(self) -> None:
        assert enc("WR1") == bytes([0xE5])

    def test_wr2(self) -> None:
        assert enc("WR2") == bytes([0xE6])

    def test_wr3(self) -> None:
        assert enc("WR3") == bytes([0xE7])

    def test_sbm(self) -> None:
        assert enc("SBM") == bytes([0xE8])

    def test_rdm(self) -> None:
        assert enc("RDM") == bytes([0xE9])

    def test_rdr(self) -> None:
        assert enc("RDR") == bytes([0xEA])

    def test_adm(self) -> None:
        assert enc("ADM") == bytes([0xEB])

    def test_rd0(self) -> None:
        assert enc("RD0") == bytes([0xEC])

    def test_rd1(self) -> None:
        assert enc("RD1") == bytes([0xED])

    def test_rd2(self) -> None:
        assert enc("RD2") == bytes([0xEE])

    def test_rd3(self) -> None:
        assert enc("RD3") == bytes([0xEF])

    def test_clb(self) -> None:
        assert enc("CLB") == bytes([0xF0])

    def test_clc(self) -> None:
        assert enc("CLC") == bytes([0xF1])

    def test_iac(self) -> None:
        assert enc("IAC") == bytes([0xF2])

    def test_cmc(self) -> None:
        assert enc("CMC") == bytes([0xF3])

    def test_cma(self) -> None:
        assert enc("CMA") == bytes([0xF4])

    def test_ral(self) -> None:
        assert enc("RAL") == bytes([0xF5])

    def test_rar(self) -> None:
        assert enc("RAR") == bytes([0xF6])

    def test_tcc(self) -> None:
        assert enc("TCC") == bytes([0xF7])

    def test_dac(self) -> None:
        assert enc("DAC") == bytes([0xF8])

    def test_tcs(self) -> None:
        assert enc("TCS") == bytes([0xF9])

    def test_stc(self) -> None:
        assert enc("STC") == bytes([0xFA])

    def test_daa(self) -> None:
        assert enc("DAA") == bytes([0xFB])

    def test_kbp(self) -> None:
        assert enc("KBP") == bytes([0xFC])

    def test_dcl(self) -> None:
        assert enc("DCL") == bytes([0xFD])


class TestLdm:
    """LDM k — load 4-bit immediate into ACC."""

    def test_ldm_zero(self) -> None:
        assert enc("LDM", "0") == bytes([0xD0])

    def test_ldm_five(self) -> None:
        assert enc("LDM", "5") == bytes([0xD5])

    def test_ldm_fifteen(self) -> None:
        assert enc("LDM", "15") == bytes([0xDF])

    def test_ldm_hex(self) -> None:
        assert enc("LDM", "0xF") == bytes([0xDF])

    def test_ldm_out_of_range_high(self) -> None:
        with pytest.raises(AssemblerError, match="out of range"):
            enc("LDM", "16")

    def test_ldm_out_of_range_negative(self) -> None:
        with pytest.raises(AssemblerError):
            enc("LDM", "-1")


class TestBbl:
    """BBL k — branch back and load k (return instruction)."""

    def test_bbl_zero(self) -> None:
        assert enc("BBL", "0") == bytes([0xC0])

    def test_bbl_seven(self) -> None:
        assert enc("BBL", "7") == bytes([0xC7])

    def test_bbl_fifteen(self) -> None:
        assert enc("BBL", "15") == bytes([0xCF])

    def test_bbl_out_of_range(self) -> None:
        with pytest.raises(AssemblerError, match="out of range"):
            enc("BBL", "16")


class TestRegisterInstructions:
    """INC, ADD, SUB, LD, XCH — one register operand."""

    def test_inc_r0(self) -> None:
        assert enc("INC", "R0") == bytes([0x60])

    def test_inc_r15(self) -> None:
        assert enc("INC", "R15") == bytes([0x6F])

    def test_add_r0(self) -> None:
        assert enc("ADD", "R0") == bytes([0x80])

    def test_add_r5(self) -> None:
        assert enc("ADD", "R5") == bytes([0x85])

    def test_add_r15(self) -> None:
        assert enc("ADD", "R15") == bytes([0x8F])

    def test_sub_r2(self) -> None:
        assert enc("SUB", "R2") == bytes([0x92])

    def test_ld_r2(self) -> None:
        assert enc("LD", "R2") == bytes([0xA2])

    def test_ld_r15(self) -> None:
        assert enc("LD", "R15") == bytes([0xAF])

    def test_xch_r2(self) -> None:
        assert enc("XCH", "R2") == bytes([0xB2])

    def test_xch_r0(self) -> None:
        assert enc("XCH", "R0") == bytes([0xB0])

    def test_invalid_register(self) -> None:
        with pytest.raises(AssemblerError, match="Invalid register"):
            enc("INC", "R16")

    def test_invalid_register_name(self) -> None:
        with pytest.raises(AssemblerError, match="Invalid register"):
            enc("LD", "X5")


class TestSrcFinJin:
    """SRC, FIN, JIN — register pair operand."""

    def test_src_p0(self) -> None:
        # P0 → 0x21 (0x20 | (2*0 + 1))
        assert enc("SRC", "P0") == bytes([0x21])

    def test_src_p1(self) -> None:
        # P1 → 0x23 (0x20 | (2*1 + 1))
        assert enc("SRC", "P1") == bytes([0x23])

    def test_src_p7(self) -> None:
        # P7 → 0x2F (0x20 | (2*7 + 1) = 0x2F)
        assert enc("SRC", "P7") == bytes([0x2F])

    def test_fin_p0(self) -> None:
        # P0 → 0x30 (0x30 | (2*0))
        assert enc("FIN", "P0") == bytes([0x30])

    def test_fin_p1(self) -> None:
        # P1 → 0x32 (0x30 | (2*1))
        assert enc("FIN", "P1") == bytes([0x32])

    def test_jin_p0(self) -> None:
        # P0 → 0x31 (0x30 | (2*0 + 1))
        assert enc("JIN", "P0") == bytes([0x31])

    def test_jin_p3(self) -> None:
        # P3 → 0x37 (0x30 | (2*3 + 1))
        assert enc("JIN", "P3") == bytes([0x37])

    def test_invalid_pair(self) -> None:
        with pytest.raises(AssemblerError, match="Invalid register pair"):
            enc("SRC", "P8")


class TestFim:
    """FIM Pp, d8 — two bytes: pair byte + immediate byte."""

    def test_fim_p0_0x42(self) -> None:
        # P0 → byte1 = 0x20 | (2*0) = 0x20; byte2 = 0x42
        assert enc("FIM", "P0", "0x42") == bytes([0x20, 0x42])

    def test_fim_p1_0x00(self) -> None:
        assert enc("FIM", "P1", "0") == bytes([0x22, 0x00])

    def test_fim_p7_0xFF(self) -> None:
        assert enc("FIM", "P7", "255") == bytes([0x2E, 0xFF])

    def test_fim_out_of_range(self) -> None:
        with pytest.raises(AssemblerError, match="out of range"):
            enc("FIM", "P0", "256")


class TestJcn:
    """JCN cond, addr — conditional jump."""

    def test_jcn_zero_condition(self) -> None:
        # cond=0x4 (test zero), addr=0x010 → byte1=0x14, byte2=0x10
        result = enc("JCN", "0x4", "0x10")
        assert result == bytes([0x14, 0x10])

    def test_jcn_nonzero_condition(self) -> None:
        # cond=0xC (test zero, inverted = nonzero), addr=0x42 → 0x1C 0x42
        result = enc("JCN", "0xC", "0x42")
        assert result == bytes([0x1C, 0x42])

    def test_jcn_cond_zero(self) -> None:
        result = enc("JCN", "0", "0")
        assert result == bytes([0x10, 0x00])

    def test_jcn_cond_fifteen(self) -> None:
        result = enc("JCN", "15", "0xFF")
        assert result == bytes([0x1F, 0xFF])

    def test_jcn_with_label(self) -> None:
        result = encode_instruction(
            "JCN", ("0x4", "mytarget"), {"mytarget": 0x020}, pc=0
        )
        assert result == bytes([0x14, 0x20])

    def test_jcn_cond_out_of_range(self) -> None:
        with pytest.raises(AssemblerError, match="out of range"):
            enc("JCN", "16", "0x00")

    def test_jcn_addr_out_of_range(self) -> None:
        with pytest.raises(AssemblerError, match="out of range"):
            enc("JCN", "0", "0x1000")


class TestJun:
    """JUN addr12 — unconditional jump."""

    def test_jun_zero(self) -> None:
        # addr=0x000 → 0x40 0x00
        assert enc("JUN", "0x000") == bytes([0x40, 0x00])

    def test_jun_0x042(self) -> None:
        # addr=0x042 → high_nibble=0, low=0x42 → 0x40 0x42
        assert enc("JUN", "0x042") == bytes([0x40, 0x42])

    def test_jun_0x1AB(self) -> None:
        # addr=0x1AB → high_nibble=1, low=0xAB → 0x41 0xAB
        assert enc("JUN", "0x1AB") == bytes([0x41, 0xAB])

    def test_jun_0xFFF(self) -> None:
        # addr=0xFFF → high_nibble=0xF, low=0xFF → 0x4F 0xFF
        assert enc("JUN", "0xFFF") == bytes([0x4F, 0xFF])

    def test_jun_self_loop(self) -> None:
        # JUN $ at PC=0x005 → addr=0x005 → 0x40 0x05
        result = encode_instruction("JUN", ("$",), {}, pc=0x005)
        assert result == bytes([0x40, 0x05])

    def test_jun_with_label(self) -> None:
        result = encode_instruction("JUN", ("loop",), {"loop": 0x042}, pc=0)
        assert result == bytes([0x40, 0x42])

    def test_jun_undefined_label(self) -> None:
        with pytest.raises(AssemblerError, match="Undefined label"):
            encode_instruction("JUN", ("missing",), {}, pc=0)

    def test_jun_out_of_range(self) -> None:
        with pytest.raises(AssemblerError, match="out of range"):
            enc("JUN", "0x1000")


class TestJms:
    """JMS addr12 — jump to subroutine."""

    def test_jms_zero(self) -> None:
        assert enc("JMS", "0x000") == bytes([0x50, 0x00])

    def test_jms_0x2AB(self) -> None:
        # addr=0x2AB → high_nibble=2, low=0xAB → 0x52 0xAB
        assert enc("JMS", "0x2AB") == bytes([0x52, 0xAB])

    def test_jms_0xFFF(self) -> None:
        assert enc("JMS", "0xFFF") == bytes([0x5F, 0xFF])


class TestIsz:
    """ISZ Rn, addr8 — increment and skip if zero."""

    def test_isz_r0_addr_0(self) -> None:
        # R0, addr=0 → 0x70 0x00
        assert enc("ISZ", "R0", "0") == bytes([0x70, 0x00])

    def test_isz_r5_addr_0x42(self) -> None:
        assert enc("ISZ", "R5", "0x42") == bytes([0x75, 0x42])

    def test_isz_r15_addr_0xFF(self) -> None:
        assert enc("ISZ", "R15", "255") == bytes([0x7F, 0xFF])

    def test_isz_addr_out_of_range(self) -> None:
        with pytest.raises(AssemblerError, match="out of range"):
            enc("ISZ", "R0", "256")


class TestAddImm:
    """ADD_IMM Rd, Rs, k — pseudo-instruction expands to LDM k + ADD Rs."""

    def test_add_imm_r2_r2_1(self) -> None:
        # LDM 1 → 0xD1; ADD R2 → 0x82
        assert enc("ADD_IMM", "R2", "R2", "1") == bytes([0xD1, 0x82])

    def test_add_imm_k_zero(self) -> None:
        # LDM 0 → 0xD0; ADD R0 → 0x80
        assert enc("ADD_IMM", "R0", "R0", "0") == bytes([0xD0, 0x80])

    def test_add_imm_k_fifteen(self) -> None:
        # LDM 15 → 0xDF; ADD R5 → 0x85
        assert enc("ADD_IMM", "R5", "R5", "15") == bytes([0xDF, 0x85])

    def test_add_imm_out_of_range(self) -> None:
        with pytest.raises(AssemblerError, match="out of range"):
            enc("ADD_IMM", "R2", "R2", "16")


class TestOrgDirective:
    """ORG directive emits no bytes."""

    def test_org_emits_nothing(self) -> None:
        assert enc("ORG", "0x000") == b""

    def test_org_emits_nothing_any_addr(self) -> None:
        assert enc("ORG", "0x100") == b""


class TestUnknownMnemonic:
    """Unknown mnemonics raise AssemblerError."""

    def test_unknown_mnemonic(self) -> None:
        with pytest.raises(AssemblerError, match="Unknown mnemonic"):
            enc("FOOBAR")

    def test_unknown_mnemonic_size(self) -> None:
        with pytest.raises(AssemblerError, match="Unknown mnemonic"):
            instruction_size("FOOBAR", ())


class TestInstructionSize:
    """instruction_size returns correct byte counts for all instruction groups."""

    def test_nop_size(self) -> None:
        assert instruction_size("NOP", ()) == 1

    def test_hlt_size(self) -> None:
        assert instruction_size("HLT", ()) == 1

    def test_ldm_size(self) -> None:
        assert instruction_size("LDM", ("5",)) == 1

    def test_bbl_size(self) -> None:
        assert instruction_size("BBL", ("0",)) == 1

    def test_inc_size(self) -> None:
        assert instruction_size("INC", ("R0",)) == 1

    def test_add_size(self) -> None:
        assert instruction_size("ADD", ("R0",)) == 1

    def test_sub_size(self) -> None:
        assert instruction_size("SUB", ("R0",)) == 1

    def test_ld_size(self) -> None:
        assert instruction_size("LD", ("R0",)) == 1

    def test_xch_size(self) -> None:
        assert instruction_size("XCH", ("R0",)) == 1

    def test_src_size(self) -> None:
        assert instruction_size("SRC", ("P0",)) == 1

    def test_fin_size(self) -> None:
        assert instruction_size("FIN", ("P0",)) == 1

    def test_jin_size(self) -> None:
        assert instruction_size("JIN", ("P0",)) == 1

    def test_wrm_size(self) -> None:
        assert instruction_size("WRM", ()) == 1

    def test_rdm_size(self) -> None:
        assert instruction_size("RDM", ()) == 1

    def test_clb_size(self) -> None:
        assert instruction_size("CLB", ()) == 1

    def test_jcn_size(self) -> None:
        assert instruction_size("JCN", ("0x4", "0x10")) == 2

    def test_fim_size(self) -> None:
        assert instruction_size("FIM", ("P0", "0x42")) == 2

    def test_jun_size(self) -> None:
        assert instruction_size("JUN", ("0x042",)) == 2

    def test_jms_size(self) -> None:
        assert instruction_size("JMS", ("0x100",)) == 2

    def test_isz_size(self) -> None:
        assert instruction_size("ISZ", ("R0", "0x10")) == 2

    def test_add_imm_size(self) -> None:
        assert instruction_size("ADD_IMM", ("R2", "R2", "1")) == 2

    def test_org_size(self) -> None:
        assert instruction_size("ORG", ("0x000",)) == 0


class TestLabelResolution:
    """encode_instruction resolves $ and labels correctly."""

    def test_dollar_resolves_to_pc(self) -> None:
        # JUN $ at PC=0x010 → addr=0x010 → 0x40 0x10
        result = encode_instruction("JUN", ("$",), {}, pc=0x010)
        assert result == bytes([0x40, 0x10])

    def test_label_lookup(self) -> None:
        syms = {"target": 0x123}
        result = encode_instruction("JUN", ("target",), syms, pc=0)
        assert result == bytes([0x41, 0x23])

    def test_undefined_label_raises(self) -> None:
        with pytest.raises(AssemblerError, match="Undefined label"):
            encode_instruction("JUN", ("ghost",), {}, pc=0)
