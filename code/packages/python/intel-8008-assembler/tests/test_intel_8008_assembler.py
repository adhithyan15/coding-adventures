"""test_intel_8008_assembler.py -- Tests for the Intel 8008 assembler.

Tests cover:
- Lexer: tokenising labels, mnemonics, operands, hi()/lo() expressions
- Encoder: encoding every instruction type (MOV, MVI, ALU reg, ALU imm,
           jump/call, IN, OUT, fixed opcodes, INR/DCR, RST)
- Assembler: two-pass label resolution (forward and backward references),
             ORG directive, hi()/lo() directives, error cases
- Integration: complete programs matching expected byte sequences
"""

from __future__ import annotations

import pytest

from intel_8008_assembler import AssemblerError, Intel8008Assembler, assemble
from intel_8008_assembler.encoder import (
    encode_instruction,
    instruction_size,
)
from intel_8008_assembler.lexer import lex_line, lex_program

# ===========================================================================
# Lexer tests
# ===========================================================================

class TestLexLine:
    """Unit tests for the lex_line tokeniser."""

    def test_blank_line(self) -> None:
        """Blank line → label=None, mnemonic=None, operands=()."""
        p = lex_line("")
        assert p.label is None
        assert p.mnemonic is None
        assert p.operands == ()

    def test_comment_only(self) -> None:
        """; comment → all None/empty."""
        p = lex_line("; this is a comment")
        assert p.label is None
        assert p.mnemonic is None

    def test_simple_mnemonic_no_operands(self) -> None:
        """HLT → mnemonic='HLT', no operands."""
        p = lex_line("    HLT")
        assert p.mnemonic == "HLT"
        assert p.operands == ()
        assert p.label is None

    def test_mnemonic_uppercased(self) -> None:
        """Mnemonics are uppercased by the lexer."""
        p = lex_line("    rfc")
        assert p.mnemonic == "RFC"

    def test_single_operand(self) -> None:
        """JMP label → operands=('label',)."""
        p = lex_line("    JMP  _start")
        assert p.mnemonic == "JMP"
        assert p.operands == ("_start",)

    def test_two_operands(self) -> None:
        """MOV A, B → operands=('A', 'B')."""
        p = lex_line("    MOV  A, B")
        assert p.mnemonic == "MOV"
        assert p.operands == ("A", "B")

    def test_mvi_immediate(self) -> None:
        """MVI B, 42 → operands=('B', '42')."""
        p = lex_line("    MVI  B, 42")
        assert p.mnemonic == "MVI"
        assert p.operands == ("B", "42")

    def test_mvi_hex_immediate(self) -> None:
        """MVI H, 0xFF → operands=('H', '0xFF')."""
        p = lex_line("    MVI  H, 0xFF")
        assert p.mnemonic == "MVI"
        assert p.operands == ("H", "0xFF")

    def test_hi_expression_preserved(self) -> None:
        """hi(counter) is preserved as a verbatim operand string."""
        p = lex_line("    MVI  H, hi(counter)")
        assert p.mnemonic == "MVI"
        assert p.operands == ("H", "hi(counter)")

    def test_lo_expression_preserved(self) -> None:
        """lo(counter) is preserved as a verbatim operand string."""
        p = lex_line("    MVI  L, lo(counter)")
        assert p.mnemonic == "MVI"
        assert p.operands == ("L", "lo(counter)")

    def test_label_alone(self) -> None:
        """Label-only line → label set, mnemonic=None."""
        p = lex_line("loop_0_start:")
        assert p.label == "loop_0_start"
        assert p.mnemonic is None

    def test_label_with_instruction(self) -> None:
        """Label + instruction on same line."""
        p = lex_line("_start:  MVI  B, 0")
        assert p.label == "_start"
        assert p.mnemonic == "MVI"
        assert p.operands == ("B", "0")

    def test_comment_stripped(self) -> None:
        """Inline comment is stripped; operands stop before ;."""
        p = lex_line("    MVI  B, 42  ; load 42 into B")
        assert p.mnemonic == "MVI"
        assert p.operands == ("B", "42")

    def test_dollar_sign_operand(self) -> None:
        """$ in operand is preserved for PC reference."""
        p = lex_line("    JMP  $")
        assert p.mnemonic == "JMP"
        assert p.operands == ("$",)

    def test_org_directive(self) -> None:
        """ORG 0x0000 → mnemonic='ORG', operands=('0x0000',)."""
        p = lex_line("    ORG  0x0000")
        assert p.mnemonic == "ORG"
        assert p.operands == ("0x0000",)


class TestLexProgram:
    """Tests for multi-line lexing."""

    def test_multiline_program(self) -> None:
        """Full program is lexed into one ParsedLine per source line."""
        src = "    ORG 0x0000\n_start:\n    HLT"
        lines = lex_program(src)
        assert len(lines) == 3
        assert lines[0].mnemonic == "ORG"
        assert lines[1].label == "_start"
        assert lines[2].mnemonic == "HLT"

    def test_empty_string(self) -> None:
        """Empty string → no lines (splitlines on '' yields empty list)."""
        lines = lex_program("")
        assert lines == []


# ===========================================================================
# Encoder tests -- instruction_size
# ===========================================================================

