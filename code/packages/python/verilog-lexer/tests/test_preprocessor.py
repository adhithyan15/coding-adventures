"""Tests for the Verilog Preprocessor.

The Verilog preprocessor is a ``pre_tokenize`` hook that processes
directives like `` `define ``, `` `ifdef ``, and `` `include `` before
the lexer sees the source text.

These tests verify the preprocessor in isolation (str → str transform)
and in integration with the lexer.
"""

from __future__ import annotations

from verilog_lexer.preprocessor import verilog_preprocess
from verilog_lexer import tokenize_verilog


# ============================================================================
# Test: Simple Macro Definition and Expansion
# ============================================================================


class TestSimpleMacros:
    """Test `define and macro expansion."""

    def test_define_and_expand(self) -> None:
        """`` `define WIDTH 8 `` followed by `` `WIDTH `` expands to ``8``."""
        source = "`define WIDTH 8\nwire [`WIDTH-1:0] data;"
        result = verilog_preprocess(source)
        assert "8-1:0" in result
        assert "`WIDTH" not in result

    def test_define_no_value(self) -> None:
        """`` `define FLAG `` defines a flag with empty value."""
        source = "`define FLAG\n`FLAG"
        result = verilog_preprocess(source)
        lines = result.split("\n")
        assert lines[1] == ""  # Empty expansion

    def test_multiple_macros(self) -> None:
        """Multiple macros can be defined and expanded independently."""
        source = "`define A 1\n`define B 2\nx = `A + `B;"
        result = verilog_preprocess(source)
        assert "x = 1 + 2;" in result

    def test_macro_in_expression(self) -> None:
        """Macro inside a larger expression."""
        source = "`define SIZE 16\nassign out = data[`SIZE-1:0];"
        result = verilog_preprocess(source)
        assert "data[16-1:0]" in result

    def test_undef(self) -> None:
        """`` `undef `` removes a macro definition."""
        source = "`define X 5\na = `X;\n`undef X\nb = `X;"
        result = verilog_preprocess(source)
        lines = result.split("\n")
        assert "a = 5;" in lines[1]
        assert "`X" in lines[3]  # Not expanded after undef

    def test_predefined_macros(self) -> None:
        """Predefined macros are available without `define."""
        source = "wire [`WIDTH-1:0] data;"
        result = verilog_preprocess(source, predefined={"WIDTH": "32"})
        assert "32-1:0" in result


# ============================================================================
# Test: Parameterized Macros
# ============================================================================


class TestParameterizedMacros:
    """Test `define with parameters."""

    def test_parameterized_macro(self) -> None:
        """`` `define MAX(a, b) ((a) > (b) ? (a) : (b)) ``."""
        source = "`define MAX(a, b) ((a) > (b) ? (a) : (b))\nassign y = `MAX(x, 5);"
        result = verilog_preprocess(source)
        assert "((x) > (5) ? (x) : (5))" in result

    def test_nested_parens_in_args(self) -> None:
        """Macro arguments with nested parentheses."""
        source = "`define F(x) ((x) + 1)\nassign y = `F((a + b));"
        result = verilog_preprocess(source)
        assert "(((a + b)) + 1)" in result


# ============================================================================
# Test: Conditional Compilation
# ============================================================================


