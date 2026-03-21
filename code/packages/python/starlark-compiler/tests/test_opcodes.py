"""Tests for Starlark opcodes and compiler rule handlers."""

from __future__ import annotations

import pytest

from starlark_compiler.opcodes import (
    AUGMENTED_ASSIGN_MAP,
    BINARY_OP_MAP,
    COMPARE_OP_MAP,
    Op,
)
from starlark_compiler.compiler import (
    _parse_string_literal,
    _type_name,
    create_starlark_compiler,
)


# =========================================================================
# Test: Opcode Enum
# =========================================================================


class TestOpcodeEnum:
    """Verify opcode organization and values."""

    def test_stack_opcodes_range(self):
        """Stack opcodes are in 0x01-0x06."""
        assert Op.LOAD_CONST == 0x01
        assert Op.POP == 0x02
        assert Op.DUP == 0x03
        assert Op.LOAD_NONE == 0x04
        assert Op.LOAD_TRUE == 0x05
        assert Op.LOAD_FALSE == 0x06

    def test_variable_opcodes_range(self):
        """Variable opcodes are in 0x10-0x15."""
        assert Op.STORE_NAME == 0x10
        assert Op.LOAD_NAME == 0x11
        assert Op.STORE_LOCAL == 0x12
        assert Op.LOAD_LOCAL == 0x13
        assert Op.STORE_CLOSURE == 0x14
        assert Op.LOAD_CLOSURE == 0x15

    def test_arithmetic_opcodes_range(self):
        """Arithmetic opcodes are in 0x20-0x2D."""
        assert Op.ADD == 0x20
        assert Op.RSHIFT == 0x2D

    def test_comparison_opcodes_range(self):
        """Comparison opcodes are in 0x30-0x38."""
        assert Op.CMP_EQ == 0x30
        assert Op.NOT == 0x38

    def test_control_flow_range(self):
        """Control flow opcodes are in 0x40-0x44."""
        assert Op.JUMP == 0x40
        assert Op.JUMP_IF_TRUE_OR_POP == 0x44

    def test_function_opcodes_range(self):
        """Function opcodes are in 0x50-0x53."""
        assert Op.MAKE_FUNCTION == 0x50
        assert Op.RETURN == 0x53

    def test_collection_opcodes_range(self):
        """Collection opcodes are in 0x60-0x64."""
        assert Op.BUILD_LIST == 0x60
        assert Op.DICT_SET == 0x64

    def test_subscript_opcodes_range(self):
        """Subscript opcodes are in 0x70-0x74."""
        assert Op.LOAD_SUBSCRIPT == 0x70
        assert Op.LOAD_SLICE == 0x74

    def test_iteration_opcodes_range(self):
        """Iteration opcodes are in 0x80-0x82."""
        assert Op.GET_ITER == 0x80
        assert Op.UNPACK_SEQUENCE == 0x82

    def test_halt_opcode(self):
        assert Op.HALT == 0xFF


# =========================================================================
# Test: Operator Maps
# =========================================================================


class TestOperatorMaps:
    """Verify operator-to-opcode mappings."""

    def test_binary_ops_complete(self):
        """All binary operators are mapped."""
        expected = {"+", "-", "*", "/", "//", "%", "**", "&", "|", "^", "<<", ">>"}
        assert set(BINARY_OP_MAP.keys()) == expected

    def test_compare_ops_complete(self):
        """All comparison operators are mapped."""
        expected = {"==", "!=", "<", ">", "<=", ">=", "in", "not in"}
        assert set(COMPARE_OP_MAP.keys()) == expected

    def test_augmented_assign_ops_complete(self):
        """All augmented assignment operators are mapped."""
        expected = {
            "+=", "-=", "*=", "/=", "//=", "%=",
            "&=", "|=", "^=", "<<=", ">>=", "**=",
        }
        assert set(AUGMENTED_ASSIGN_MAP.keys()) == expected


# =========================================================================
# Test: String Literal Parsing
# =========================================================================


class TestStringLiteralParsing:
    """Test the string literal parser."""

    def test_double_quoted(self):
        assert _parse_string_literal('"hello"') == "hello"

    def test_single_quoted(self):
        assert _parse_string_literal("'hello'") == "hello"

    def test_escape_newline(self):
        assert _parse_string_literal('"hello\\nworld"') == "hello\nworld"

    def test_escape_tab(self):
        assert _parse_string_literal('"hello\\tworld"') == "hello\tworld"

    def test_escape_backslash(self):
        assert _parse_string_literal('"hello\\\\world"') == "hello\\world"

    def test_escape_quote(self):
        assert _parse_string_literal('"hello\\"world"') == 'hello"world'

    def test_empty_string(self):
        assert _parse_string_literal('""') == ""


# =========================================================================
# Test: Compiler Factory
# =========================================================================


class TestCompilerFactory:
    """Test create_starlark_compiler factory."""

    def test_creates_compiler(self):
        """Factory returns a GenericCompiler with handlers registered."""
        compiler = create_starlark_compiler()
        # Verify key rules are registered
        assert "file" in compiler._dispatch
        assert "assign_stmt" in compiler._dispatch
        assert "if_stmt" in compiler._dispatch
        assert "for_stmt" in compiler._dispatch
        assert "def_stmt" in compiler._dispatch
        assert "arith" in compiler._dispatch
        assert "atom" in compiler._dispatch

    def test_all_expected_rules_registered(self):
        """All rules that need handlers are registered."""
        compiler = create_starlark_compiler()
        expected_rules = [
            "file", "simple_stmt", "assign_stmt", "return_stmt",
            "break_stmt", "continue_stmt", "pass_stmt", "load_stmt",
            "if_stmt", "for_stmt", "def_stmt", "suite",
            "expression", "expression_list", "or_expr", "and_expr",
            "not_expr", "comparison", "arith", "term", "shift",
            "bitwise_or", "bitwise_xor", "bitwise_and",
            "factor", "power", "primary", "atom",
            "list_expr", "dict_expr", "paren_expr", "lambda_expr",
        ]
        for rule in expected_rules:
            assert rule in compiler._dispatch, f"Missing handler for '{rule}'"