class TestInstructionSize:
    """Tests for instruction_size (Pass 1 helper)."""

    def test_fixed_opcodes_are_1_byte(self) -> None:
        """All fixed-opcode instructions are 1 byte."""
        for mnemonic in ("HLT", "RFC", "RET", "RLC", "RRC", "RAL", "RAR",
                         "RFZ", "RFS", "RFP", "RTC", "RTZ", "RTS", "RTP"):
            assert instruction_size(mnemonic, ()) == 1, f"Expected 1 for {mnemonic}"

    def test_alu_reg_ops_are_1_byte(self) -> None:
        """ALU register operations are 1 byte."""
        for mnemonic in ("ADD", "ADC", "SUB", "SBB", "ANA", "XRA", "ORA", "CMP"):
            assert instruction_size(mnemonic, ("B",)) == 1

    def test_mov_is_1_byte(self) -> None:
        assert instruction_size("MOV", ("A", "B")) == 1

    def test_inr_dcr_are_1_byte(self) -> None:
        assert instruction_size("INR", ("B",)) == 1
        assert instruction_size("DCR", ("C",)) == 1

    def test_in_out_are_1_byte(self) -> None:
        assert instruction_size("IN", ("0",)) == 1
        assert instruction_size("OUT", ("1",)) == 1

    def test_mvi_is_2_bytes(self) -> None:
        assert instruction_size("MVI", ("B", "42")) == 2

    def test_alu_imm_are_2_bytes(self) -> None:
        for mnemonic in ("ADI", "ACI", "SUI", "SBI", "ANI", "XRI", "ORI", "CPI"):
            assert instruction_size(mnemonic, ("5",)) == 2

    def test_jmp_cal_are_3_bytes(self) -> None:
        for mnemonic in ("JMP", "CAL"):
            assert instruction_size(mnemonic, ("label",)) == 3

    def test_conditional_jumps_are_3_bytes(self) -> None:
        for mnemonic in ("JFC", "JTC", "JFZ", "JTZ", "JFS", "JTS", "JFP", "JTP"):
            assert instruction_size(mnemonic, ("label",)) == 3

    def test_org_is_0_bytes(self) -> None:
        assert instruction_size("ORG", ("0x0000",)) == 0

    def test_unknown_mnemonic_raises(self) -> None:
        with pytest.raises(AssemblerError, match="Unknown mnemonic"):
            instruction_size("FOOBAR", ())


# ===========================================================================
# Encoder tests -- encode_instruction
# ===========================================================================

class TestEncodeFixedOpcodes:
    """Tests for fixed one-byte instructions."""

    def test_hlt(self) -> None:
        """HLT encodes as 0xFF."""
        assert encode_instruction("HLT", (), {}, 0) == bytes([0xFF])

    def test_rfc(self) -> None:
        """RFC encodes as 0x03 (00_000_011: CCC=0=CY, T=0=false → carry-false return)."""
        assert encode_instruction("RFC", (), {}, 0) == bytes([0x03])

    def test_ret_is_rfc(self) -> None:
        """RET is a synonym for RFC — same encoding (0x03)."""
        assert encode_instruction("RET", (), {}, 0) == bytes([0x03])

    def test_rlc(self) -> None:
        """RLC → 0x02."""
        assert encode_instruction("RLC", (), {}, 0) == bytes([0x02])

    def test_rrc(self) -> None:
        """RRC → 0x0A."""
        assert encode_instruction("RRC", (), {}, 0) == bytes([0x0A])

    def test_ral(self) -> None:
        """RAL → 0x12."""
        assert encode_instruction("RAL", (), {}, 0) == bytes([0x12])

    def test_rar(self) -> None:
        """RAR → 0x1A."""
        assert encode_instruction("RAR", (), {}, 0) == bytes([0x1A])

    def test_rfz(self) -> None:
        """RFZ → 0x0B."""
        assert encode_instruction("RFZ", (), {}, 0) == bytes([0x0B])

    def test_rfs(self) -> None:
        """RFS → 0x13."""
        assert encode_instruction("RFS", (), {}, 0) == bytes([0x13])

    def test_rfp(self) -> None:
        """RFP → 0x1B."""
        assert encode_instruction("RFP", (), {}, 0) == bytes([0x1B])

    def test_rtc(self) -> None:
        """RTC → 0x07 (00_000_111: CCC=0=CY, T=1=true → carry-true return)."""
        assert encode_instruction("RTC", (), {}, 0) == bytes([0x07])

    def test_rtz(self) -> None:
        """RTZ → 0x0F (00_001_111: CCC=1=Z, T=1=true → zero-true return)."""
        assert encode_instruction("RTZ", (), {}, 0) == bytes([0x0F])

    def test_rts(self) -> None:
        """RTS → 0x17 (00_010_111: CCC=2=S, T=1=true → sign-true return)."""
        assert encode_instruction("RTS", (), {}, 0) == bytes([0x17])

    def test_rtp(self) -> None:
        """RTP → 0x1F (00_011_111: CCC=3=P, T=1=true → parity-true return)."""
        assert encode_instruction("RTP", (), {}, 0) == bytes([0x1F])

    def test_wrong_operand_count_raises(self) -> None:
        """Fixed opcodes with operands raise AssemblerError."""
        with pytest.raises(AssemblerError, match="expects 0 operand"):
            encode_instruction("HLT", ("extra",), {}, 0)


