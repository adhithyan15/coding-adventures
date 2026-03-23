"""Tests for the Lattice expression evaluator.

The evaluator converts AST expression nodes into LatticeValue objects.
We test at two levels:

1. **Value types**: Direct construction and string representation.
2. **Token conversion**: Converting parser tokens to values.
3. **Truthiness**: Which values are truthy/falsy.
4. **Arithmetic**: Addition, subtraction, multiplication, negation.
5. **Comparison**: Equality, ordering.
6. **Boolean logic**: and, or.
7. **Variable lookup**: Variables resolved from scope chain.

For the arithmetic and comparison tests, we build mock AST structures
that mirror what the grammar parser produces, then feed them to the
evaluator.
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Any

import pytest

from lattice_ast_to_css.errors import TypeErrorInExpression
from lattice_ast_to_css.evaluator import (
    ExpressionEvaluator,
    LatticeBool,
    LatticeColor,
    LatticeDimension,
    LatticeIdent,
    LatticeList,
    LatticeNull,
    LatticeNumber,
    LatticePercentage,
    LatticeString,
    is_truthy,
    token_to_value,
    value_to_css,
)
from lattice_ast_to_css.scope import ScopeChain


# ---------------------------------------------------------------------------
# Mock AST Types
# ---------------------------------------------------------------------------
#
# We need lightweight AST nodes and tokens for testing without pulling in
# the full parser. These mirror the real ASTNode and Token classes.
# ---------------------------------------------------------------------------


@dataclass
class MockToken:
    """Minimal token for testing."""

    type: str
    value: str
    line: int = 1
    column: int = 1


@dataclass
class MockNode:
    """Minimal AST node for testing."""

    rule_name: str
    children: list[Any]


# ---------------------------------------------------------------------------
# Helper: build expression trees
# ---------------------------------------------------------------------------


def _number_node(value: str) -> MockNode:
    """Build a lattice_primary wrapping a NUMBER token."""
    return MockNode("lattice_primary", [MockToken("NUMBER", value)])


def _dimension_node(value: str) -> MockNode:
    """Build a lattice_primary wrapping a DIMENSION token."""
    return MockNode("lattice_primary", [MockToken("DIMENSION", value)])


def _percentage_node(value: str) -> MockNode:
    """Build a lattice_primary wrapping a PERCENTAGE token."""
    return MockNode("lattice_primary", [MockToken("PERCENTAGE", value)])


def _ident_node(value: str) -> MockNode:
    """Build a lattice_primary wrapping an IDENT token."""
    return MockNode("lattice_primary", [MockToken("IDENT", value)])


def _variable_node(name: str) -> MockNode:
    """Build a lattice_primary wrapping a VARIABLE token."""
    return MockNode("lattice_primary", [MockToken("VARIABLE", name)])


def _hash_node(value: str) -> MockNode:
    """Build a lattice_primary wrapping a HASH token."""
    return MockNode("lattice_primary", [MockToken("HASH", value)])


def _string_node(value: str) -> MockNode:
    """Build a lattice_primary wrapping a STRING token."""
    return MockNode("lattice_primary", [MockToken("STRING", value)])


def _additive_node(
    left: MockNode, op: str, right: MockNode
) -> MockNode:
    """Build a lattice_additive node: left OP right."""
    return MockNode("lattice_additive", [left, MockToken("PLUS" if op == "+" else "MINUS", op), right])


def _multiplicative_node(left: MockNode, right: MockNode) -> MockNode:
    """Build a lattice_multiplicative node: left * right."""
    return MockNode("lattice_multiplicative", [left, MockToken("STAR", "*"), right])


def _unary_minus_node(operand: MockNode) -> MockNode:
    """Build a lattice_unary node: -operand."""
    return MockNode("lattice_unary", [MockToken("MINUS", "-"), operand])


def _comparison_node(
    left: MockNode, op_type: str, op_value: str, right: MockNode
) -> MockNode:
    """Build a lattice_comparison node: left OP right."""
    op_node = MockNode("comparison_op", [MockToken(op_type, op_value)])
    return MockNode("lattice_comparison", [left, op_node, right])


def _and_node(left: MockNode, right: MockNode) -> MockNode:
    """Build a lattice_and_expr node: left and right."""
    return MockNode("lattice_and_expr", [left, MockToken("IDENT", "and"), right])


def _or_node(left: MockNode, right: MockNode) -> MockNode:
    """Build a lattice_or_expr node: left or right."""
    return MockNode("lattice_or_expr", [left, MockToken("IDENT", "or"), right])


# ===========================================================================
# Value Type Tests
# ===========================================================================


class TestValueTypes:
    """Test LatticeValue construction and string representation."""

    def test_number_integer(self) -> None:
        assert str(LatticeNumber(42)) == "42"

    def test_number_float(self) -> None:
        assert str(LatticeNumber(3.14)) == "3.14"

    def test_dimension(self) -> None:
        assert str(LatticeDimension(16, "px")) == "16px"

    def test_dimension_float(self) -> None:
        assert str(LatticeDimension(1.5, "em")) == "1.5em"

    def test_percentage(self) -> None:
        assert str(LatticePercentage(50)) == "50%"

    def test_percentage_float(self) -> None:
        assert str(LatticePercentage(33.33)) == "33.33%"

    def test_string(self) -> None:
        assert str(LatticeString("hello")) == '"hello"'

    def test_ident(self) -> None:
        assert str(LatticeIdent("red")) == "red"

    def test_color(self) -> None:
        assert str(LatticeColor("#4a90d9")) == "#4a90d9"

    def test_bool_true(self) -> None:
        assert str(LatticeBool(True)) == "true"

    def test_bool_false(self) -> None:
        assert str(LatticeBool(False)) == "false"

    def test_null(self) -> None:
        assert str(LatticeNull()) == ""

    def test_list(self) -> None:
        items = (LatticeIdent("red"), LatticeIdent("green"), LatticeIdent("blue"))
        assert str(LatticeList(items)) == "red, green, blue"


# ===========================================================================
# Token Conversion Tests
# ===========================================================================


class TestTokenConversion:
    """Test token_to_value conversion."""

    def test_number_token(self) -> None:
        result = token_to_value(MockToken("NUMBER", "42"))
        assert result == LatticeNumber(42)

    def test_dimension_token(self) -> None:
        result = token_to_value(MockToken("DIMENSION", "16px"))
        assert result == LatticeDimension(16, "px")

    def test_dimension_with_float(self) -> None:
        result = token_to_value(MockToken("DIMENSION", "1.5em"))
        assert result == LatticeDimension(1.5, "em")

    def test_percentage_token(self) -> None:
        result = token_to_value(MockToken("PERCENTAGE", "50%"))
        assert result == LatticePercentage(50)

    def test_string_token(self) -> None:
        result = token_to_value(MockToken("STRING", "hello"))
        assert result == LatticeString("hello")

    def test_hash_token(self) -> None:
        result = token_to_value(MockToken("HASH", "#fff"))
        assert result == LatticeColor("#fff")

    def test_ident_token(self) -> None:
        result = token_to_value(MockToken("IDENT", "red"))
        assert result == LatticeIdent("red")

    def test_true_ident(self) -> None:
        result = token_to_value(MockToken("IDENT", "true"))
        assert result == LatticeBool(True)

    def test_false_ident(self) -> None:
        result = token_to_value(MockToken("IDENT", "false"))
        assert result == LatticeBool(False)

    def test_null_ident(self) -> None:
        result = token_to_value(MockToken("IDENT", "null"))
        assert isinstance(result, LatticeNull)

    def test_unknown_token_type(self) -> None:
        result = token_to_value(MockToken("UNKNOWN", "???"))
        assert result == LatticeIdent("???")


class TestValueToCss:
    """Test value_to_css conversion."""

    def test_number(self) -> None:
        assert value_to_css(LatticeNumber(42)) == "42"

    def test_dimension(self) -> None:
        assert value_to_css(LatticeDimension(16, "px")) == "16px"

    def test_color(self) -> None:
        assert value_to_css(LatticeColor("#fff")) == "#fff"


# ===========================================================================
# Truthiness Tests
# ===========================================================================


class TestTruthiness:
    """Test is_truthy() for all value types."""

    def test_true_is_truthy(self) -> None:
        assert is_truthy(LatticeBool(True)) is True

    def test_false_is_falsy(self) -> None:
        assert is_truthy(LatticeBool(False)) is False

    def test_null_is_falsy(self) -> None:
        assert is_truthy(LatticeNull()) is False

    def test_zero_is_falsy(self) -> None:
        assert is_truthy(LatticeNumber(0)) is False

    def test_nonzero_number_is_truthy(self) -> None:
        assert is_truthy(LatticeNumber(1)) is True
        assert is_truthy(LatticeNumber(-1)) is True

    def test_dimension_is_truthy(self) -> None:
        assert is_truthy(LatticeDimension(16, "px")) is True

    def test_string_is_truthy(self) -> None:
        assert is_truthy(LatticeString("")) is True  # Even empty strings!

    def test_ident_is_truthy(self) -> None:
        assert is_truthy(LatticeIdent("red")) is True

    def test_color_is_truthy(self) -> None:
        assert is_truthy(LatticeColor("#000")) is True

    def test_list_is_truthy(self) -> None:
        assert is_truthy(LatticeList(())) is True  # Even empty lists!


# ===========================================================================
# Arithmetic Tests
# ===========================================================================


class TestAddition:
    """Test addition via the evaluator."""

    def test_number_plus_number(self) -> None:
        scope = ScopeChain()
        ev = ExpressionEvaluator(scope)
        node = _additive_node(_number_node("2"), "+", _number_node("3"))
        assert ev.evaluate(node) == LatticeNumber(5)

    def test_dimension_plus_dimension(self) -> None:
        scope = ScopeChain()
        ev = ExpressionEvaluator(scope)
        node = _additive_node(_dimension_node("10px"), "+", _dimension_node("5px"))
        assert ev.evaluate(node) == LatticeDimension(15, "px")

    def test_percentage_plus_percentage(self) -> None:
        scope = ScopeChain()
        ev = ExpressionEvaluator(scope)
        node = _additive_node(_percentage_node("50%"), "+", _percentage_node("25%"))
        assert ev.evaluate(node) == LatticePercentage(75)

    def test_string_concat(self) -> None:
        scope = ScopeChain()
        ev = ExpressionEvaluator(scope)
        node = _additive_node(_string_node("hello"), "+", _string_node(" world"))
        assert ev.evaluate(node) == LatticeString("hello world")

    def test_mismatched_units_raises(self) -> None:
        scope = ScopeChain()
        ev = ExpressionEvaluator(scope)
        node = _additive_node(_dimension_node("10px"), "+", _dimension_node("2em"))
        with pytest.raises(TypeErrorInExpression):
            ev.evaluate(node)

    def test_incompatible_types_raises(self) -> None:
        scope = ScopeChain()
        ev = ExpressionEvaluator(scope)
        node = _additive_node(_number_node("10"), "+", _ident_node("red"))
        with pytest.raises(TypeErrorInExpression):
            ev.evaluate(node)


class TestSubtraction:
    """Test subtraction via the evaluator."""

    def test_number_minus_number(self) -> None:
        scope = ScopeChain()
        ev = ExpressionEvaluator(scope)
        node = _additive_node(_number_node("10"), "-", _number_node("3"))
        assert ev.evaluate(node) == LatticeNumber(7)

    def test_dimension_minus_dimension(self) -> None:
        scope = ScopeChain()
        ev = ExpressionEvaluator(scope)
        node = _additive_node(_dimension_node("20px"), "-", _dimension_node("5px"))
        assert ev.evaluate(node) == LatticeDimension(15, "px")

    def test_percentage_minus_percentage(self) -> None:
        scope = ScopeChain()
        ev = ExpressionEvaluator(scope)
        node = _additive_node(_percentage_node("100%"), "-", _percentage_node("25%"))
        assert ev.evaluate(node) == LatticePercentage(75)

    def test_mismatched_units_raises(self) -> None:
        scope = ScopeChain()
        ev = ExpressionEvaluator(scope)
        node = _additive_node(_dimension_node("10px"), "-", _dimension_node("2em"))
        with pytest.raises(TypeErrorInExpression):
            ev.evaluate(node)


class TestMultiplication:
    """Test multiplication via the evaluator."""

    def test_number_times_number(self) -> None:
        scope = ScopeChain()
        ev = ExpressionEvaluator(scope)
        node = _multiplicative_node(_number_node("3"), _number_node("4"))
        assert ev.evaluate(node) == LatticeNumber(12)

    def test_number_times_dimension(self) -> None:
        scope = ScopeChain()
        ev = ExpressionEvaluator(scope)
        node = _multiplicative_node(_number_node("2"), _dimension_node("8px"))
        assert ev.evaluate(node) == LatticeDimension(16, "px")

    def test_dimension_times_number(self) -> None:
        scope = ScopeChain()
        ev = ExpressionEvaluator(scope)
        node = _multiplicative_node(_dimension_node("8px"), _number_node("2"))
        assert ev.evaluate(node) == LatticeDimension(16, "px")

    def test_number_times_percentage(self) -> None:
        scope = ScopeChain()
        ev = ExpressionEvaluator(scope)
        node = _multiplicative_node(_number_node("2"), _percentage_node("50%"))
        assert ev.evaluate(node) == LatticePercentage(100)

    def test_incompatible_types_raises(self) -> None:
        scope = ScopeChain()
        ev = ExpressionEvaluator(scope)
        node = _multiplicative_node(_dimension_node("10px"), _dimension_node("5px"))
        with pytest.raises(TypeErrorInExpression):
            ev.evaluate(node)


class TestUnaryMinus:
    """Test unary negation."""

    def test_negate_number(self) -> None:
        scope = ScopeChain()
        ev = ExpressionEvaluator(scope)
        node = _unary_minus_node(_number_node("5"))
        assert ev.evaluate(node) == LatticeNumber(-5)

    def test_negate_dimension(self) -> None:
        scope = ScopeChain()
        ev = ExpressionEvaluator(scope)
        node = _unary_minus_node(_dimension_node("10px"))
        assert ev.evaluate(node) == LatticeDimension(-10, "px")

    def test_negate_percentage(self) -> None:
        scope = ScopeChain()
        ev = ExpressionEvaluator(scope)
        node = _unary_minus_node(_percentage_node("50%"))
        assert ev.evaluate(node) == LatticePercentage(-50)

    def test_negate_non_numeric_raises(self) -> None:
        scope = ScopeChain()
        ev = ExpressionEvaluator(scope)
        node = _unary_minus_node(_ident_node("red"))
        with pytest.raises(TypeErrorInExpression):
            ev.evaluate(node)


# ===========================================================================
# Comparison Tests
# ===========================================================================


class TestComparison:
    """Test comparison operations."""

    def test_equals_true(self) -> None:
        scope = ScopeChain()
        ev = ExpressionEvaluator(scope)
        node = _comparison_node(
            _number_node("5"), "EQUALS_EQUALS", "==", _number_node("5")
        )
        assert ev.evaluate(node) == LatticeBool(True)

    def test_equals_false(self) -> None:
        scope = ScopeChain()
        ev = ExpressionEvaluator(scope)
        node = _comparison_node(
            _number_node("5"), "EQUALS_EQUALS", "==", _number_node("3")
        )
        assert ev.evaluate(node) == LatticeBool(False)

    def test_not_equals(self) -> None:
        scope = ScopeChain()
        ev = ExpressionEvaluator(scope)
        node = _comparison_node(
            _number_node("5"), "NOT_EQUALS", "!=", _number_node("3")
        )
        assert ev.evaluate(node) == LatticeBool(True)

    def test_greater(self) -> None:
        scope = ScopeChain()
        ev = ExpressionEvaluator(scope)
        node = _comparison_node(
            _number_node("10"), "GREATER", ">", _number_node("5")
        )
        assert ev.evaluate(node) == LatticeBool(True)

    def test_greater_equals(self) -> None:
        scope = ScopeChain()
        ev = ExpressionEvaluator(scope)
        node = _comparison_node(
            _number_node("5"), "GREATER_EQUALS", ">=", _number_node("5")
        )
        assert ev.evaluate(node) == LatticeBool(True)

    def test_less_equals(self) -> None:
        scope = ScopeChain()
        ev = ExpressionEvaluator(scope)
        node = _comparison_node(
            _number_node("3"), "LESS_EQUALS", "<=", _number_node("5")
        )
        assert ev.evaluate(node) == LatticeBool(True)

    def test_dimension_equals_same_unit(self) -> None:
        scope = ScopeChain()
        ev = ExpressionEvaluator(scope)
        node = _comparison_node(
            _dimension_node("16px"), "EQUALS_EQUALS", "==", _dimension_node("16px")
        )
        assert ev.evaluate(node) == LatticeBool(True)

    def test_dimension_equals_different_unit(self) -> None:
        scope = ScopeChain()
        ev = ExpressionEvaluator(scope)
        node = _comparison_node(
            _dimension_node("16px"), "EQUALS_EQUALS", "==", _dimension_node("16em")
        )
        assert ev.evaluate(node) == LatticeBool(False)

    def test_ident_equality(self) -> None:
        scope = ScopeChain()
        ev = ExpressionEvaluator(scope)
        node = _comparison_node(
            _ident_node("dark"), "EQUALS_EQUALS", "==", _ident_node("dark")
        )
        assert ev.evaluate(node) == LatticeBool(True)

    def test_ident_inequality(self) -> None:
        scope = ScopeChain()
        ev = ExpressionEvaluator(scope)
        node = _comparison_node(
            _ident_node("dark"), "NOT_EQUALS", "!=", _ident_node("light")
        )
        assert ev.evaluate(node) == LatticeBool(True)


# ===========================================================================
# Boolean Logic Tests
# ===========================================================================


class TestBooleanLogic:
    """Test and/or operations with short-circuit evaluation."""

    def test_and_both_true(self) -> None:
        scope = ScopeChain()
        ev = ExpressionEvaluator(scope)
        node = _and_node(_ident_node("true"), _ident_node("true"))
        assert is_truthy(ev.evaluate(node))

    def test_and_first_false(self) -> None:
        scope = ScopeChain()
        ev = ExpressionEvaluator(scope)
        node = _and_node(_ident_node("false"), _ident_node("true"))
        assert not is_truthy(ev.evaluate(node))

    def test_and_second_false(self) -> None:
        scope = ScopeChain()
        ev = ExpressionEvaluator(scope)
        node = _and_node(_ident_node("true"), _ident_node("false"))
        assert not is_truthy(ev.evaluate(node))

    def test_or_both_false(self) -> None:
        scope = ScopeChain()
        ev = ExpressionEvaluator(scope)
        node = _or_node(_ident_node("false"), _ident_node("false"))
        assert not is_truthy(ev.evaluate(node))

    def test_or_first_true(self) -> None:
        scope = ScopeChain()
        ev = ExpressionEvaluator(scope)
        node = _or_node(_ident_node("true"), _ident_node("false"))
        assert is_truthy(ev.evaluate(node))

    def test_or_second_true(self) -> None:
        scope = ScopeChain()
        ev = ExpressionEvaluator(scope)
        node = _or_node(_ident_node("false"), _ident_node("true"))
        assert is_truthy(ev.evaluate(node))


# ===========================================================================
# Variable Lookup Tests
# ===========================================================================


class TestVariableLookup:
    """Test variable resolution from scope chain."""

    def test_variable_found(self) -> None:
        scope = ScopeChain()
        scope.set("$x", LatticeNumber(42))
        ev = ExpressionEvaluator(scope)
        node = _variable_node("$x")
        assert ev.evaluate(node) == LatticeNumber(42)

    def test_variable_from_parent_scope(self) -> None:
        parent = ScopeChain()
        parent.set("$color", LatticeIdent("red"))
        child = parent.child()
        ev = ExpressionEvaluator(child)
        node = _variable_node("$color")
        assert ev.evaluate(node) == LatticeIdent("red")

    def test_variable_not_found_returns_ident(self) -> None:
        """Undefined variables return an ident (transformer handles errors)."""
        scope = ScopeChain()
        ev = ExpressionEvaluator(scope)
        node = _variable_node("$missing")
        result = ev.evaluate(node)
        assert isinstance(result, LatticeIdent)

    def test_variable_in_expression(self) -> None:
        """$n * 8px evaluates with $n from scope."""
        scope = ScopeChain()
        scope.set("$n", LatticeNumber(2))
        ev = ExpressionEvaluator(scope)
        node = _multiplicative_node(_variable_node("$n"), _dimension_node("8px"))
        assert ev.evaluate(node) == LatticeDimension(16, "px")


# ===========================================================================
# Wrapper Rule Tests
# ===========================================================================


class TestWrapperRules:
    """Test that wrapper rules (single-child nodes) are properly unwrapped."""

    def test_expression_wraps_or_expr(self) -> None:
        scope = ScopeChain()
        ev = ExpressionEvaluator(scope)
        inner = _number_node("42")
        wrapper = MockNode("lattice_expression", [inner])
        assert ev.evaluate(wrapper) == LatticeNumber(42)

    def test_deeply_nested_wrappers(self) -> None:
        scope = ScopeChain()
        ev = ExpressionEvaluator(scope)
        leaf = _number_node("7")
        n1 = MockNode("lattice_unary", [leaf])
        n2 = MockNode("lattice_multiplicative", [n1])
        n3 = MockNode("lattice_additive", [n2])
        n4 = MockNode("lattice_comparison", [n3])
        n5 = MockNode("lattice_and_expr", [n4])
        n6 = MockNode("lattice_or_expr", [n5])
        n7 = MockNode("lattice_expression", [n6])
        assert ev.evaluate(n7) == LatticeNumber(7)

    def test_parenthesized_expression(self) -> None:
        """(5 + 3) evaluates through LPAREN expr RPAREN."""
        scope = ScopeChain()
        ev = ExpressionEvaluator(scope)
        inner_add = _additive_node(_number_node("5"), "+", _number_node("3"))
        paren_node = MockNode("lattice_primary", [
            MockToken("LPAREN", "("),
            inner_add,
            MockToken("RPAREN", ")"),
        ])
        assert ev.evaluate(paren_node) == LatticeNumber(8)
