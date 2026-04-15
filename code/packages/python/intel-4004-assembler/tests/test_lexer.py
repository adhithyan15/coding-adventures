"""test_lexer.py — Unit tests for the Intel 4004 assembly lexer.

The lexer converts raw text lines into structured ``ParsedLine`` objects.
Each test validates a specific input pattern.
"""

from __future__ import annotations

import pytest

from intel_4004_assembler.lexer import ParsedLine, lex_line, lex_program


class TestBlankAndCommentLines:
    """Blank lines and comment-only lines produce a ParsedLine with no mnemonic."""

    def test_blank_line(self) -> None:
        result = lex_line("")
        assert result.label is None
        assert result.mnemonic is None
        assert result.operands == ()

    def test_whitespace_only_line(self) -> None:
        result = lex_line("    \t   ")
        assert result.mnemonic is None

    def test_comment_only_line(self) -> None:
        result = lex_line("; this is a comment")
        assert result.label is None
        assert result.mnemonic is None
        assert result.operands == ()

    def test_comment_with_leading_whitespace(self) -> None:
        result = lex_line("    ; indented comment")
        assert result.mnemonic is None

    def test_comment_preserves_source(self) -> None:
        source = "  ; full source preserved"
        result = lex_line(source)
        assert result.source == source


class TestLabelOnlyLines:
    """Label-only lines declare a symbol but have no mnemonic."""

    def test_simple_label(self) -> None:
        result = lex_line("loop_start:")
        assert result.label == "loop_start"
        assert result.mnemonic is None
        assert result.operands == ()

    def test_indented_label(self) -> None:
        result = lex_line("    _start:")
        assert result.label == "_start"
        assert result.mnemonic is None

    def test_label_with_trailing_comment(self) -> None:
        result = lex_line("my_label: ; a label")
        assert result.label == "my_label"
        assert result.mnemonic is None

    def test_label_with_numbers(self) -> None:
        result = lex_line("loop_0_end:")
        assert result.label == "loop_0_end"

    def test_label_case_preserved(self) -> None:
        """Labels are case-sensitive; the lexer must NOT uppercase them."""
        result = lex_line("MyLabel:")
        assert result.label == "MyLabel"


class TestInstructionLines:
    """Instruction lines produce the correct mnemonic and operands."""

    def test_nop_no_operands(self) -> None:
        result = lex_line("    NOP")
        assert result.label is None
        assert result.mnemonic == "NOP"
        assert result.operands == ()

    def test_mnemonic_is_uppercased(self) -> None:
        """The lexer normalises mnemonics to uppercase."""
        result = lex_line("    nop")
        assert result.mnemonic == "NOP"

    def test_ldm_single_operand(self) -> None:
        result = lex_line("    LDM 5")
        assert result.mnemonic == "LDM"
        assert result.operands == ("5",)

    def test_xch_register_operand(self) -> None:
        result = lex_line("    XCH R2")
        assert result.mnemonic == "XCH"
        assert result.operands == ("R2",)

    def test_jcn_two_operands(self) -> None:
        result = lex_line("    JCN 0x4, loop_0_end")
        assert result.mnemonic == "JCN"
        assert result.operands == ("0x4", "loop_0_end")

    def test_fim_two_operands(self) -> None:
        result = lex_line("    FIM P0, 0x42")
        assert result.mnemonic == "FIM"
        assert result.operands == ("P0", "0x42")

    def test_add_imm_three_operands(self) -> None:
        result = lex_line("    ADD_IMM R2, R2, 1")
        assert result.mnemonic == "ADD_IMM"
        assert result.operands == ("R2", "R2", "1")

    def test_operands_are_stripped(self) -> None:
        """Extra whitespace around operands must be stripped."""
        result = lex_line("    JCN  0x4 ,  loop_end")
        assert result.operands == ("0x4", "loop_end")


class TestInlineComments:
    """Inline ';' comments are stripped; only the instruction part remains."""

    def test_inline_comment_stripped(self) -> None:
        result = lex_line("    LDM 5 ; load 5")
        assert result.mnemonic == "LDM"
        assert result.operands == ("5",)

    def test_inline_comment_does_not_affect_operands(self) -> None:
        result = lex_line("    JCN 0x4, end_lbl ; jump if zero")
        assert result.operands == ("0x4", "end_lbl")


class TestLabelAndInstructionOnSameLine:
    """A line may carry both a label definition and an instruction."""

    def test_label_plus_instruction(self) -> None:
        result = lex_line("_start: NOP")
        assert result.label == "_start"
        assert result.mnemonic == "NOP"
        assert result.operands == ()

    def test_label_plus_instruction_with_operand(self) -> None:
        result = lex_line("entry: LDM 3")
        assert result.label == "entry"
        assert result.mnemonic == "LDM"
        assert result.operands == ("3",)

    def test_label_plus_instruction_with_comment(self) -> None:
        result = lex_line("start: NOP ; first instruction")
        assert result.label == "start"
        assert result.mnemonic == "NOP"


class TestOrgDirective:
    """ORG directive parsing."""

    def test_org_hex_address(self) -> None:
        result = lex_line("    ORG 0x000")
        assert result.mnemonic == "ORG"
        assert result.operands == ("0x000",)

    def test_org_decimal_address(self) -> None:
        result = lex_line("    ORG 256")
        assert result.mnemonic == "ORG"
        assert result.operands == ("256",)


class TestDollarOperand:
    """``$`` as an operand in JUN (self-loop)."""

    def test_dollar_as_operand(self) -> None:
        result = lex_line("    JUN $")
        assert result.mnemonic == "JUN"
        assert result.operands == ("$",)

    def test_dollar_with_comment(self) -> None:
        result = lex_line("    JUN $ ; self-loop halt")
        assert result.mnemonic == "JUN"
        assert result.operands == ("$",)


class TestLexProgram:
    """``lex_program`` processes multi-line text."""

    def test_line_count_matches(self) -> None:
        src = "    NOP\n    HLT\n    LDM 5"
        lines = lex_program(src)
        assert len(lines) == 3

    def test_empty_string(self) -> None:
        lines = lex_program("")
        # splitlines("") returns [] so we get zero lines
        assert lines == []

    def test_multi_line_program(self) -> None:
        src = """\
    ORG 0x000
_start:
    LDM 5
    HLT
"""
        lines = lex_program(src)
        mnemonics = [ln.mnemonic for ln in lines if ln.mnemonic]
        assert mnemonics == ["ORG", "LDM", "HLT"]
        labels = [ln.label for ln in lines if ln.label]
        assert labels == ["_start"]

    def test_all_returns_parsed_line_objects(self) -> None:
        lines = lex_program("    NOP\n    HLT")
        for ln in lines:
            assert isinstance(ln, ParsedLine)