class TestEncodeMov:
    """Tests for MOV dst, src (Group 01: 0x40 | dst<<3 | src)."""

    def test_mov_a_b(self) -> None:
        """MOV A, B: dst=A(7), src=B(0) → 0x40 | (7<<3) | 0 = 0x78."""
        assert encode_instruction("MOV", ("A", "B"), {}, 0) == bytes([0x78])

    def test_mov_a_c(self) -> None:
        """MOV A, C: dst=A(7), src=C(1) → 0x40 | (7<<3) | 1 = 0x79."""
        assert encode_instruction("MOV", ("A", "C"), {}, 0) == bytes([0x79])

    def test_mov_m_a(self) -> None:
        """MOV M, A: dst=M(6), src=A(7) → 0x40 | (6<<3) | 7 = 0x77."""
        assert encode_instruction("MOV", ("M", "A"), {}, 0) == bytes([0x77])

    def test_mov_a_m(self) -> None:
        """MOV A, M: dst=A(7), src=M(6) → 0x40 | (7<<3) | 6 = 0x7E."""
        assert encode_instruction("MOV", ("A", "M"), {}, 0) == bytes([0x7E])

    def test_mov_b_c(self) -> None:
        """MOV B, C: dst=B(0), src=C(1) → 0x40 | (0<<3) | 1 = 0x41."""
        assert encode_instruction("MOV", ("B", "C"), {}, 0) == bytes([0x41])

    def test_mov_c_a(self) -> None:
        """MOV C, A: dst=C(1), src=A(7) → 0x40 | (1<<3) | 7 = 0x4F."""
        assert encode_instruction("MOV", ("C", "A"), {}, 0) == bytes([0x4F])

    def test_mov_d_e(self) -> None:
        """MOV D, E: dst=D(2), src=E(3) → 0x40 | (2<<3) | 3 = 0x53."""
        assert encode_instruction("MOV", ("D", "E"), {}, 0) == bytes([0x53])

    def test_mov_h_l(self) -> None:
        """MOV H, L: dst=H(4), src=L(5) → 0x40 | (4<<3) | 5 = 0x65."""
        assert encode_instruction("MOV", ("H", "L"), {}, 0) == bytes([0x65])

    def test_mov_invalid_register_raises(self) -> None:
        """Invalid register name raises AssemblerError."""
        with pytest.raises(AssemblerError, match="Invalid 8008 register"):
            encode_instruction("MOV", ("X", "B"), {}, 0)

    def test_mov_wrong_operand_count_raises(self) -> None:
        """MOV with 1 operand raises AssemblerError."""
        with pytest.raises(AssemblerError, match="expects 2 operand"):
            encode_instruction("MOV", ("A",), {}, 0)


class TestEncodeMvi:
    """Tests for MVI r, d8 (Group 00: (r<<3) | 0x06, d8)."""

    def test_mvi_b_zero(self) -> None:
        """MVI B, 0: opcode = (0<<3)|0x06 = 0x06; data = 0x00 → [0x06, 0x00]."""
        assert encode_instruction("MVI", ("B", "0"), {}, 0) == bytes([0x06, 0x00])

    def test_mvi_b_42(self) -> None:
        """MVI B, 42: opcode = 0x06; data = 42 = 0x2A → [0x06, 0x2A]."""
        assert encode_instruction("MVI", ("B", "42"), {}, 0) == bytes([0x06, 0x2A])

    def test_mvi_c_imm(self) -> None:
        """MVI C, 255: opcode = (1<<3)|0x06 = 0x0E; data = 0xFF → [0x0E, 0xFF]."""
        assert encode_instruction("MVI", ("C", "255"), {}, 0) == bytes([0x0E, 0xFF])

    def test_mvi_d_hex(self) -> None:
        """MVI D, 0x10: opcode = (2<<3)|0x06 = 0x16; data = 0x10 → [0x16, 0x10]."""
        assert encode_instruction("MVI", ("D", "0x10"), {}, 0) == bytes([0x16, 0x10])

    def test_mvi_e_imm(self) -> None:
        """MVI E, 1: opcode = (3<<3)|0x06 = 0x1E; data = 1 → [0x1E, 0x01]."""
        assert encode_instruction("MVI", ("E", "1"), {}, 0) == bytes([0x1E, 0x01])

    def test_mvi_h_imm(self) -> None:
        """MVI H, 0x20: opcode = (4<<3)|0x06 = 0x26; data = 0x20 → [0x26, 0x20]."""
        assert encode_instruction("MVI", ("H", "0x20"), {}, 0) == bytes([0x26, 0x20])

    def test_mvi_l_imm(self) -> None:
        """MVI L, 0x00: opcode = (5<<3)|0x06 = 0x2E; data = 0 → [0x2E, 0x00]."""
        assert encode_instruction("MVI", ("L", "0x00"), {}, 0) == bytes([0x2E, 0x00])

    def test_mvi_a_imm(self) -> None:
        """MVI A, 0: opcode = (7<<3)|0x06 = 0x3E; data = 0 → [0x3E, 0x00]."""
        assert encode_instruction("MVI", ("A", "0"), {}, 0) == bytes([0x3E, 0x00])

    def test_mvi_hi_expression(self) -> None:
        """MVI H, hi(sym): hi(0x2010) = (0x2010 >> 8) & 0x3F = 0x20."""
        syms = {"sym": 0x2010}
        encoded = encode_instruction("MVI", ("H", "hi(sym)"), syms, 0)
        assert encoded == bytes([0x26, 0x20])

    def test_mvi_lo_expression(self) -> None:
        """MVI L, lo(sym): lo(0x2010) = 0x2010 & 0xFF = 0x10."""
        syms = {"sym": 0x2010}
        encoded = encode_instruction("MVI", ("L", "lo(sym)"), syms, 0)
        assert encoded == bytes([0x2E, 0x10])

    def test_mvi_hi_lo_zero_addr(self) -> None:
        """hi(0) = 0, lo(0) = 0."""
        syms = {"x": 0}
        assert encode_instruction("MVI", ("H", "hi(x)"), syms, 0) == bytes([0x26, 0x00])
        assert encode_instruction("MVI", ("L", "lo(x)"), syms, 0) == bytes([0x2E, 0x00])

    def test_mvi_out_of_range_raises(self) -> None:
        """MVI with immediate > 255 raises AssemblerError."""
        with pytest.raises(AssemblerError, match="out of range"):
            encode_instruction("MVI", ("B", "256"), {}, 0)

    def test_mvi_undefined_hi_sym_raises(self) -> None:
        """hi(undefined_sym) raises AssemblerError."""
        with pytest.raises(AssemblerError, match="Undefined label"):
            encode_instruction("MVI", ("H", "hi(no_such_label)"), {}, 0)