class TestConditionals:
    """Test `ifdef / `ifndef / `else / `endif."""

    def test_ifdef_defined(self) -> None:
        """`` `ifdef `` includes lines when macro is defined."""
        source = "`define USE_CACHE\n`ifdef USE_CACHE\nwire cache;\n`endif"
        result = verilog_preprocess(source)
        assert "wire cache;" in result

    def test_ifdef_not_defined(self) -> None:
        """`` `ifdef `` excludes lines when macro is not defined."""
        source = "`ifdef USE_CACHE\nwire cache;\n`endif"
        result = verilog_preprocess(source)
        assert "wire cache;" not in result

    def test_ifdef_else(self) -> None:
        """`` `ifdef / `else `` selects the else branch when not defined."""
        source = "`ifdef USE_CACHE\nwire cache;\n`else\nwire mem;\n`endif"
        result = verilog_preprocess(source)
        assert "wire cache;" not in result
        assert "wire mem;" in result

    def test_ifdef_else_when_defined(self) -> None:
        """`` `ifdef / `else `` selects the if branch when defined."""
        source = "`define USE_CACHE\n`ifdef USE_CACHE\nwire cache;\n`else\nwire mem;\n`endif"
        result = verilog_preprocess(source)
        assert "wire cache;" in result
        assert "wire mem;" not in result

    def test_ifndef(self) -> None:
        """`` `ifndef `` includes lines when macro is NOT defined."""
        source = "`ifndef DEBUG\nwire release;\n`endif"
        result = verilog_preprocess(source)
        assert "wire release;" in result

    def test_ifndef_when_defined(self) -> None:
        """`` `ifndef `` excludes lines when macro IS defined."""
        source = "`define DEBUG\n`ifndef DEBUG\nwire release;\n`endif"
        result = verilog_preprocess(source)
        assert "wire release;" not in result

    def test_nested_ifdef(self) -> None:
        """Nested conditionals."""
        source = (
            "`define A\n"
            "`define B\n"
            "`ifdef A\n"
            "line_a;\n"
            "`ifdef B\n"
            "line_ab;\n"
            "`endif\n"
            "`endif"
        )
        result = verilog_preprocess(source)
        assert "line_a;" in result
        assert "line_ab;" in result

    def test_nested_ifdef_inner_false(self) -> None:
        """Nested conditional where inner is false."""
        source = (
            "`define A\n"
            "`ifdef A\n"
            "line_a;\n"
            "`ifdef B\n"
            "line_ab;\n"
            "`endif\n"
            "`endif"
        )
        result = verilog_preprocess(source)
        assert "line_a;" in result
        assert "line_ab;" not in result

    def test_line_numbers_preserved(self) -> None:
        """Excluded lines become empty (preserve line numbers)."""
        source = "`ifdef UNDEF\nskipped;\nskipped;\n`endif\nkept;"
        result = verilog_preprocess(source)
        lines = result.split("\n")
        assert len(lines) == 5  # Same number of lines as input
        assert lines[4] == "kept;"


# ============================================================================
# Test: Include and Timescale
# ============================================================================


class TestIncludeAndTimescale:
    """Test handling of `include and `timescale directives."""

    def test_include_stubbed(self) -> None:
        """`` `include `` is replaced with a comment."""
        source = '`include "types.vh"'
        result = verilog_preprocess(source)
        assert "types.vh" in result
        assert "not resolved" in result
        assert result.startswith("/*")  # Wrapped in a comment

    def test_timescale_stripped(self) -> None:
        """`` `timescale `` is removed from the source."""
        source = "`timescale 1ns/1ps\nmodule m;"
        result = verilog_preprocess(source)
        assert "`timescale" not in result
        assert "module m;" in result


# ============================================================================
# Test: Integration with Lexer
# ============================================================================


class TestIntegration:
    """Test preprocessor + lexer working together."""

    def test_define_expands_before_tokenization(self) -> None:
        """Macro expansion happens before tokenization."""
        source = "`define WIDTH 8\nwire [`WIDTH-1:0] data;"
        tokens = tokenize_verilog(source, preprocess=True)
        values = [t.value for t in tokens]
        # Should see 8, not `WIDTH
        assert "8" in values
        assert "`WIDTH" not in values

    def test_ifdef_removes_code_before_tokenization(self) -> None:
        """Conditional compilation removes code before tokenization."""
        source = "`ifdef UNDEF\nwire ghost;\n`endif\nwire real_wire;"
        tokens = tokenize_verilog(source, preprocess=True)
        values = [t.value for t in tokens]
        assert "ghost" not in values
        assert "real_wire" in values

    def test_no_preprocess_leaves_directives(self) -> None:
        """With preprocess=False, directives become DIRECTIVE tokens."""
        source = "`define WIDTH 8"
        tokens = tokenize_verilog(source, preprocess=False)
        types = [t.type.name if hasattr(t.type, "name") else t.type for t in tokens]
        assert "DIRECTIVE" in types
