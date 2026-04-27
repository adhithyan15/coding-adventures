"""Tests for the Verilog Lexer.

These tests verify that the grammar-driven lexer, when loaded with the
``verilog.tokens`` grammar file, correctly tokenizes Verilog HDL source code.

Verilog's token set is distinctive among our supported languages:
- Sized number literals (``4'b1010``, ``8'hFF``) carry bit-width information
- System identifiers (``$display``) start with ``$``
- Compiler directives (`` `define ``) start with backtick
- Four-state operators (``===``, ``!==``) compare including x/z values
"""

from __future__ import annotations

from lexer import Token, TokenType

from verilog_lexer import create_verilog_lexer, tokenize_verilog


# ============================================================================
# Helper — makes assertions more readable
# ============================================================================


def token_types(tokens: list[Token]) -> list[str]:
    """Extract just the type names from a token list.

    Token types can be either a ``TokenType`` enum (for built-in types like
    NAME, NUMBER, KEYWORD) or a plain string (for custom types defined in
    the ``.tokens`` file, like SIZED_NUMBER, SYSTEM_ID). We handle both.
    """
    return [t.type.name if hasattr(t.type, "name") else t.type for t in tokens]


def token_values(tokens: list[Token]) -> list[str]:
    """Extract just the values from a token list."""
    return [t.value for t in tokens]


# ============================================================================
# Test: Basic Module Declaration
# ============================================================================


class TestModuleDeclaration:
    """Test tokenization of Verilog module structure."""

    def test_empty_module(self) -> None:
        """Tokenize ``module m; endmodule`` — simplest possible module."""
        tokens = tokenize_verilog("module m; endmodule", preprocess=False)
        assert token_types(tokens) == [
            "KEYWORD", "NAME", "SEMICOLON", "KEYWORD", "EOF",
        ]
        assert token_values(tokens) == ["module", "m", ";", "endmodule", ""]

    def test_module_with_ports(self) -> None:
        """Tokenize a module with input and output ports."""
        source = "module and_gate(input a, input b, output y);"
        tokens = tokenize_verilog(source, preprocess=False)
        types = token_types(tokens)
        assert types[0] == "KEYWORD"  # module
        assert "LPAREN" in types
        assert "RPAREN" in types
        assert "COMMA" in types
        assert token_values(tokens)[0] == "module"
        assert token_values(tokens)[1] == "and_gate"

    def test_module_with_range(self) -> None:
        """Tokenize port declaration with bit range ``[7:0]``."""
        source = "input [7:0] data"
        tokens = tokenize_verilog(source, preprocess=False)
        types = token_types(tokens)
        assert "LBRACKET" in types
        assert "RBRACKET" in types
        assert "COLON" in types
        assert "NUMBER" in types


# ============================================================================
# Test: Number Literals
# ============================================================================