class TestEncodeAluReg:
    """Tests for ALU register operations (Group 10).

    All ALU register ops have the form: base_opcode | reg_code
    Register codes: B=0, C=1, D=2, E=3, H=4, L=5, M=6, A=7
    """

    def test_add_b(self) -> None:
        """ADD B: 0x80 | 0 = 0x80."""
        assert encode_instruction("ADD", ("B",), {}, 0) == bytes([0x80])

    def test_add_c(self) -> None:
        """ADD C: 0x80 | 1 = 0x81."""
        assert encode_instruction("ADD", ("C",), {}, 0) == bytes([0x81])

    def test_add_m(self) -> None:
        """ADD M: 0x80 | 6 = 0x86."""
        assert encode_instruction("ADD", ("M",), {}, 0) == bytes([0x86])

    def test_add_a(self) -> None:
        """ADD A: 0x80 | 7 = 0x87."""
        assert encode_instruction("ADD", ("A",), {}, 0) == bytes([0x87])

    def test_adc_e(self) -> None:
        """ADC E: 0x88 | 3 = 0x8B."""
        assert encode_instruction("ADC", ("E",), {}, 0) == bytes([0x8B])

    def test_sub_d(self) -> None:
        """SUB D: 0x90 | 2 = 0x92."""
        assert encode_instruction("SUB", ("D",), {}, 0) == bytes([0x92])

    def test_sbb_b(self) -> None:
        """SBB B: 0x98 | 0 = 0x98."""
        assert encode_instruction("SBB", ("B",), {}, 0) == bytes([0x98])

    def test_ana_c(self) -> None:
        """ANA C: 0xA0 | 1 = 0xA1."""
        assert encode_instruction("ANA", ("C",), {}, 0) == bytes([0xA1])

    def test_xra_h(self) -> None:
        """XRA H: 0xA8 | 4 = 0xAC."""
        assert encode_instruction("XRA", ("H",), {}, 0) == bytes([0xAC])

    def test_ora_l(self) -> None:
        """ORA L: 0xB0 | 5 = 0xB5."""
        assert encode_instruction("ORA", ("L",), {}, 0) == bytes([0xB5])

    def test_cmp_a(self) -> None:
        """CMP A: 0xB8 | 7 = 0xBF."""
        assert encode_instruction("CMP", ("A",), {}, 0) == bytes([0xBF])

    def test_cmp_b(self) -> None:
        """CMP B: 0xB8 | 0 = 0xB8."""
        assert encode_instruction("CMP", ("B",), {}, 0) == bytes([0xB8])

    def test_ora_a_for_flag_refresh(self) -> None:
        """ORA A (OR A with itself): 0xB0 | 7 = 0xB7 — used to refresh flags."""
        assert encode_instruction("ORA", ("A",), {}, 0) == bytes([0xB7])


class TestEncodeAluImm:
    """Tests for ALU immediate operations (Group 11, 2 bytes).

    Encoding: 11 OOO 100, d8  (group=11, sss=100, operation in bits[5:3])
    All opcodes are in the range 0xC4..0xFC with sss=100 (bit pattern xxx_100).
    """

    def test_adi_5(self) -> None:
        """ADI 5: [0xC4, 0x05] — 11_000_100, group=11, OOO=000 (ADD)."""
        assert encode_instruction("ADI", ("5",), {}, 0) == bytes([0xC4, 0x05])

    def test_aci_0(self) -> None:
        """ACI 0: [0xCC, 0x00] — used for carry() materialisation."""
        assert encode_instruction("ACI", ("0",), {}, 0) == bytes([0xCC, 0x00])

    def test_sui_1(self) -> None:
        """SUI 1: [0xD4, 0x01]."""
        assert encode_instruction("SUI", ("1",), {}, 0) == bytes([0xD4, 0x01])

    def test_sbi_0(self) -> None:
        """SBI 0: [0xDC, 0x00]."""
        assert encode_instruction("SBI", ("0",), {}, 0) == bytes([0xDC, 0x00])

    def test_ani_0xff(self) -> None:
        """ANI 0xFF: [0xE4, 0xFF]."""
        assert encode_instruction("ANI", ("0xFF",), {}, 0) == bytes([0xE4, 0xFF])

    def test_xri_0xff(self) -> None:
        """XRI 0xFF: [0xEC, 0xFF] — used for bitwise NOT (flip all bits)."""
        assert encode_instruction("XRI", ("0xFF",), {}, 0) == bytes([0xEC, 0xFF])

    def test_ori_1(self) -> None:
        """ORI 1: [0xF4, 0x01]."""
        assert encode_instruction("ORI", ("1",), {}, 0) == bytes([0xF4, 0x01])

    def test_cpi_0(self) -> None:
        """CPI 0: [0xFC, 0x00] — compare A with 0 (for BRANCH_Z/BRANCH_NZ)."""
        assert encode_instruction("CPI", ("0",), {}, 0) == bytes([0xFC, 0x00])

    def test_adi_out_of_range_raises(self) -> None:
        """ADI 256 raises AssemblerError."""
        with pytest.raises(AssemblerError, match="out of range"):
            encode_instruction("ADI", ("256",), {}, 0)


class TestEncodeJumpCall:
    """Tests for 3-byte jump and call instructions.

    Address encoding for 3-byte instructions:
      byte 2 = addr & 0xFF           (low 8 bits)
      byte 3 = (addr >> 8) & 0x3F   (high 6 bits)

    Opcode derivation:
      JMP = 0x7C (01_111_100): special unconditional, hardcoded in simulator
      CAL = 0x7E (01_111_110): special unconditional, hardcoded in simulator
      Conditional jumps: 01_CCC_T_00  (CCC=condition 0..3, T=sense bit)
      Conditional calls: 01_CCC_T_10
    """

    def test_jmp_addr_zero(self) -> None:
        """JMP 0x0000 → [0x7C, 0x00, 0x00] — unconditional jump."""
        result = encode_instruction("JMP", ("0x0000",), {}, 0)
        assert result == bytes([0x7C, 0x00, 0x00])

    def test_jmp_addr_10(self) -> None:
        """JMP 0x000A → [0x7C, 0x0A, 0x00]."""
        result = encode_instruction("JMP", ("0x000A",), {}, 0)
        assert result == bytes([0x7C, 0x0A, 0x00])

    def test_jmp_addr_crosses_256(self) -> None:
        """JMP 0x0300 → [0x7C, 0x00, 0x03] — hi6 = 3."""
        result = encode_instruction("JMP", ("0x0300",), {}, 0)
        assert result == bytes([0x7C, 0x00, 0x03])

    def test_jmp_addr_max(self) -> None:
        """JMP 0x3FFF → [0x7C, 0xFF, 0x3F] — maximum 14-bit address."""
        result = encode_instruction("JMP", ("0x3FFF",), {}, 0)
        assert result == bytes([0x7C, 0xFF, 0x3F])

    def test_cal(self) -> None:
        """CAL 0x0010 → [0x7E, 0x10, 0x00] — unconditional call."""
        result = encode_instruction("CAL", ("0x0010",), {}, 0)
        assert result == bytes([0x7E, 0x10, 0x00])

    def test_jfc(self) -> None:
        """JFC: opcode 0x40 (01_000_000: CCC=0=CY, T=0=false, bits[1:0]=00)."""
        result = encode_instruction("JFC", ("0x0005",), {}, 0)
        assert result == bytes([0x40, 0x05, 0x00])

    def test_jtc(self) -> None:
        """JTC: opcode 0x44 (01_000_100: CCC=0=CY, T=1=true)."""
        result = encode_instruction("JTC", ("0x0005",), {}, 0)
        assert result == bytes([0x44, 0x05, 0x00])

    def test_jfz(self) -> None:
        """JFZ: opcode 0x48 (01_001_000: CCC=1=Z, T=0=false)."""
        result = encode_instruction("JFZ", ("0x0005",), {}, 0)
        assert result == bytes([0x48, 0x05, 0x00])

    def test_jtz(self) -> None:
        """JTZ: opcode 0x4C (01_001_100: CCC=1=Z, T=1=true)."""
        result = encode_instruction("JTZ", ("0x0005",), {}, 0)
        assert result == bytes([0x4C, 0x05, 0x00])

    def test_jfs(self) -> None:
        """JFS: opcode 0x50 (01_010_000: CCC=2=S, T=0=false)."""
        result = encode_instruction("JFS", ("0x0005",), {}, 0)
        assert result == bytes([0x50, 0x05, 0x00])

    def test_jts(self) -> None:
        """JTS: opcode 0x54 (01_010_100: CCC=2=S, T=1=true)."""
        result = encode_instruction("JTS", ("0x0005",), {}, 0)
        assert result == bytes([0x54, 0x05, 0x00])

    def test_jfp(self) -> None:
        """JFP: opcode 0x58 (01_011_000: CCC=3=P, T=0=false) — jump if parity false."""
        result = encode_instruction("JFP", ("0x0005",), {}, 0)
        assert result == bytes([0x58, 0x05, 0x00])

    def test_jtp(self) -> None:
        """JTP: opcode 0x5C (01_011_100: CCC=3=P, T=1=true)."""
        result = encode_instruction("JTP", ("0x0005",), {}, 0)
        assert result == bytes([0x5C, 0x05, 0x00])

    def test_jmp_label_reference(self) -> None:
        """JMP with a label resolves via the symbol table."""
        syms = {"loop": 0x0020}
        encoded = encode_instruction("JMP", ("loop",), syms, 0)
        assert encoded == bytes([0x7C, 0x20, 0x00])

    def test_jmp_undefined_label_raises(self) -> None:
        """JMP to an undefined label raises AssemblerError."""
        with pytest.raises(AssemblerError, match="Undefined label"):
            encode_instruction("JMP", ("no_such_label",), {}, 0)

    def test_jmp_out_of_range_raises(self) -> None:
        """JMP to an address > 0x3FFF raises AssemblerError."""
        with pytest.raises(AssemblerError, match="out of range"):
            encode_instruction("JMP", ("0x4000",), {}, 0)

    def test_cal_label_reference(self) -> None:
        """CAL with a label resolves via the symbol table."""
        syms = {"func": 0x0100}
        encoded = encode_instruction("CAL", ("func",), syms, 0)
        assert encoded == bytes([0x7E, 0x00, 0x01])

    def test_jmp_dollar_sign(self) -> None:
        """JMP $ jumps to the current PC (self-loop)."""
        # At PC=0x0010, JMP $ → [0x7C, 0x10, 0x00]
        encoded = encode_instruction("JMP", ("$",), {}, 0x0010)
        assert encoded == bytes([0x7C, 0x10, 0x00])