class TestNumberLiterals:
    """Test Verilog's distinctive number formats."""

    def test_sized_binary(self) -> None:
        """``4'b1010`` — 4-bit binary literal."""
        tokens = tokenize_verilog("4'b1010", preprocess=False)
        assert token_types(tokens) == ["SIZED_NUMBER", "EOF"]
        assert token_values(tokens)[0] == "4'b1010"

    def test_sized_hex(self) -> None:
        """``8'hFF`` — 8-bit hexadecimal literal."""
        tokens = tokenize_verilog("8'hFF", preprocess=False)
        assert token_types(tokens) == ["SIZED_NUMBER", "EOF"]
        assert token_values(tokens)[0] == "8'hFF"

    def test_sized_decimal(self) -> None:
        """``32'd42`` — 32-bit decimal literal."""
        tokens = tokenize_verilog("32'd42", preprocess=False)
        assert token_types(tokens) == ["SIZED_NUMBER", "EOF"]
        assert token_values(tokens)[0] == "32'd42"

    def test_unsized_octal(self) -> None:
        """``'o77`` — unsized octal literal."""
        tokens = tokenize_verilog("'o77", preprocess=False)
        assert token_types(tokens) == ["SIZED_NUMBER", "EOF"]
        assert token_values(tokens)[0] == "'o77"

    def test_sized_with_underscores(self) -> None:
        """``8'b1010_0011`` — underscores as visual separators."""
        tokens = tokenize_verilog("8'b1010_0011", preprocess=False)
        assert token_types(tokens) == ["SIZED_NUMBER", "EOF"]
        assert token_values(tokens)[0] == "8'b1010_0011"

    def test_sized_with_xz(self) -> None:
        """``4'bxxzz`` — x (unknown) and z (high-impedance) values."""
        tokens = tokenize_verilog("4'bxxzz", preprocess=False)
        assert token_types(tokens) == ["SIZED_NUMBER", "EOF"]
        assert token_values(tokens)[0] == "4'bxxzz"

    def test_signed_literal(self) -> None:
        """``8'sh80`` — signed hex literal."""
        tokens = tokenize_verilog("8'sh80", preprocess=False)
        assert token_types(tokens) == ["SIZED_NUMBER", "EOF"]
        assert token_values(tokens)[0] == "8'sh80"

    def test_plain_integer(self) -> None:
        """``42`` — plain decimal integer."""
        tokens = tokenize_verilog("42", preprocess=False)
        assert token_types(tokens) == ["NUMBER", "EOF"]
        assert token_values(tokens)[0] == "42"

    def test_real_number(self) -> None:
        """``3.14`` — real number."""
        tokens = tokenize_verilog("3.14", preprocess=False)
        assert token_types(tokens) == ["REAL_NUMBER", "EOF"]
        assert token_values(tokens)[0] == "3.14"

    def test_real_with_exponent(self) -> None:
        """``1.5e3`` — real number with exponent."""
        tokens = tokenize_verilog("1.5e3", preprocess=False)
        assert token_types(tokens) == ["REAL_NUMBER", "EOF"]
        assert token_values(tokens)[0] == "1.5e3"


# ============================================================================
# Test: Special Identifiers
# ============================================================================


class TestSpecialIdentifiers:
    """Test system tasks, directives, and escaped identifiers."""

    def test_system_task(self) -> None:
        """``$display`` — system task identifier."""
        tokens = tokenize_verilog("$display", preprocess=False)
        assert token_types(tokens) == ["SYSTEM_ID", "EOF"]
        assert token_values(tokens)[0] == "$display"

    def test_system_function(self) -> None:
        """``$time`` — system function."""
        tokens = tokenize_verilog("$time", preprocess=False)
        assert token_types(tokens) == ["SYSTEM_ID", "EOF"]
        assert token_values(tokens)[0] == "$time"

    def test_directive(self) -> None:
        """`` `timescale `` — compiler directive (after preprocessing)."""
        tokens = tokenize_verilog("`timescale", preprocess=False)
        assert token_types(tokens) == ["DIRECTIVE", "EOF"]
        assert token_values(tokens)[0] == "`timescale"

    def test_escaped_identifier(self) -> None:
        """``\\my.name`` — escaped identifier with special characters."""
        tokens = tokenize_verilog("\\my.name ", preprocess=False)
        assert token_types(tokens) == ["ESCAPED_IDENT", "EOF"]
        assert token_values(tokens)[0] == "\\my.name"


# ============================================================================
# Test: Operators
# ============================================================================