class TestEncodeInOut:
    """Tests for IN p and OUT p (I/O port instructions)."""

    def test_in_port_0(self) -> None:
        """IN 0: 0x41 | (0<<3) = 0x41."""
        assert encode_instruction("IN", ("0",), {}, 0) == bytes([0x41])

    def test_in_port_1(self) -> None:
        """IN 1: 0x41 | (1<<3) = 0x49."""
        assert encode_instruction("IN", ("1",), {}, 0) == bytes([0x49])

    def test_in_port_7(self) -> None:
        """IN 7: 0x41 | (7<<3) = 0x79."""
        assert encode_instruction("IN", ("7",), {}, 0) == bytes([0x79])

    def test_in_port_out_of_range_raises(self) -> None:
        """IN 8 raises AssemblerError (max port is 7)."""
        with pytest.raises(AssemblerError, match="out of range"):
            encode_instruction("IN", ("8",), {}, 0)

    def test_out_port_0(self) -> None:
        """OUT 0: p<<1 = 0x00.

        Intel 8008 OUT encoding: opcode = port << 1.  The simulator detects
        OUT by matching group=00, sss=010, ddd>3 and extracts port via
        ``(opcode >> 1) & 0x1F``.
        """
        assert encode_instruction("OUT", ("0",), {}, 0) == bytes([0x00])

    def test_out_port_1(self) -> None:
        """OUT 1: p<<1 = 0x02."""
        assert encode_instruction("OUT", ("1",), {}, 0) == bytes([0x02])

    def test_out_port_2(self) -> None:
        """OUT 2: p<<1 = 0x04."""
        assert encode_instruction("OUT", ("2",), {}, 0) == bytes([0x04])

    def test_out_port_17(self) -> None:
        """OUT 17: p<<1 = 0x22 — the standard simulator-compatible port.

        17<<1 = 0x22 = 00_100_010.  group=00, sss=010, ddd=4 (>3) ✓
        The simulator extracts port as (0x22 >> 1) & 0x1F = 0x11 = 17.
        """
        assert encode_instruction("OUT", ("17",), {}, 0) == bytes([0x22])

    def test_out_port_23(self) -> None:
        """OUT 23: p<<1 = 0x2E."""
        assert encode_instruction("OUT", ("23",), {}, 0) == bytes([0x2E])

    def test_out_port_out_of_range_raises(self) -> None:
        """OUT 24 raises AssemblerError (max port is 23)."""
        with pytest.raises(AssemblerError, match="out of range"):
            encode_instruction("OUT", ("24",), {}, 0)


class TestEncodeInrDcrRst:
    """Tests for INR, DCR, and RST instructions."""

    def test_inr_b(self) -> None:
        """INR B: 0<<3 = 0x00."""
        assert encode_instruction("INR", ("B",), {}, 0) == bytes([0x00])

    def test_inr_d(self) -> None:
        """INR D: 2<<3 = 0x10."""
        assert encode_instruction("INR", ("D",), {}, 0) == bytes([0x10])

    def test_inr_h(self) -> None:
        """INR H: 4<<3 = 0x20."""
        assert encode_instruction("INR", ("H",), {}, 0) == bytes([0x20])

    def test_dcr_b(self) -> None:
        """DCR B: (0<<3)|1 = 0x01."""
        assert encode_instruction("DCR", ("B",), {}, 0) == bytes([0x01])

    def test_dcr_c(self) -> None:
        """DCR C: (1<<3)|1 = 0x09."""
        assert encode_instruction("DCR", ("C",), {}, 0) == bytes([0x09])

    def test_rst_0(self) -> None:
        """RST 0: (0<<3)|5 = 0x05."""
        assert encode_instruction("RST", ("0",), {}, 0) == bytes([0x05])

    def test_rst_1(self) -> None:
        """RST 1: (1<<3)|5 = 0x0D."""
        assert encode_instruction("RST", ("1",), {}, 0) == bytes([0x0D])

    def test_rst_7(self) -> None:
        """RST 7: (7<<3)|5 = 0x3D."""
        assert encode_instruction("RST", ("7",), {}, 0) == bytes([0x3D])

    def test_rst_out_of_range_raises(self) -> None:
        """RST 8 raises AssemblerError."""
        with pytest.raises(AssemblerError, match="out of range"):
            encode_instruction("RST", ("8",), {}, 0)


class TestEncodeUnknownMnemonic:
    """Test that unknown mnemonics are rejected."""

    def test_unknown_mnemonic(self) -> None:
        """Unknown mnemonic raises AssemblerError."""
        with pytest.raises(AssemblerError, match="Unknown mnemonic"):
            encode_instruction("BOGUS", (), {}, 0)


# ===========================================================================
# Assembler integration tests
# ===========================================================================

class TestHltProgram:
    """Minimal programs with HLT."""

    def test_hlt_only(self) -> None:
        """Single HLT instruction → [0xFF]."""
        assert assemble("    HLT") == bytes([0xFF])

    def test_org_then_hlt(self) -> None:
        """ORG 0x0000 then HLT → [0xFF]."""
        assert assemble("    ORG 0x0000\n    HLT") == bytes([0xFF])

    def test_org_with_padding(self) -> None:
        """ORG 0x0002 pads with 0xFF (erased flash state) then emits HLT."""
        binary = assemble("    ORG 0x0000\n    ORG 0x0002\n    HLT")
        assert binary == bytes([0xFF, 0xFF, 0xFF])


class TestLabelResolution:
    """Tests for forward and backward label resolution."""

    def test_backward_label_reference(self) -> None:
        """JMP to a label defined earlier (backward reference)."""
        src = """\
    ORG 0x0000
loop:
    HLT
    JMP  loop
"""
        binary = assemble(src)
        # HLT at 0x0000 = 0xFF
        # JMP loop at 0x0001 → [0x7C, 0x00, 0x00]  (JMP=0x7C unconditional)
        assert binary == bytes([0xFF, 0x7C, 0x00, 0x00])

    def test_forward_label_reference(self) -> None:
        """JMP to a label defined later (forward reference)."""
        src = """\
    ORG 0x0000
    JMP  done
    MVI  B, 42
done:
    HLT
"""
        binary = assemble(src)
        # JMP done: at 0x0000 → [0x7C, lo, hi] where done = 0x0005
        # MVI B, 42: at 0x0003 → [0x06, 0x2A]
        # HLT: at 0x0005 → [0xFF]
        assert binary == bytes([0x7C, 0x05, 0x00, 0x06, 0x2A, 0xFF])

    def test_multiple_labels(self) -> None:
        """Multiple labels in one program all resolve correctly."""
        src = """\
    ORG 0x0000
start:
    CAL  middle
    HLT
middle:
    RFC
"""
        binary = assemble(src)
        # CAL middle: at 0x0000 → [0x7E, 0x04, 0x00]  (CAL=0x7E, middle=0x0004)
        # HLT:        at 0x0003 → [0xFF]
        # RFC:        at 0x0004 → [0x03]  (RFC = 00_000_011, T=0 → carry-false return)
        assert binary == bytes([0x7E, 0x04, 0x00, 0xFF, 0x03])

    def test_label_in_conditional_jump(self) -> None:
        """JTZ with a label resolves correctly."""
        src = """\
    ORG 0x0000
    CPI  0
    JTZ  done
    HLT
done:
    RFC
"""
        binary = assemble(src)
        # CPI 0:    [0xFC, 0x00]  at 0x0000  (CPI = 11_111_100 = 0xFC)
        # JTZ done: [0x4C, lo, hi] where done = 0x0006  at 0x0002
        #           JTZ = 01_001_100 = 0x4C  (CCC=1=zero, T=1=true)
        # HLT:      [0xFF]         at 0x0005
        # RFC:      [0x03]         at 0x0006  (RFC = 00_000_011 = 0x03)
        assert binary == bytes([0xFC, 0x00, 0x4C, 0x06, 0x00, 0xFF, 0x03])


class TestHiLoDirectives:
    """Tests for hi(sym) and lo(sym) in MVI instructions."""

    def test_load_addr_sequence(self) -> None:
        """MVI H, hi(sym); MVI L, lo(sym) for static variable address.

        With sym at address 0x2010:
          hi(0x2010) = (0x2010 >> 8) & 0x3F = 0x20
          lo(0x2010) = 0x2010 & 0xFF = 0x10
        """
        src = """\
    ORG 0x0000
    MVI  H, hi(sym)
    MVI  L, lo(sym)
sym:
    HLT
"""
        binary = assemble(src)
        # MVI H, hi(sym): sym is at 0x0004
        #   hi(0x0004) = (0x0004 >> 8) & 0x3F = 0x00
        # MVI L, lo(sym): lo(0x0004) = 0x0004 & 0xFF = 0x04
        # MVI H, 0x00: [0x26, 0x00]
        # MVI L, 0x04: [0x2E, 0x04]
        # HLT: [0xFF]
        assert binary == bytes([0x26, 0x00, 0x2E, 0x04, 0xFF])

    def test_hi_lo_for_ram_address(self) -> None:
        """hi()/lo() of a RAM address (0x2000)."""
        # We'll define a label at ORG 0x2000 to simulate a static var
        src = """\
    ORG 0x0000
    MVI  H, hi(counter)
    MVI  L, lo(counter)
    HLT
    ORG 0x2000
counter:
    HLT
"""
        binary = assemble(src)
        # MVI H, hi(0x2000): hi = (0x2000 >> 8) & 0x3F = 0x20
        # MVI L, lo(0x2000): lo = 0x2000 & 0xFF = 0x00
        # MVI H: [0x26, 0x20]
        # MVI L: [0x2E, 0x00]
        # HLT:   [0xFF] at 0x0004
        # then padding from 0x0005 to 0x2000 is 0xFF bytes
        # counter: HLT at 0x2000
        prefix = bytes([0x26, 0x20, 0x2E, 0x00, 0xFF])
        padding = bytes([0xFF] * (0x2000 - 0x0005))
        suffix = bytes([0xFF])  # HLT at counter
        expected = prefix + padding + suffix
        assert binary == expected