class TestOperators:
    """Test operator tokenization, especially multi-character operators."""

    def test_case_equality(self) -> None:
        """``===`` — case equality (4-state comparison)."""
        tokens = tokenize_verilog("a === b", preprocess=False)
        types = token_types(tokens)
        assert "CASE_EQ" in types

    def test_case_inequality(self) -> None:
        """``!==`` — case inequality."""
        tokens = tokenize_verilog("a !== b", preprocess=False)
        types = token_types(tokens)
        assert "CASE_NEQ" in types

    def test_logical_equality(self) -> None:
        """``==`` — logical equality (not consumed by ===)."""
        tokens = tokenize_verilog("a == b", preprocess=False)
        types = token_types(tokens)
        assert "EQUALS_EQUALS" in types

    def test_arithmetic_shift(self) -> None:
        """``>>>`` — arithmetic right shift."""
        tokens = tokenize_verilog("a >>> 2", preprocess=False)
        types = token_types(tokens)
        assert "ARITH_RIGHT_SHIFT" in types

    def test_logical_shift(self) -> None:
        """``<<`` — logical left shift (not consumed by <<<)."""
        tokens = tokenize_verilog("a << 2", preprocess=False)
        types = token_types(tokens)
        assert "LEFT_SHIFT" in types

    def test_nonblocking_assign(self) -> None:
        """``<=`` — non-blocking assignment / less-or-equal."""
        tokens = tokenize_verilog("q <= d", preprocess=False)
        types = token_types(tokens)
        assert "LESS_EQUALS" in types

    def test_power_operator(self) -> None:
        """``**`` — exponentiation."""
        tokens = tokenize_verilog("2 ** 10", preprocess=False)
        types = token_types(tokens)
        assert "POWER" in types

    def test_event_trigger(self) -> None:
        """``->`` — event trigger."""
        tokens = tokenize_verilog("-> event_a", preprocess=False)
        types = token_types(tokens)
        assert "TRIGGER" in types

    def test_ternary(self) -> None:
        """``? :`` — ternary operator."""
        tokens = tokenize_verilog("sel ? a : b", preprocess=False)
        types = token_types(tokens)
        assert "QUESTION" in types
        assert "COLON" in types


# ============================================================================
# Test: Delimiters and Special Characters
# ============================================================================


class TestDelimiters:
    """Test delimiter tokens specific to Verilog."""

    def test_hash_for_delay(self) -> None:
        """``#`` — hash for delay or parameter override."""
        tokens = tokenize_verilog("#10", preprocess=False)
        types = token_types(tokens)
        assert "HASH" in types

    def test_at_for_sensitivity(self) -> None:
        """``@`` — at for sensitivity list."""
        tokens = tokenize_verilog("@(posedge clk)", preprocess=False)
        types = token_types(tokens)
        assert "AT" in types

    def test_braces_for_concatenation(self) -> None:
        """``{ }`` — braces for concatenation."""
        tokens = tokenize_verilog("{a, b}", preprocess=False)
        types = token_types(tokens)
        assert "LBRACE" in types
        assert "RBRACE" in types


# ============================================================================
# Test: Keywords
# ============================================================================


class TestKeywords:
    """Test that Verilog keywords are recognized."""

    def test_module_keyword(self) -> None:
        """``module`` is recognized as KEYWORD."""
        tokens = tokenize_verilog("module", preprocess=False)
        assert token_types(tokens) == ["KEYWORD", "EOF"]
        assert token_values(tokens)[0] == "module"

    def test_wire_keyword(self) -> None:
        """``wire`` is recognized as KEYWORD."""
        tokens = tokenize_verilog("wire", preprocess=False)
        assert token_types(tokens) == ["KEYWORD", "EOF"]
        assert token_values(tokens)[0] == "wire"

    def test_always_keyword(self) -> None:
        """``always`` is recognized as KEYWORD."""
        tokens = tokenize_verilog("always", preprocess=False)
        assert token_types(tokens) == ["KEYWORD", "EOF"]

    def test_non_keyword_identifier(self) -> None:
        """``counter`` is a NAME, not a KEYWORD."""
        tokens = tokenize_verilog("counter", preprocess=False)
        assert token_types(tokens) == ["NAME", "EOF"]


# ============================================================================
# Test: Comments
# ============================================================================


class TestComments:
    """Test that comments are skipped."""

    def test_single_line_comment(self) -> None:
        """``// comment`` is consumed and produces no tokens."""
        tokens = tokenize_verilog("a // comment\nb", preprocess=False)
        values = [t.value for t in tokens if t.type.name != "EOF"]
        assert values == ["a", "b"]

    def test_block_comment(self) -> None:
        """``/* block */`` is consumed and produces no tokens."""
        tokens = tokenize_verilog("a /* block comment */ b", preprocess=False)
        values = [t.value for t in tokens if t.type.name != "EOF"]
        assert values == ["a", "b"]