class TestCompletePrograms:
    """Full programs matching expected byte sequences."""

    def test_minimal_main_call(self) -> None:
        """Entry stub: MVI B, 0; CAL func; HLT then func: MOV A, B; RFC."""
        src = """\
    ORG 0x0000
_start:
    MVI  B, 0
    CAL  _fn_main
    HLT
_fn_main:
    MOV  A, B
    RFC
"""
        binary = assemble(src)
        # MVI B, 0:    [0x06, 0x00]  at 0x0000
        # CAL _fn_main: [0x7E, 0x06, 0x00] at 0x0002  (CAL=0x7E, _fn_main=0x0006)
        # HLT:         [0xFF]         at 0x0005
        # MOV A, B:    [0x78]         at 0x0006  (0x40|(7<<3)|0 = 0x78)
        # RFC:         [0x03]         at 0x0007  (RFC = 00_000_011 = 0x03)
        assert binary == bytes([
            0x06, 0x00,       # MVI B, 0
            0x7E, 0x06, 0x00, # CAL _fn_main  (CAL=0x7E unconditional)
            0xFF,             # HLT
            0x78,             # MOV A, B
            0x03,             # RFC  (carry-false → always returns since ALU clears CY)
        ])

    def test_comparison_sequence(self) -> None:
        """CMP_EQ materialisation sequence.

        MOV A, Ra; CMP Rb; MVI Rdst, 1; JTZ done; MVI Rdst, 0
        """
        src = """\
    ORG 0x0000
    MOV  A, B
    CMP  C
    MVI  D, 1
    JTZ  cmp_done
    MVI  D, 0
cmp_done:
    HLT
"""
        binary = assemble(src)
        # MOV A, B:    [0x78]         at 0x0000
        # CMP C:       [0xB9]         at 0x0001  (0xB8|1 = 0xB9)
        # MVI D, 1:    [0x16, 0x01]   at 0x0002  ((2<<3)|6 = 0x16)
        # JTZ cmp_done: [0x4C, 0x09, 0x00] at 0x0004  (JTZ=0x4C, cmp_done=0x0009)
        # MVI D, 0:    [0x16, 0x00]   at 0x0007
        # HLT:         [0xFF]         at 0x0009
        assert binary == bytes([
            0x78,             # MOV A, B
            0xB9,             # CMP C
            0x16, 0x01,       # MVI D, 1
            0x4C, 0x09, 0x00, # JTZ cmp_done  (JTZ = 01_001_100 = 0x4C)
            0x16, 0x00,       # MVI D, 0
            0xFF,             # HLT (cmp_done)
        ])

    def test_alu_sequence(self) -> None:
        """MOV A, Ra; ADD Rb; MOV Rdst, A — the standard ADD pattern."""
        src = """\
    ORG 0x0000
    MOV  A, C
    ADD  D
    MOV  B, A
    HLT
"""
        binary = assemble(src)
        # MOV A, C:  0x40|(7<<3)|1 = 0x79
        # ADD D:     0x80|2 = 0x82
        # MOV B, A:  0x40|(0<<3)|7 = 0x47
        # HLT:       0xFF
        assert binary == bytes([0x79, 0x82, 0x47, 0xFF])

    def test_rotate_sequence(self) -> None:
        """Rotation: MOV A, D; RLC; MOV C, A."""
        src = """\
    ORG 0x0000
    MOV  A, D
    RLC
    MOV  C, A
    HLT
"""
        binary = assemble(src)
        # MOV A, D:  0x40|(7<<3)|2 = 0x7A
        # RLC:       0x02
        # MOV C, A:  0x40|(1<<3)|7 = 0x4F
        # HLT:       0xFF
        assert binary == bytes([0x7A, 0x02, 0x4F, 0xFF])

    def test_not_via_xri(self) -> None:
        """NOT via XRI 0xFF: MOV A, Ra; XRI 0xFF; MOV Rdst, A."""
        src = """\
    ORG 0x0000
    MOV  A, B
    XRI  0xFF
    MOV  C, A
    HLT
"""
        binary = assemble(src)
        # MOV A, B:  0x78
        # XRI 0xFF:  0xEC, 0xFF  (XRI = 11_101_100 = 0xEC; was wrong 0x2C = group 00)
        # MOV C, A:  0x4F
        # HLT:       0xFF
        assert binary == bytes([0x78, 0xEC, 0xFF, 0x4F, 0xFF])

    def test_io_operations(self) -> None:
        """IN p (read) and OUT p (write) sequences."""
        src = """\
    ORG 0x0000
    IN   0
    MOV  C, A
    MOV  A, D
    OUT  1
    HLT
"""
        binary = assemble(src)
        # IN 0:    0x41  (01_000_001)
        # MOV C,A: 0x4F  (0x40|(1<<3)|7)
        # MOV A,D: 0x7A  (0x40|(7<<3)|2)
        # OUT 1:   1<<1 = 0x02
        # HLT:     0xFF
        assert binary == bytes([0x41, 0x4F, 0x7A, 0x02, 0xFF])


class TestOrgDirective:
    """Tests for the ORG directive."""

    def test_org_at_zero(self) -> None:
        """ORG 0x0000 is the default starting address."""
        binary = assemble("    ORG 0x0000\n    HLT")
        assert binary == bytes([0xFF])

    def test_org_nonzero_pads_with_ff(self) -> None:
        """ORG 0x0003 pads the first 3 bytes with 0xFF (erased flash)."""
        binary = assemble("    ORG 0x0003\n    HLT")
        assert binary == bytes([0xFF, 0xFF, 0xFF, 0xFF])

    def test_org_overflow_raises(self) -> None:
        """ORG with address > 0x3FFF raises AssemblerError."""
        with pytest.raises(AssemblerError, match="exceeds Intel 8008"):
            assemble("    ORG 0x4000\n    HLT")

    def test_org_missing_operand_raises(self) -> None:
        """ORG with no operand raises AssemblerError."""
        with pytest.raises(AssemblerError, match="ORG requires"):
            assemble("    ORG\n    HLT")


class TestErrorCases:
    """Tests for error handling in the assembler."""

    def test_undefined_label_raises(self) -> None:
        """JMP to undefined label raises AssemblerError."""
        with pytest.raises(AssemblerError, match="Undefined label"):
            assemble("    JMP  no_such_label")

    def test_unknown_mnemonic_raises(self) -> None:
        """Unknown mnemonic raises AssemblerError."""
        with pytest.raises(AssemblerError, match="Unknown mnemonic"):
            assemble("    BOGUS  B, 42")

    def test_immediate_out_of_range_raises(self) -> None:
        """MVI with immediate > 255 raises AssemblerError."""
        with pytest.raises(AssemblerError, match="out of range"):
            assemble("    MVI  B, 300")

    def test_invalid_org_literal_raises(self) -> None:
        """ORG with non-numeric operand raises AssemblerError."""
        with pytest.raises(AssemblerError, match="Invalid address literal"):
            assemble("    ORG notanumber")


class TestAssemblerClass:
    """Tests for the Intel8008Assembler class."""

    def test_reusable(self) -> None:
        """Assembler instance can be reused for multiple programs."""
        asm = Intel8008Assembler()
        b1 = asm.assemble("    HLT")
        b2 = asm.assemble("    RFC")
        assert b1 == bytes([0xFF])
        assert b2 == bytes([0x03])

    def test_stateless_between_calls(self) -> None:
        """Symbol table is not shared between calls."""
        asm = Intel8008Assembler()
        # First call defines 'x'
        asm.assemble("    ORG 0x0000\nx:\n    HLT")
        # Second call should not know about 'x'
        with pytest.raises(AssemblerError, match="Undefined label"):
            asm.assemble("    JMP  x")


class TestConvenienceFunction:
    """Tests for the module-level assemble() function."""

    def test_assemble_function(self) -> None:
        """Module-level assemble() delegates to Intel8008Assembler."""
        binary = assemble("    HLT")
        assert binary == bytes([0xFF])