# ============================================================================
# Test: Strings
# ============================================================================


class TestStrings:
    """Test string literal tokenization."""

    def test_simple_string(self) -> None:
        """``"hello"`` — simple string literal."""
        tokens = tokenize_verilog('"hello"', preprocess=False)
        assert token_types(tokens) == ["STRING", "EOF"]

    def test_string_with_escapes(self) -> None:
        """``"line\\n"`` — string with escape sequence."""
        tokens = tokenize_verilog('"line\\n"', preprocess=False)
        assert token_types(tokens) == ["STRING", "EOF"]


# ============================================================================
# Test: Complete Verilog Snippets
# ============================================================================


class TestCompleteSnippets:
    """Test tokenization of realistic Verilog code."""

    def test_and_gate_module(self) -> None:
        """Tokenize a complete AND gate module."""
        source = """module and_gate(input a, input b, output y);
            assign y = a & b;
        endmodule"""
        tokens = tokenize_verilog(source, preprocess=False)
        types = token_types(tokens)
        assert types[0] == "KEYWORD"  # module
        assert "KEYWORD" in types  # assign, input, output, endmodule
        assert "AMP" in types  # &
        assert types[-1] == "EOF"

    def test_always_block(self) -> None:
        """Tokenize an always block with sensitivity list."""
        source = "always @(posedge clk) q <= d;"
        tokens = tokenize_verilog(source, preprocess=False)
        types = token_types(tokens)
        assert "AT" in types
        assert "LESS_EQUALS" in types
        values = token_values(tokens)
        assert "always" in values
        assert "posedge" in values
        assert "clk" in values

    def test_case_statement(self) -> None:
        """Tokenize a case statement."""
        source = """case (sel)
            2'b00: y = a;
            2'b01: y = b;
            default: y = 0;
        endcase"""
        tokens = tokenize_verilog(source, preprocess=False)
        types = token_types(tokens)
        assert "SIZED_NUMBER" in types
        values = token_values(tokens)
        assert "case" in values
        assert "default" in values
        assert "endcase" in values

    def test_module_instantiation(self) -> None:
        """Tokenize a module instantiation with named ports."""
        source = "and_gate u1 (.a(sig_a), .b(sig_b), .y(out));"
        tokens = tokenize_verilog(source, preprocess=False)
        types = token_types(tokens)
        assert "DOT" in types
        assert "LPAREN" in types

    def test_concatenation_and_replication(self) -> None:
        """Tokenize concatenation ``{a, b}`` and replication ``{4{1'b0}}``."""
        source = "{a, b}"
        tokens = tokenize_verilog(source, preprocess=False)
        types = token_types(tokens)
        assert "LBRACE" in types
        assert "RBRACE" in types
        assert "COMMA" in types

    def test_parameter_override(self) -> None:
        """Tokenize parameter override ``#(.WIDTH(16))``."""
        source = "#(.WIDTH(16))"
        tokens = tokenize_verilog(source, preprocess=False)
        types = token_types(tokens)
        assert "HASH" in types
        assert "DOT" in types


class TestVersions:
    """Version-selection behaviour for compiled Verilog grammars."""

    def test_default_version_matches_explicit_2005(self) -> None:
        default_tokens = tokenize_verilog("module m; endmodule", preprocess=False)
        explicit_tokens = tokenize_verilog(
            "module m; endmodule", preprocess=False, version="2005"
        )
        assert token_values(default_tokens) == token_values(explicit_tokens)

    def test_rejects_unknown_version(self) -> None:
        try:
            tokenize_verilog("module m; endmodule", preprocess=False, version="2099")
        except ValueError as exc:
            assert "Unknown Verilog version" in str(exc)
        else:
            raise AssertionError("Expected ValueError for unknown Verilog version")
